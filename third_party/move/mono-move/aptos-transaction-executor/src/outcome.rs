// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    errors::{
        DiscardReason, ExecutionStatus, MaterializationError, NoEffectsReason, SystemTxnFailure,
    },
    materialize::{self, PartialOutput},
    providers::AptosDataProvider,
};
use aptos_types::{
    fee_statement::FeeStatement,
    on_chain_config::Features,
    transaction::{TransactionAuxiliaryData, TransactionOutput, TransactionStatus},
};
use mono_move_core::{value_layout::LayoutProvider, VMInternalError, VMResult};
use mono_move_runtime::{ResourceReadWriteSet, SessionEffects};

/// The outcome of one transaction, not yet materialized into a write set.
/// Intended to be consumed by the block coordinator for efficient handling.
pub enum TxnOutcome {
    /// Rejected without side effects. The effects are carried only so the
    /// session's reads are available; nothing here may be published.
    Discarded {
        reason: DiscardReason,
        /// `None` when the transaction was rejected before a session existed.
        effects: Option<SessionEffects>,
    },
    /// A system transaction failed unexpectedly: there is no per-transaction
    /// output, and the whole block must be aborted.
    UnexpectedSystemTransactionFailure(SystemTxnFailure),
    /// Closing the session failed, which happens only on a VM bug. Neither the
    /// reads nor the writes survive it, so there is no per-transaction output
    /// and the block must be aborted.
    Panic(VMInternalError),
    /// Committed with no side effects and a zero fee. The reason distinguishes
    /// a transaction that had nothing to do from a block epilogue whose
    /// failure was absorbed; both render as an empty success.
    //
    // TODO(correctness): revisit for hot-state parity. The legacy VM promotes
    // the read-only keys of a failed block epilogue, so the failed session's
    // read keys need to be preserved here.
    ExecutedNoEffects(NoEffectsReason),
    /// Executed: the fee is charged and the side effects are real, whether or
    /// not the payload succeeded.
    Executed {
        status: ExecutionStatus,
        fee_statement: FeeStatement,
        effects: SessionEffects,
    },
}

impl TxnOutcome {
    pub fn is_discarded(&self) -> bool {
        matches!(self, TxnOutcome::Discarded { .. })
    }

    /// The global storage the transaction touched, present whenever it opened a
    /// session. Only [`Self::Executed`] turns its writes into output; every
    /// other variant keeps this solely so its reads can be validated.
    pub fn read_write_set(&self) -> Option<&ResourceReadWriteSet> {
        match self {
            TxnOutcome::Executed { effects, .. } => Some(effects.read_write_set()),
            TxnOutcome::Discarded { effects, .. } => {
                effects.as_ref().map(SessionEffects::read_write_set)
            },
            // Nothing to commit or to validate.
            TxnOutcome::ExecutedNoEffects(_) => None,
            // The block is aborted, so there is nothing left to validate.
            TxnOutcome::UnexpectedSystemTransactionFailure(_) | TxnOutcome::Panic(_) => None,
        }
    }

    /// Whether this transaction emitted a reconfiguration (new-epoch) event. The
    /// block executor skips the remaining transactions when one occurs.
    pub fn has_new_epoch_event(&self) -> VMResult<bool> {
        match self {
            TxnOutcome::Executed { effects, .. } => mono_move_output::has_new_epoch_event(effects),
            TxnOutcome::Discarded { .. }
            | TxnOutcome::UnexpectedSystemTransactionFailure(_)
            | TxnOutcome::Panic(_)
            | TxnOutcome::ExecutedNoEffects(_) => Ok(false),
        }
    }

    /// Forces the outcome into a `TransactionOutput`: converts the status,
    /// serializes writes (merging resource-group members), and finalizes
    /// events. Fails only if the effects cannot be converted to storage
    /// formats, e.g. BCS serialized, which is abnormal — unlike a discard,
    /// which is a normal outcome carried in the output's status. `layouts`
    /// must describe the written and event values' interned types, and
    /// `provider` supplies resource-group pre-state for member merges.
    //
    // TODO(perf): this is only intended for compatibility and testing, and
    // the block coordinator should eventually handle the effects directly.
    pub fn materialize(
        &self,
        layouts: &impl LayoutProvider,
        provider: &dyn AptosDataProvider,
        features: &Features,
        auxiliary_data: TransactionAuxiliaryData,
    ) -> Result<TransactionOutput, MaterializationError> {
        let group_writes = match self {
            TxnOutcome::Executed { effects, .. } => {
                materialize::group_writes(effects, layouts, provider)?
            },
            TxnOutcome::Discarded { .. }
            | TxnOutcome::UnexpectedSystemTransactionFailure(_)
            | TxnOutcome::Panic(_)
            | TxnOutcome::ExecutedNoEffects(_) => vec![],
        };
        self.materialize_partial(layouts, features, auxiliary_data)?
            .with_group_writes(group_writes)
    }

    /// Like [`Self::materialize`], but leaves out resource groups: the caller
    /// assembles their writes and completes the output with them.
    pub fn materialize_partial(
        &self,
        layouts: &impl LayoutProvider,
        features: &Features,
        auxiliary_data: TransactionAuxiliaryData,
    ) -> Result<PartialOutput, MaterializationError> {
        match self {
            TxnOutcome::Discarded { reason, .. } => Ok(materialize::discarded_output(
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
            TxnOutcome::Panic(err) => Err(MaterializationError::new(vec![format!(
                "session could not be closed: {err:?}"
            )])),
            TxnOutcome::ExecutedNoEffects(_) => {
                Ok(materialize::empty_success_output(auxiliary_data))
            },
            TxnOutcome::Executed {
                status: execution_status,
                fee_statement,
                effects,
            } => {
                let vm_status = materialize::executed_vm_status(execution_status);
                // The `true` is `memory_limit_exceeded_as_miscellaneous_error`.
                // We always set this to true to replicate the latest behavior.
                let txn_status = TransactionStatus::from_vm_status(vm_status, features, true);
                materialize::executed_output(
                    effects,
                    layouts,
                    fee_statement.gas_used(),
                    txn_status,
                    auxiliary_data,
                )
            },
        }
    }
}
