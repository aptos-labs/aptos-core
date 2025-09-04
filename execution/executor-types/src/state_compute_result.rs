// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    execution_output::ExecutionOutput, state_checkpoint_output::StateCheckpointOutput,
    ChunkCommitNotification, LedgerUpdateOutput,
};
use velor_crypto::{
    hash::{TransactionAccumulatorHasher, ACCUMULATOR_PLACEHOLDER_HASH},
    HashValue,
};
use velor_storage_interface::chunk_to_commit::ChunkToCommit;
use velor_types::{
    contract_event::ContractEvent,
    epoch_state::EpochState,
    proof::{accumulator::InMemoryTransactionAccumulator, AccumulatorExtensionProof},
    transaction::{Transaction, TransactionStatus, Version},
};
use std::sync::Arc;

/// A structure that summarizes the result of the execution needed for consensus to agree on.
/// The execution is responsible for generating the ID of the new state, which is returned in the
/// result.
///
/// Not every transaction in the payload succeeds: the returned vector keeps the boolean status
/// of success / failure of the transactions.
/// Note that the specific details of compute_status are opaque to StateMachineReplication,
/// which is going to simply pass the results between StateComputer and PayloadClient.
#[derive(Clone, Debug)]
pub struct StateComputeResult {
    pub execution_output: ExecutionOutput,
    pub state_checkpoint_output: StateCheckpointOutput,
    pub ledger_update_output: LedgerUpdateOutput,
}

impl StateComputeResult {
    pub fn new(
        execution_output: ExecutionOutput,
        state_checkpoint_output: StateCheckpointOutput,
        ledger_update_output: LedgerUpdateOutput,
    ) -> Self {
        Self {
            execution_output,
            state_checkpoint_output,
            ledger_update_output,
        }
    }

    pub fn new_dummy_with_accumulator(
        transaction_accumulator: Arc<InMemoryTransactionAccumulator>,
    ) -> Self {
        Self {
            execution_output: ExecutionOutput::new_dummy(),
            state_checkpoint_output: StateCheckpointOutput::new_dummy(),
            ledger_update_output: LedgerUpdateOutput::new_empty(transaction_accumulator),
        }
    }

    /// generate a new dummy state compute result with a given root hash.
    /// this function is used in RandomComputeResultStateComputer to assert that the compute
    /// function is really called.
    pub fn new_dummy_with_root_hash(root_hash: HashValue) -> Self {
        Self {
            execution_output: ExecutionOutput::new_dummy(),
            state_checkpoint_output: StateCheckpointOutput::new_dummy(),
            ledger_update_output: LedgerUpdateOutput::new_dummy_with_root_hash(root_hash),
        }
    }

    /// generate a new dummy state compute result with ACCUMULATOR_PLACEHOLDER_HASH as the root hash.
    /// this function is used in ordering_state_computer as a dummy state compute result,
    /// where the real compute result is generated after ordering_state_computer.commit pushes
    /// the blocks and the finality proof to the execution phase.
    pub fn new_dummy() -> Self {
        Self::new_dummy_with_root_hash(*ACCUMULATOR_PLACEHOLDER_HASH)
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn new_dummy_with_input_txns(txns: Vec<Transaction>) -> Self {
        Self {
            execution_output: ExecutionOutput::new_dummy_with_input_txns(txns),
            state_checkpoint_output: StateCheckpointOutput::new_dummy(),
            ledger_update_output: LedgerUpdateOutput::new_dummy(),
        }
    }

    pub fn root_hash(&self) -> HashValue {
        self.ledger_update_output.transaction_accumulator.root_hash
    }

    pub fn compute_status_for_input_txns(&self) -> &Vec<TransactionStatus> {
        &self.execution_output.statuses_for_input_txns
    }

    pub fn num_input_transactions(&self) -> usize {
        self.execution_output.statuses_for_input_txns.len()
    }

    pub fn num_transactions_to_commit(&self) -> usize {
        self.execution_output.num_transactions_to_commit()
    }

    pub fn epoch_state(&self) -> &Option<EpochState> {
        &self.execution_output.next_epoch_state
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

    pub fn transactions_to_commit(&self) -> &[Transaction] {
        &self.execution_output.to_commit.transactions
    }

    pub fn transaction_info_hashes(&self) -> &Vec<HashValue> {
        &self.ledger_update_output.transaction_info_hashes
    }

    pub fn expect_last_version(&self) -> Version {
        self.execution_output.expect_last_version()
    }

    pub fn next_version(&self) -> Version {
        self.execution_output.next_version()
    }

    pub fn last_version_or_0(&self) -> Version {
        self.next_version().saturating_sub(1)
    }

    pub fn has_reconfiguration(&self) -> bool {
        self.execution_output.next_epoch_state.is_some()
    }

    pub fn subscribable_events(&self) -> &[ContractEvent] {
        &self.execution_output.subscribable_events
    }

    pub fn make_chunk_commit_notification(&self) -> ChunkCommitNotification {
        ChunkCommitNotification {
            subscribable_events: self
                .execution_output
                .subscribable_events
                .get(Some("wait_for_subscribable_events"))
                .clone(),
            committed_transactions: self.execution_output.to_commit.transactions.clone(),
            reconfiguration_occurred: self.execution_output.next_epoch_state.is_some(),
        }
    }

    pub fn as_chunk_to_commit(&self) -> ChunkToCommit {
        ChunkToCommit {
            first_version: self.ledger_update_output.first_version(),
            transactions: &self.execution_output.to_commit.transactions,
            persisted_auxiliary_infos: &self.execution_output.to_commit.persisted_auxiliary_infos,
            transaction_outputs: &self.execution_output.to_commit.transaction_outputs,
            transaction_infos: &self.ledger_update_output.transaction_infos,
            state: &self.execution_output.result_state,
            state_summary: &self.state_checkpoint_output.state_summary,
            state_update_refs: self.execution_output.to_commit.state_update_refs(),
            state_reads: &self.execution_output.state_reads,
            is_reconfig: self.execution_output.next_epoch_state.is_some(),
        }
    }
}
