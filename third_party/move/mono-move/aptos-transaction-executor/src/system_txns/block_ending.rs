// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::common::{system_txn_outcome, SystemTxnMetadata};
use crate::{errors::DiscardReason, executor::AptosTransactionExecutor, outcome::TxnOutcome};
use aptos_types::transaction::{block_epilogue::BlockEpiloguePayload, SessionId};

impl AptosTransactionExecutor<'_> {
    /// A state-checkpoint transaction renders to the kept, empty, fee-free
    /// success output the legacy AptosVM produces: no Move code runs.
    pub fn execute_state_checkpoint(&self) -> TxnOutcome {
        self.empty_kept_success(SessionId::void())
    }

    /// A V0 block epilogue carries only block-end info and distributes no fees,
    /// so it renders to the same kept, empty success as the legacy AptosVM. V1
    /// and V2 distribute transaction fees by running `0x1::block::block_epilogue`,
    /// which MonoMove does not support this milestone; a MonoMove block must run
    /// with `CALCULATE_TRANSACTION_FEE_FOR_DISTRIBUTION` disabled so the block
    /// coordinator only ever appends the V0 variant.
    pub fn execute_block_epilogue(&self, payload: &BlockEpiloguePayload) -> TxnOutcome {
        match payload {
            BlockEpiloguePayload::V0 { block_id, .. } => {
                self.empty_kept_success(SessionId::block_epilogue(*block_id))
            },
            BlockEpiloguePayload::V1 { .. } | BlockEpiloguePayload::V2 { .. } => {
                TxnOutcome::Discarded(DiscardReason::Unsupported(
                    "block epilogues with fee distribution",
                ))
            },
        }
    }

    /// Opens a system session and finishes it immediately, yielding a kept,
    /// empty, fee-free success output with no side effects.
    fn empty_kept_success(&self, session_id: SessionId) -> TxnOutcome {
        let metadata = SystemTxnMetadata::from_session_id(session_id);
        let interp = self.system_session(&metadata);
        system_txn_outcome(interp)
    }
}
