// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::common::{call_block_function, serialize, system_txn_outcome, SystemTxnMetadata};
use crate::{errors::NoEffectsReason, executor::AptosTransactionExecutor, outcome::TxnOutcome};
use aptos_types::transaction::{BlockEpiloguePayload, FeeDistribution};
use move_core_types::{ident_str, identifier::IdentStr};

const BLOCK_EPILOGUE: &IdentStr = ident_str!("block_epilogue");

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
    ) -> TxnOutcome<'guard> {
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
        let (validator_indices, amounts): (Vec<u64>, Vec<u64>) = amount
            .iter()
            .map(|(index, amount)| (*index, *amount))
            .unzip();
        let args = match (serialize(&validator_indices), serialize(&amounts)) {
            (Ok(validator_indices), Ok(amounts)) => [validator_indices, amounts],
            (Err(failure), _) | (_, Err(failure)) => {
                return TxnOutcome::ExecutedNoEffects(NoEffectsReason::BlockEpilogueFailed(failure))
            },
        };
        let txn_data = SystemTxnMetadata::for_block_epilogue(block_epilogue);
        let mut interp = self.system_session(&txn_data);
        match call_block_function(&mut interp, self.guard, BLOCK_EPILOGUE, &args) {
            Ok(()) => system_txn_outcome(interp),
            Err(failure) => {
                TxnOutcome::ExecutedNoEffects(NoEffectsReason::BlockEpilogueFailed(failure))
            },
        }
    }
}
