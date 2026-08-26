// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    errors::{
        DiscardReason, ExecutionStatus, MaterializationError, NoEffectsReason, SystemTxnFailure,
    },
    materialize,
    providers::AptosDataProvider,
};
use aptos_types::{
    fee_statement::FeeStatement,
    on_chain_config::Features,
    transaction::{TransactionAuxiliaryData, TransactionOutput, TransactionStatus},
};
use mono_move_runtime::SessionEffects;

/// The outcome of one transaction, not yet materialized into a write set.
/// Intended to be consumed by the block coordinator for efficient handling.
pub enum TxnOutcome<'guard> {
    /// Rejected without side effects.
    Discarded(DiscardReason),
    /// A system transaction failed unexpectedly: there is no per-transaction
    /// output, and the whole block must be aborted.
    UnexpectedSystemTransactionFailure(SystemTxnFailure),
    /// Committed with no side effects and a zero fee. The reason distinguishes
    /// a transaction that had nothing to do from a block epilogue whose
    /// failure was absorbed; both render as an empty success.
    ExecutedNoEffects(NoEffectsReason),
    /// Executed: the fee is charged and the side effects are real, whether or
    /// not the payload succeeded.
    Executed {
        status: ExecutionStatus,
        fee_statement: FeeStatement,
        effects: SessionEffects<'guard>,
    },
}

impl TxnOutcome<'_> {
    pub fn is_discarded(&self) -> bool {
        matches!(self, TxnOutcome::Discarded(_))
    }

    /// Forces the outcome into a `TransactionOutput`: converts the status,
    /// serializes writes (merging resource-group members), and finalizes
    /// events. Fails only if the effects cannot be converted to storage
    /// formats, e.g. BCS serialized, which is abnormal — unlike a discard,
    /// which is a normal outcome carried in the output's status.
    /// `provider` must be the exact provider instance used for execution so its
    /// pointer-keyed caches and resource-group pre-state match the effects;
    /// materialization rejects a different provider.
    //
    // TODO(perf): this is only intended for compatibility and testing, and
    // the block coordinator should eventually handle the effects directly.
    pub fn materialize(
        self,
        provider: &dyn AptosDataProvider,
        features: &Features,
        auxiliary_data: TransactionAuxiliaryData,
    ) -> Result<TransactionOutput, MaterializationError> {
        match self {
            TxnOutcome::Discarded(reason) => Ok(materialize::discarded_output(
                materialize::discard_to_vm_status(reason).status_code(),
                auxiliary_data,
            )),
            // A fatally failed system transaction has no output; the block
            // coordinator aborts the block instead.
            TxnOutcome::UnexpectedSystemTransactionFailure(failure) => {
                Err(MaterializationError::new(vec![format!(
                    "system transaction failed in {}: {:?}",
                    failure.call, failure.failure
                )]))
            },
            TxnOutcome::ExecutedNoEffects(_) => {
                Ok(materialize::empty_success_output(auxiliary_data))
            },
            TxnOutcome::Executed {
                status: execution_status,
                fee_statement,
                effects,
            } => {
                let vm_status = materialize::executed_vm_status(&execution_status);
                // The `true` is `memory_limit_exceeded_as_miscellaneous_error`.
                // We always set this to true to replicate the latest behavior.
                let txn_status = TransactionStatus::from_vm_status(vm_status, features, true);
                materialize::executed_output(
                    &effects,
                    provider,
                    fee_statement.gas_used(),
                    txn_status,
                    auxiliary_data,
                )
            },
        }
    }
}
