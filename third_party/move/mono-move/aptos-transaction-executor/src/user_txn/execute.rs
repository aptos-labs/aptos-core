// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The user-transaction execution flow: pre-execution checks, then one
//! session hosting the prologue, the payload, and the epilogue.

use super::{
    account::{create_account, needs_account_creation},
    entry_func::call_entry_function,
    metadata::TxnMetadata,
    pre_execution_checks::PreExecutionChecker,
    script::run_script,
    validation::{run_epilogue, run_prologue, ValidationSigners},
};
use crate::{
    errors::{call_result, DiscardReason, ExecutionStage, ExecutionStatus, MoveExecutionFailure},
    executor::AptosTransactionExecutor,
    natives::extensions_with,
    outcome::TxnOutcome,
};
use aptos_types::{
    fee_statement::FeeStatement,
    state_store::state_storage_usage::StateStorageUsage,
    transaction::{
        AuxiliaryInfo, EntryFunction, Script, SignedTransaction, TransactionExecutableRef,
    },
};
use mono_move_core::{
    intern_type_tag, native::NativeExtensions, types::InternedTypeList, GasMeter, Interner,
};
use mono_move_loader::{Loader, LoadingPolicy, LoweringPolicy};
use mono_move_natives::TransactionContextExtension;
use mono_move_runtime::InterpreterContext;
use move_core_types::language_storage::TypeTag;

impl<'guard> AptosTransactionExecutor<'guard> {
    /// Executes one user transaction, returning its side effects unmaterialized (see [`TxnOutcome`]).
    /// `aux_info` carries the transaction's index in its block, which seeds
    /// `monotonically_increasing_number`.
    //
    // TODO(completeness): add logging. Should warn on unexpected errors/discards.
    pub fn execute_user_transaction(
        &self,
        txn: &SignedTransaction,
        aux_info: &AuxiliaryInfo,
    ) -> TxnOutcome {
        match self.execute_user_transaction_impl(txn, aux_info) {
            Ok(outcome) => outcome,
            Err(reason) => TxnOutcome::Discarded(reason),
        }
    }

    /// Actual implementation of the transaction execution flow.
    fn execute_user_transaction_impl(
        &self,
        txn: &SignedTransaction,
        aux_info: &AuxiliaryInfo,
    ) -> Result<TxnOutcome, DiscardReason> {
        let guard = self.guard;

        // ======================== Pre-execution checks ========================
        // Reject what this executor cannot execute, before touching any state.
        let txn_data = TxnMetadata::new(txn, aux_info);
        let gas_params = self.env.gas_params().as_ref().map_err(|e| {
            DiscardReason::InvariantViolation(format!("the gas schedule is unavailable: {e}"))
        })?;
        let checker = PreExecutionChecker::new(
            gas_params,
            self.env.gas_feature_version(),
            self.env.features(),
            &txn_data,
        );
        checker
            .run_checks()
            .map_err(DiscardReason::PreExecutionCheck)?;

        // TODO(completeness): multisig payloads. Refused for now, since the
        // inner executable must run as the multisig account, not the sender.
        if txn.multisig_address().is_some() {
            return Err(DiscardReason::Unsupported("multisig payloads"));
        }
        let executable = match txn.payload().executable_ref() {
            Ok(TransactionExecutableRef::EntryFunction(entry)) => Executable::EntryFunction(entry),
            Ok(TransactionExecutableRef::Script(script)) => Executable::Script(script),
            Ok(TransactionExecutableRef::Encrypted) => Executable::Encrypted,
            // Only a multisig transaction may leave the executable out.
            Ok(TransactionExecutableRef::Empty) => return Err(DiscardReason::EmptyPayload),
            Err(_) => return Err(DiscardReason::Deprecated("module-bundle payload")),
        };
        // TODO(security): these type arguments are user supplied, so interning
        // them can pollute the global caches. Needs a bound.
        let interned_ty_args = executable
            .ty_args()
            .iter()
            .map(|tag| intern_type_tag(tag, guard))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| DiscardReason::InvalidTypeArgument(format!("{e:#}")))?;
        let ty_args = guard.type_list_of(&interned_ty_args);

        // ========================== Session setup ===========================
        // One session hosts the whole transaction: prologue, payload, epilogue.
        // TODO(completeness): make the loading policy configurable.
        let loader = Loader::new_with_policy(
            guard,
            self.module_provider,
            LoadingPolicy::Lazy(LoweringPolicy::Lazy),
            self.natives,
        );

        let extensions = transaction_extensions(&txn_data, self.usage);

        let max_gas = txn_data.max_gas_amount;
        // TODO(metering): charge the transaction's base cost: intrinsic gas
        // plus the keyless, SLH-DSA, and decryption surcharges.
        let mut interp = InterpreterContext::new(
            loader,
            // TODO(metering): MonoMove gas units are uncalibrated; budgeting
            // 1:1 against the transaction's gas units is a placeholder.
            GasMeter::new(max_gas),
            self.data_provider,
            self.natives,
        )
        .with_extensions(extensions);

        let signers = ValidationSigners::new(&txn_data);

        // ======================= Lazy account creation ========================
        // The sender's first transaction may come from an address holding no
        // `Account` resource yet. The account is created between the prologue
        // and the payload, so that it survives the payload's rollback.
        let create_sender_account =
            needs_account_creation(&mut interp, guard, &txn_data).map_err(|e| {
                DiscardReason::Failure {
                    stage: ExecutionStage::AccountCreation,
                    failure: MoveExecutionFailure::RuntimeError(e),
                }
            })?;
        if create_sender_account {
            checker
                .check_gas_budget_covers_account_creation()
                .map_err(DiscardReason::PreExecutionCheck)?;
        }

        // ============================ Prologue ==============================
        // Validate the transaction (auth key, sequence number or nonce, fee coverage etc.)
        run_prologue(&mut interp, guard, &signers, &txn_data).map_err(|failure| {
            DiscardReason::Failure {
                stage: ExecutionStage::Prologue,
                failure,
            }
        })?;
        if create_sender_account {
            self.run_metered(&mut interp, |interp| {
                create_account(interp, guard, &txn_data.sender)
            })
            .map_err(|failure| DiscardReason::Failure {
                stage: ExecutionStage::AccountCreation,
                failure,
            })?;
        }
        // A failed payload rolls back to here, so the prologue's effects (e.g.
        // nonce insertion) and the created account survive.
        checkpoint(&mut interp)?;

        // ========================== User payload ============================
        // An unmetered payload leaves the balance untouched, making the
        // epilogue charge nothing.
        let payload_result = self.run_metered(&mut interp, |interp| {
            self.execute_payload(interp, &txn_data, &executable, ty_args)
        });
        let gas_remaining = interp.gas_balance();
        let gas_used = max_gas.saturating_sub(gas_remaining);

        // TODO(metering): charge gas for global storage writes and events.

        // A failed payload keeps only the prologue's effects; the epilogue below
        // still charges the fee.
        let payload_succeeded = payload_result.is_ok();
        let execution_status = match payload_result {
            Ok(()) => ExecutionStatus::Success,
            Err(failure) => {
                rollback(&mut interp, 1)?;
                ExecutionStatus::Failure {
                    stage: ExecutionStage::Payload,
                    failure,
                }
            },
        };

        // ============================ Epilogue ==============================
        // Transaction cleanup -- charge gas, bump sequence number etc.
        let fee_statement = placeholder_fee_statement(gas_used);
        let epilogue = |interp: &mut InterpreterContext<'guard>| {
            run_epilogue(
                interp,
                guard,
                &signers,
                &txn_data,
                fee_statement,
                gas_remaining,
            )
        };
        let execution_status = match epilogue(&mut interp) {
            Ok(()) => execution_status,
            // Payload failed + epilogue failed => no choice but to discard.
            // This should not happen unless there is a bug in the executor.
            Err(failure) if !payload_succeeded => {
                return Err(DiscardReason::Failure {
                    stage: ExecutionStage::EpilogueAfterRollback,
                    failure,
                })
            },
            // Payload succeeded + epilogue failed => rollback payload effects and
            // retry. The transaction still commits, charging the fee; which
            // epilogue failures are legitimate is decided when the status is
            // converted.
            //
            // TODO(correctness): audit this against the legacy VM, which may
            // discard here instead.
            Err(failure) => {
                rollback(&mut interp, 1)?;
                epilogue(&mut interp).map_err(|failure| DiscardReason::Failure {
                    stage: ExecutionStage::EpilogueRetry,
                    failure,
                })?;
                ExecutionStatus::Failure {
                    stage: ExecutionStage::Epilogue,
                    failure,
                }
            },
        };

        // ============================= Output ===============================
        // Hand the side effects back unmaterialized; the coordinator decides
        // when (and whether) to render them into storage formats.
        //
        // TODO(completeness): resolve a pending `code::request_publish` into
        // module writes, as AptosVM does when finishing the session.
        Ok(TxnOutcome::Executed {
            status: execution_status,
            fee_statement,
            effects: interp.finish(),
        })
    }

    fn execute_payload(
        &self,
        interp: &mut InterpreterContext<'guard>,
        txn_data: &TxnMetadata,
        executable: &Executable<'_>,
        ty_args: InternedTypeList,
    ) -> Result<(), MoveExecutionFailure> {
        // TODO(completeness): multi-agent transactions are untested.
        let status = match executable {
            Executable::EntryFunction(entry) => call_entry_function(
                self.guard,
                interp,
                &entry.module().address,
                entry.module().name(),
                entry.function(),
                ty_args,
                &txn_data.sender,
                &txn_data.secondary_signers,
                entry.args(),
            )?,
            Executable::Script(script) => run_script(
                self.guard,
                interp,
                self.env.chain_id(),
                script.code(),
                ty_args,
                &txn_data.sender,
                &txn_data.secondary_signers,
                script.args(),
            )?,
            // Decryption failed upstream: nothing runs, the fee is still charged.
            Executable::Encrypted => return Err(MoveExecutionFailure::UndecryptedPayload),
        };

        call_result(status)
    }

    /// Runs `f` metered against the transaction's budget, or unmetered if the
    /// executor was built `without_metering`.
    fn run_metered<R>(
        &self,
        interp: &mut InterpreterContext<'guard>,
        f: impl FnOnce(&mut InterpreterContext<'guard>) -> R,
    ) -> R {
        if self.unmetered {
            interp.unmetered(f)
        } else {
            f(interp)
        }
    }
}

/// The payloads this executor runs.
enum Executable<'a> {
    EntryFunction(&'a EntryFunction),
    Script(&'a Script),
    /// An encrypted payload that was not decrypted, so there is nothing to run.
    Encrypted,
}

impl Executable<'_> {
    fn ty_args(&self) -> &[TypeTag] {
        match self {
            Executable::EntryFunction(entry) => entry.ty_args(),
            Executable::Script(script) => script.ty_args(),
            Executable::Encrypted => &[],
        }
    }
}

/// The native extensions a user transaction runs with.
fn transaction_extensions(txn_data: &TxnMetadata, usage: StateStorageUsage) -> NativeExtensions {
    extensions_with(
        TransactionContextExtension::new(
            txn_data.txn_hash,
            txn_data.script_hash.clone(),
            txn_data.chain_id,
            txn_data.session_counter,
            Some(txn_data.as_user_transaction_context()),
        ),
        usage,
    )
}

/// The fee statement for a transaction that consumed `gas_used` units.
//
// TODO(metering): IO gas, storage fees, and refunds are all zero until they
// are charged; `gas_used` is in uncalibrated MonoMove units.
fn placeholder_fee_statement(gas_used: u64) -> FeeStatement {
    FeeStatement::builder()
        .total_charge_gas_units(gas_used)
        .execution_gas_units(gas_used)
        .io_gas_units(0)
        .storage_fee_octas(0)
        .storage_fee_refund_octas(0)
        .build()
}

fn checkpoint(interp: &mut InterpreterContext<'_>) -> Result<(), DiscardReason> {
    interp
        .checkpoint()
        .map_err(|e| DiscardReason::InvariantViolation(format!("checkpoint failed: {e}")))
}

fn rollback(interp: &mut InterpreterContext<'_>, n: usize) -> Result<(), DiscardReason> {
    interp
        .rollback(n)
        .map_err(|e| DiscardReason::InvariantViolation(format!("rollback failed: {e}")))
}
