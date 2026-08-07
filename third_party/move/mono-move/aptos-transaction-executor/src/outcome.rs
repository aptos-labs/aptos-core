// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    errors::{DiscardReason, ExecutionStatus, MaterializationError},
    materialize,
    providers::AptosDataProvider,
};
use aptos_types::{
    fee_statement::FeeStatement,
    on_chain_config::Features,
    transaction::{TransactionAuxiliaryData, TransactionOutput, TransactionStatus},
};
use mono_move_global_context::ExecutionGuard;
use mono_move_runtime::SessionEffects;

/// The outcome of one transaction, not yet materialized into a write set.
/// Intended to be consumed by the block coordinator for efficient handling.
pub enum TxnOutcome {
    /// Rejected without side effects.
    Discarded(DiscardReason),
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
        matches!(self, TxnOutcome::Discarded(_))
    }

    /// Forces the outcome into a `TransactionOutput`: converts the status,
    /// serializes writes (merging resource-group members), and finalizes
    /// events. Fails only if the effects cannot be converted to storage
    /// formats, e.g. BCS serialized, which is abnormal — unlike a discard,
    /// which is a normal outcome carried in the output's status.
    //
    // TODO(perf): this is only intended for compatibility and testing, and
    // the block coordinator should eventually handle the effects directly.
    pub fn materialize(
        self,
        guard: &ExecutionGuard,
        provider: &dyn AptosDataProvider,
        features: &Features,
        auxiliary_data: TransactionAuxiliaryData,
    ) -> Result<TransactionOutput, MaterializationError> {
        match self {
            TxnOutcome::Discarded(reason) => Ok(materialize::discarded_output(
                materialize::discard_to_vm_status(reason).status_code(),
                auxiliary_data,
            )),
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
                    guard,
                    provider,
                    fee_statement.gas_used(),
                    txn_status,
                    auxiliary_data,
                )
            },
        }
    }
}
