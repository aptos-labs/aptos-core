// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    calls::{call_function, CallStatus},
    errors::{DiscardReason, ExecutionStage, ExecutionStatus, MoveExecutionFailure},
    metadata::TxnMetadata,
    natives::transaction_extensions,
    outcome::TxnOutcome,
    sys_calls::{run_epilogue, run_prologue, validation_signers},
};
use aptos_types::{
    fee_statement::FeeStatement,
    on_chain_config::Features,
    state_store::state_storage_usage::StateStorageUsage,
    transaction::{EntryFunction, SignedTransaction, TransactionExecutableRef},
};
use mono_move_core::{
    intern_type_tag, storage::module_provider::ModuleProvider, types::InternedTypeList, GasMeter,
    Interner, ResourceProvider,
};
use mono_move_global_context::ExecutionGuard;
use mono_move_loader::{Loader, LoadingPolicy, LoweringPolicy};
use mono_move_runtime::{InterpreterContext, ProductionNativeRegistry};

/// The Aptos transaction executor on MonoMove (the legacy AptosVM's role).
///
/// It is designed to be a transient object borrowing various contexts from the block coordinator,
/// for transaction execution (most likely just a single one).
pub struct AptosTransactionExecutor<'a> {
    guard: &'a ExecutionGuard<'a>,
    // TODO(cleanup): We are considering moving the native registry into the GlobalContext.
    // This parameter may disappear once it moves there.
    natives: &'a ProductionNativeRegistry,
    module_provider: &'a dyn ModuleProvider,
    data_provider: &'a dyn ResourceProvider,
    // TODO(completeness): Consider growing these into the full `AptosEnvironment` supplied by
    // the coordinator.
    // Should however rethink whether we can simply reuse the existing `AptosEnvironment` struct.
    //
    // Nothing reads the features yet -- gas configuration and the pre-execution
    // checks will -- but the coordinator already supplies them.
    #[allow(dead_code)]
    features: &'a Features,
    /// State storage usage at the current epoch's beginning.
    usage: StateStorageUsage,
}

impl<'a> AptosTransactionExecutor<'a> {
    /// Constructs a new `AptosTransactionExecutor`. The borrowed contexts are
    /// built and owned by the block coordinator, which reuses them across the
    /// executors it creates.
    pub fn new(
        guard: &'a ExecutionGuard<'a>,
        natives: &'a ProductionNativeRegistry,
        module_provider: &'a dyn ModuleProvider,
        data_provider: &'a dyn ResourceProvider,
        features: &'a Features,
        usage: StateStorageUsage,
    ) -> Self {
        Self {
            guard,
            natives,
            module_provider,
            data_provider,
            features,
            usage,
        }
    }

    /// Executes one user transaction, returning its side effects unmaterialized (see [`TxnOutcome`]).
    pub fn execute_user_transaction(&self, txn: &SignedTransaction) -> TxnOutcome {
        match self.execute_user_transaction_impl(txn) {
            Ok(outcome) => outcome,
            Err(reason) => TxnOutcome::Discarded(reason),
        }
    }

    /// Actual implementation of the transaction execution flow.
    fn execute_user_transaction_impl(
        &self,
        txn: &SignedTransaction,
    ) -> Result<TxnOutcome, DiscardReason> {
        let guard = self.guard;

        // ======================== Pre-execution checks ========================
        // Reject what this executor cannot execute, before touching any state.
        //
        // TODO(metering, security): gas checks (`check_gas`): txn size bounds,
        // gas price bounds, intrinsic gas coverage. Without them a transaction
        // outside the configured bounds -- including a zero gas price -- buys a
        // full `max_gas_amount` budget for free.
        let txn_data = TxnMetadata::new(txn);

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
        let mut interp = InterpreterContext::new_idle(
            loader,
            // TODO(metering): MonoMove gas units are uncalibrated; budgeting
            // 1:1 against the transaction's gas units is a placeholder.
            GasMeter::new(max_gas),
            self.data_provider,
            self.natives,
        )
        .with_extensions(extensions);

        let signers = validation_signers(&txn_data);

        // ============================ Prologue ==============================
        // Validate the transaction (auth key, sequence number or nonce, fee coverage etc.)
        run_prologue(&mut interp, guard, &signers, &txn_data).map_err(|failure| {
            DiscardReason::Failure {
                stage: ExecutionStage::Prologue,
                failure,
            }
        })?;
        // A failed payload rolls back to here, so prologue effects (e.g. nonce insertion) survive.
        checkpoint(&mut interp)?;

        // ========================== User payload ============================
        let payload_result = self.execute_entry_function(&mut interp, &txn_data, entry, ty_args);
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
        let epilogue = |interp: &mut InterpreterContext<'_>| {
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
        Ok(TxnOutcome::Executed {
            status: execution_status,
            fee_statement,
            effects: interp.finish(),
        })
    }

    fn execute_entry_function(
        &self,
        interp: &mut InterpreterContext<'_>,
        txn_data: &TxnMetadata,
        entry: &EntryFunction,
        ty_args: InternedTypeList,
    ) -> Result<(), MoveExecutionFailure> {
        // TODO(security, completeness): entry-function validation -- `entry`
        // visibility, no return values, allowed argument types, and constructed
        // arguments (`String`, `Object<T>`, `Option<..>`) from
        // `transaction_arg_validation`.

        // Signers: the sender plus any secondary signers.
        // TODO(completeness): multi-agent transactions are untested.
        let mut signer_bufs = vec![txn_data.sender.into_bytes()];
        for secondary in &txn_data.secondary_signers {
            signer_bufs.push(secondary.into_bytes());
        }

        let status = call_function(
            self.guard,
            interp,
            &entry.module().address,
            entry.module().name(),
            entry.function(),
            ty_args,
            &signer_bufs,
            entry.args(),
        )
        .map_err(MoveExecutionFailure::RuntimeError)?;

        match status {
            CallStatus::Success => Ok(()),
            CallStatus::Abort {
                code,
                message,
                location,
            } => Err(MoveExecutionFailure::Abort {
                code,
                message,
                location,
            }),
        }
    }
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
