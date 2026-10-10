// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::common::{
    call_block_function, discard_system_session, system_txn_outcome, SystemTxnMetadata,
};
use crate::{errors::NoEffectsReason, executor::AptosTransactionExecutor, outcome::TxnOutcome};
use aptos_types::transaction::{BlockEpiloguePayload, FeeDistribution};
use move_value_view::IterAsMoveVector;

impl<'guard> AptosTransactionExecutor<'guard> {
    /// Executes a block-epilogue (system) transaction.
    ///
    /// Unlike the other system transactions, a failure never aborts the block:
    /// the outcome falls back to an empty success carrying the failure, and the
    /// failed session's effects are dropped.
    //
    // TODO(completeness): the legacy VM currently ignores the payload's
    // `to_make_hot` keys and emits no hot-state output, and so do we; revisit
    // when that changes.
    pub fn execute_block_epilogue_transaction(
        &self,
        block_epilogue: &BlockEpiloguePayload,
    ) -> TxnOutcome {
        let fee_distribution = match block_epilogue {
            // V0 carries no fee distribution: nothing runs on-chain.
            BlockEpiloguePayload::V0 { .. } => {
                return TxnOutcome::ExecutedNoEffects(NoEffectsReason::NothingToExecute)
            },
            BlockEpiloguePayload::V1 {
                fee_distribution, ..
            }
            | BlockEpiloguePayload::V2 {
                fee_distribution, ..
            } => fee_distribution,
        };
        let FeeDistribution::V0 { amount } = fee_distribution;
        let txn_data = SystemTxnMetadata::for_block_epilogue(block_epilogue);
        let mut interp = self.system_session(&txn_data);
        let result = call_block_function(
            &mut interp,
            self.symbols,
            self.symbols.block_epilogue,
            |call| {
                call.arg(&IterAsMoveVector(amount.keys().copied()))?;
                call.arg(&IterAsMoveVector(amount.values().copied()))
            },
        );
        match result {
            Ok(()) => system_txn_outcome(interp),
            Err(failure) => match discard_system_session(interp) {
                Ok(()) => {
                    TxnOutcome::ExecutedNoEffects(NoEffectsReason::BlockEpilogueFailed(failure))
                },
                // The epilogue's own failure is absorbed, but a VM error while
                // closing the session is not: nothing else would report it.
                Err(e) => TxnOutcome::Panic(e),
            },
        }
    }
}
