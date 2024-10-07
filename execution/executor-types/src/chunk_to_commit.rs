// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    chunk_output::ChunkOutput, state_checkpoint_output::StateCheckpointOutput, LedgerUpdateOutput,
    StateComputeResult,
};
use aptos_types::{
    proof::accumulator::InMemoryTransactionAccumulator,
    transaction::{TransactionOutputProvider, TransactionToCommit},
};
use itertools::izip;

pub struct ChunkToCommit<'a> {
    pub chunk_output: &'a ChunkOutput,
    pub state_checkpoint_output: &'a StateCheckpointOutput,
    pub ledger_update_output: &'a LedgerUpdateOutput,
}

impl<'a> ChunkToCommit<'a> {
    pub fn make_state_compute_result(
        &self,
        parent_txn_accumulator: &InMemoryTransactionAccumulator,
    ) -> StateComputeResult {
        StateComputeResult::new(
            self.ledger_update_output
                .transaction_accumulator
                .root_hash(),
            self.ledger_update_output
                .transaction_accumulator
                .frozen_subtree_roots()
                .to_vec(),
            self.ledger_update_output
                .transaction_accumulator
                .num_leaves(),
            parent_txn_accumulator.frozen_subtree_roots().to_vec(),
            parent_txn_accumulator.num_leaves(),
            self.chunk_output.next_epoch_state.clone(),
            self.chunk_output.statuses_for_input_txns.clone(),
            self.ledger_update_output.transaction_info_hashes.clone(),
            self.ledger_update_output.subscribable_events.clone(),
            self.chunk_output.block_end_info.clone(),
        )
    }

    pub fn make_transactions_to_commit(&self) -> Vec<TransactionToCommit> {
        izip!(
            self.chunk_output.to_commit.iter(),
            &self.state_checkpoint_output.per_version_state_updates,
            &self.ledger_update_output.transaction_infos,
        )
        .map(|((txn, txn_out), state_updates, txn_info)| {
            TransactionToCommit::new(
                txn.clone(),
                txn_info.clone(),
                state_updates.clone(),
                txn_out.get_transaction_output().write_set().clone(),
                txn_out.get_transaction_output().events().to_vec(),
                txn_out.is_reconfig(),
                txn_out.get_transaction_output().auxiliary_data().clone(),
            )
        })
        .collect()
    }
}
