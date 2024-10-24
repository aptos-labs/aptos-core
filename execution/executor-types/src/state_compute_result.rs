// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{ChunkCommitNotification, LedgerUpdateOutput};
use aptos_crypto::{
    hash::{TransactionAccumulatorHasher, ACCUMULATOR_PLACEHOLDER_HASH},
    HashValue,
};
use aptos_storage_interface::{chunk_to_commit::ChunkToCommit, state_delta::StateDelta};
use aptos_types::{
    contract_event::ContractEvent,
    epoch_state::EpochState,
    proof::{accumulator::InMemoryTransactionAccumulator, AccumulatorExtensionProof},
    transaction::{Transaction, TransactionStatus, Version},
};
use std::{cmp::max, sync::Arc};

/// A structure that summarizes the result of the execution needed for consensus to agree on.
/// The execution is responsible for generating the ID of the new state, which is returned in the
/// result.
///
/// Not every transaction in the payload succeeds: the returned vector keeps the boolean status
/// of success / failure of the transactions.
/// Note that the specific details of compute_status are opaque to StateMachineReplication,
/// which is going to simply pass the results between StateComputer and PayloadClient.
#[derive(Debug, Default, Clone)]
pub struct StateComputeResult {
    pub parent_state: Arc<StateDelta>,
    pub result_state: Arc<StateDelta>,
    pub ledger_update_output: LedgerUpdateOutput,
    /// If set, this is the new epoch info that should be changed to if this is committed.
    pub next_epoch_state: Option<EpochState>,
}

impl StateComputeResult {
    pub fn new(
        parent_state: Arc<StateDelta>,
        result_state: Arc<StateDelta>,
        ledger_update_output: LedgerUpdateOutput,
        next_epoch_state: Option<EpochState>,
    ) -> Self {
        Self {
            parent_state,
            result_state,
            ledger_update_output,
            next_epoch_state,
        }
    }

    pub fn new_empty(transaction_accumulator: Arc<InMemoryTransactionAccumulator>) -> Self {
        let result_state = Arc::new(StateDelta::new_empty());
        Self {
            parent_state: result_state.clone(),
            result_state,
            ledger_update_output: LedgerUpdateOutput::new_empty(transaction_accumulator),
            next_epoch_state: None,
        }
    }

    /// generate a new dummy state compute result with a given root hash.
    /// this function is used in RandomComputeResultStateComputer to assert that the compute
    /// function is really called.
    pub fn new_dummy_with_root_hash(root_hash: HashValue) -> Self {
        let result_state = Arc::new(StateDelta::new_empty());
        Self {
            parent_state: result_state.clone(),
            result_state,
            ledger_update_output: LedgerUpdateOutput::new_dummy_with_root_hash(root_hash),
            next_epoch_state: None,
        }
    }

    /// generate a new dummy state compute result with ACCUMULATOR_PLACEHOLDER_HASH as the root hash.
    /// this function is used in ordering_state_computer as a dummy state compute result,
    /// where the real compute result is generated after ordering_state_computer.commit pushes
    /// the blocks and the finality proof to the execution phase.
    pub fn new_dummy() -> Self {
        StateComputeResult::new_dummy_with_root_hash(*ACCUMULATOR_PLACEHOLDER_HASH)
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn new_dummy_with_input_txns(txns: Vec<Transaction>) -> Self {
        let result_state = Arc::new(StateDelta::new_empty());
        Self {
            parent_state: result_state.clone(),
            result_state,
            ledger_update_output: LedgerUpdateOutput::new_dummy_with_input_txns(txns),
            next_epoch_state: None,
        }
    }

    pub fn version(&self) -> Version {
        max(self.ledger_update_output.next_version(), 1)
            .checked_sub(1)
            .expect("Integer overflow occurred")
    }

    pub fn root_hash(&self) -> HashValue {
        self.ledger_update_output.transaction_accumulator.root_hash
    }

    pub fn compute_status_for_input_txns(&self) -> &Vec<TransactionStatus> {
        &self.ledger_update_output.statuses_for_input_txns
    }

    pub fn transactions_to_commit_len(&self) -> usize {
        self.ledger_update_output.transactions.len()
    }

    pub fn epoch_state(&self) -> &Option<EpochState> {
        &self.next_epoch_state
    }

    pub fn extension_proof(&self) -> AccumulatorExtensionProof<TransactionAccumulatorHasher> {
        AccumulatorExtensionProof::new(
            self.ledger_update_output
                .transaction_accumulator
                .frozen_subtree_roots
                .clone(),
            self.ledger_update_output.transaction_accumulator.num_leaves,
            self.transaction_info_hashes().to_vec(),
        )
    }

    pub fn transaction_info_hashes(&self) -> &Vec<HashValue> {
        &self.ledger_update_output.transaction_info_hashes
    }

    pub fn num_leaves(&self) -> u64 {
        self.ledger_update_output.next_version()
    }

    pub fn has_reconfiguration(&self) -> bool {
        self.next_epoch_state.is_some()
    }

    pub fn subscribable_events(&self) -> &[ContractEvent] {
        &self.ledger_update_output.subscribable_events
    }

    pub fn is_reconfiguration_suffix(&self) -> bool {
        self.has_reconfiguration() && self.compute_status_for_input_txns().is_empty()
    }

    pub fn make_chunk_commit_notification(&self) -> ChunkCommitNotification {
        ChunkCommitNotification {
            subscribable_events: self.ledger_update_output.subscribable_events.clone(),
            committed_transactions: self.ledger_update_output.transactions.clone(),
            reconfiguration_occurred: self.has_reconfiguration(),
        }
    }

    pub fn as_chunk_to_commit(&self) -> ChunkToCommit {
        ChunkToCommit {
            first_version: self.ledger_update_output.first_version(),
            transactions: &self.ledger_update_output.transactions,
            transaction_outputs: &self.ledger_update_output.transaction_outputs,
            transaction_infos: &self.ledger_update_output.transaction_infos,
            per_version_state_updates: &self.ledger_update_output.per_version_state_updates,
            base_state_version: self.parent_state.base_version,
            latest_in_memory_state: &self.result_state,
            state_updates_until_last_checkpoint: self
                .ledger_update_output
                .state_updates_until_last_checkpoint
                .as_ref(),
            sharded_state_cache: Some(&self.ledger_update_output.sharded_state_cache),
            is_reconfig: self.ledger_update_output.block_end_info.is_some(),
        }
    }
}
