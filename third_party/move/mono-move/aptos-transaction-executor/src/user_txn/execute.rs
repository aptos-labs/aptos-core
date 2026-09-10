// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The user-transaction execution flow: pre-execution checks, then one
//! session hosting the prologue, the payload, and the epilogue.

use super::{
    args::call_entry_function,
    metadata::TxnMetadata,
    pre_execution_checks::PreExecutionChecker,
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
    transaction::{AuxiliaryInfo, EntryFunction, SignedTransaction, TransactionExecutableRef},
};
use mono_move_core::{
    intern_type_tag, native::NativeExtensions, types::InternedTypeList, GasMeter, Interner,
};
use mono_move_loader::{Loader, LoadingPolicy, LoweringPolicy};
use mono_move_natives::TransactionContextExtension;
use mono_move_runtime::InterpreterContext;

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
        let txn_data = TxnMetadata::new(txn, aux_info);
        let (entry, ty_args) = match self.prepare_user_transaction(txn, &txn_data) {
            Ok(prepared) => prepared,
            // Nothing was read yet, so there is no read set to hand back.
            Err(reason) => {
                return TxnOutcome::Discarded {
                    reason,
                    effects: None,
                }
            },
        };

        let mut interp = self.user_session(&txn_data);
        match self.execute_user_transaction_impl(&mut interp, &txn_data, entry, ty_args) {
            Ok((status, fee_statement)) => TxnOutcome::Executed {
                status,
                fee_statement,
                effects: interp.finish(),
            },
            // The session may already have read global storage, and those reads
            // have to be validated even though the writes are dropped.
            Err(reason) => TxnOutcome::Discarded {
                reason,
                effects: Some(interp.finish()),
            },
        }
    }

    /// Rejects what this executor cannot execute, before touching any state,
    /// and resolves the payload's entry function and type arguments.
    fn prepare_user_transaction<'txn>(
        &self,
        txn: &'txn SignedTransaction,
        txn_data: &TxnMetadata,
    ) -> Result<(&'txn EntryFunction, InternedTypeList), DiscardReason> {
        let gas_params = self.env.gas_params().as_ref().map_err(|e| {
            DiscardReason::InvariantViolation(format!("the gas schedule is unavailable: {e}"))
        })?;
        PreExecutionChecker::new(gas_params, self.env.gas_feature_version(), txn_data)
            .run_checks()
            .map_err(DiscardReason::PreExecutionCheck)?;

        let entry = match txn.payload().executable_ref() {
            Ok(TransactionExecutableRef::EntryFunction(entry)) => entry,
            // TODO(completeness): scripts, multisig payloads, encrypted
            // transactions, module publishing.
            Ok(TransactionExecutableRef::Script(_))
            | Ok(TransactionExecutableRef::Encrypted)
            | Ok(TransactionExecutableRef::Empty)
            | Err(_) => {
                return Err(DiscardReason::Unsupported(
                    "anything but entry-function payloads",
                ))
            },
        };
        // TODO(security): these type arguments are user supplied, so interning
        // them can pollute the global caches. Needs a bound.
        let interned_ty_args = entry
            .ty_args()
            .iter()
            .map(|tag| intern_type_tag(tag, self.guard))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| DiscardReason::InvalidTypeArgument(format!("{e:#}")))?;
        Ok((entry, self.guard.type_list_of(&interned_ty_args)))
    }

    /// One session hosts the whole transaction: prologue, payload, epilogue.
    fn user_session(&self, txn_data: &TxnMetadata) -> InterpreterContext<'guard> {
        // TODO(completeness): make the loading policy configurable.
        let loader = Loader::new_with_policy(
            self.guard,
            self.module_provider,
            LoadingPolicy::Lazy(LoweringPolicy::Lazy),
            self.natives,
        );
        InterpreterContext::new(
            loader,
            // TODO(metering): MonoMove gas units are uncalibrated; budgeting
            // 1:1 against the transaction's gas units is a placeholder.
            GasMeter::new(txn_data.max_gas_amount),
            self.data_provider,
            self.natives,
        )
        .with_extensions(transaction_extensions(txn_data, self.usage))
    }

    /// Runs the prologue, payload, and epilogue in an already-opened session.
    fn execute_user_transaction_impl(
        &self,
        interp: &mut InterpreterContext<'guard>,
        txn_data: &TxnMetadata,
        entry: &EntryFunction,
        ty_args: InternedTypeList,
    ) -> Result<(ExecutionStatus, FeeStatement), DiscardReason> {
        let guard = self.guard;
        let max_gas = txn_data.max_gas_amount;
        let signers = ValidationSigners::new(txn_data);

        // ============================ Prologue ==============================
        // Validate the transaction (auth key, sequence number or nonce, fee coverage etc.)
        run_prologue(interp, guard, &signers, txn_data).map_err(|failure| {
            DiscardReason::Failure {
                stage: ExecutionStage::Prologue,
                failure,
            }
        })?;
        // A failed payload rolls back to here, so prologue effects (e.g. nonce insertion) survive.
        checkpoint(interp)?;

        // ========================== User payload ============================
        // An unmetered payload leaves the balance untouched, making the
        // epilogue charge nothing.
        let payload_result = if self.unmetered {
            interp.unmetered(|interp| self.execute_entry_function(interp, txn_data, entry, ty_args))
        } else {
            self.execute_entry_function(interp, txn_data, entry, ty_args)
        };
        let gas_remaining = interp.gas_balance();
        let gas_used = max_gas.saturating_sub(gas_remaining);

        // TODO(metering): charge gas for global storage writes and events.

        // A failed payload keeps only the prologue's effects; the epilogue below
        // still charges the fee.
        let payload_succeeded = payload_result.is_ok();
        let execution_status = match payload_result {
            Ok(()) => ExecutionStatus::Success,
            Err(failure) => {
                rollback(interp, 1)?;
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
                txn_data,
                fee_statement,
                gas_remaining,
            )
        };
        let execution_status = match epilogue(interp) {
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
                rollback(interp, 1)?;
                epilogue(interp).map_err(|failure| DiscardReason::Failure {
                    stage: ExecutionStage::EpilogueRetry,
                    failure,
                })?;
                ExecutionStatus::Failure {
                    stage: ExecutionStage::Epilogue,
                    failure,
                }
            },
        };

        Ok((execution_status, fee_statement))
    }

    fn execute_entry_function(
        &self,
        interp: &mut InterpreterContext<'guard>,
        txn_data: &TxnMetadata,
        entry: &EntryFunction,
        ty_args: InternedTypeList,
    ) -> Result<(), MoveExecutionFailure> {
        // TODO(security, completeness): entry-function validation -- `entry`
        // visibility, no return values, allowed argument types, and constructed
        // arguments (`String`, `Object<T>`, `Option<..>`) from
        // `transaction_arg_validation`.

        // TODO(completeness): multi-agent transactions are untested.
        let status = call_entry_function(
            self.guard,
            interp,
            &entry.module().address,
            entry.module().name(),
            entry.function(),
            ty_args,
            &txn_data.sender,
            &txn_data.secondary_signers,
            entry.args(),
        )?;

        call_result(status)
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
