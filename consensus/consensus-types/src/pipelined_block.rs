// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block::Block,
    common::{Payload, Round},
    order_vote_proposal::OrderVoteProposal,
    quorum_cert::QuorumCert,
    vote_proposal::VoteProposal,
};
use aptos_crypto::hash::{HashValue, ACCUMULATOR_PLACEHOLDER_HASH};
use aptos_executor_types::StateComputeResult;
use aptos_logger::{error, info, warn};
use aptos_types::{
    block_info::BlockInfo,
    contract_event::ContractEvent,
    randomness::Randomness,
    transaction::{SignedTransaction, TransactionStatus},
    validator_txn::ValidatorTransaction,
};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    fmt::{Debug, Display, Formatter},
    sync::Arc,
    time::{Duration, Instant},
};

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrderedBlockWindow {
    blocks: Vec<Arc<PipelinedBlock>>,
}

impl OrderedBlockWindow {
    pub fn new(blocks: Vec<Arc<PipelinedBlock>>) -> Self {
        Self { blocks }
    }

    pub fn empty() -> Self {
        Self { blocks: vec![] }
    }

    // TODO: clone required?
    pub fn blocks(&self) -> Vec<Block> {
        self.blocks.iter().map(|b| b.block().clone()).collect()
    }

    pub fn pipelined_blocks(&self) -> &Vec<Arc<PipelinedBlock>> {
        &self.blocks
    }
}

/// A representation of a block that has been added to the execution pipeline. It might either be in ordered
/// or in executed state. In the ordered state, the block is waiting to be executed. In the executed state,
/// the block has been executed and the output is available.
#[derive(Clone, Eq, PartialEq)]
pub struct PipelinedBlock {
    /// Block data that cannot be regenerated.
    block: Block,
    /// A window of blocks that are needed for execution with the execution pool, excluding the current block
    block_window: OrderedBlockWindow,
    /// Input transactions in the order of execution
    input_transactions: Vec<SignedTransaction>,
    /// The state_compute_result is calculated for all the pending blocks prior to insertion to
    /// the tree. The execution results are not persisted: they're recalculated again for the
    /// pending blocks upon restart.
    state_compute_result: StateComputeResult,
    randomness: OnceCell<Randomness>,
    pipeline_insertion_time: OnceCell<Instant>,
    execution_summary: Arc<OnceCell<ExecutionSummary>>,
    committed_transactions: Arc<OnceCell<Vec<HashValue>>>,
}

impl Serialize for PipelinedBlock {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        #[serde(rename = "PipelineBlock")]
        struct SerializedBlock<'a> {
            block: &'a Block,
            block_window: &'a OrderedBlockWindow,
            input_transactions: &'a Vec<SignedTransaction>,
            state_compute_result: &'a StateComputeResult,
            randomness: Option<&'a Randomness>,
        }

        let serialized = SerializedBlock {
            block: &self.block,
            block_window: &self.block_window,
            input_transactions: &self.input_transactions,
            state_compute_result: &self.state_compute_result,
            randomness: self.randomness.get(),
        };
        serialized.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PipelinedBlock {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename = "PipelineBlock")]
        struct SerializedBlock {
            block: Block,
            block_window: OrderedBlockWindow,
            input_transactions: Vec<SignedTransaction>,
            state_compute_result: StateComputeResult,
            randomness: Option<Randomness>,
        }

        let SerializedBlock {
            block,
            block_window,
            input_transactions,
            state_compute_result,
            randomness,
        } = SerializedBlock::deserialize(deserializer)?;

        info!(
            "Deserialized PipelinedBlock: ({}, {}) {}",
            block.epoch(),
            block.round(),
            block.id()
        );
        let block = PipelinedBlock {
            block,
            block_window,
            input_transactions,
            state_compute_result,
            randomness: OnceCell::new(),
            pipeline_insertion_time: OnceCell::new(),
            execution_summary: Arc::new(OnceCell::new()),
            committed_transactions: Arc::new(OnceCell::new()),
        };
        if let Some(r) = randomness {
            block.set_randomness(r);
        }
        Ok(block)
    }
}

impl PipelinedBlock {
    pub fn set_execution_result(
        mut self,
        input_transactions: Vec<SignedTransaction>,
        result: StateComputeResult,
        execution_time: Duration,
    ) -> Self {
        self.state_compute_result = result;
        self.input_transactions = input_transactions;

        let mut committed_transactions = vec![];
        let mut to_commit = 0;
        let mut to_retry = 0;
        for (txn, status) in self
            .input_transactions
            .iter()
            .zip(self.state_compute_result.compute_status_for_input_txns())
        {
            match status {
                TransactionStatus::Keep(_) => {
                    // TODO: was this already computed?
                    committed_transactions.push(txn.committed_hash());
                    to_commit += 1
                },
                TransactionStatus::Retry => to_retry += 1,
                _ => {},
            }
        }

        let execution_summary = ExecutionSummary {
            payload_len: self
                .block
                .payload()
                .map_or(0, |payload| payload.len_for_execution()),
            to_commit,
            to_retry,
            execution_time,
            root_hash: self.state_compute_result.root_hash(),
        };

        // We might be retrying execution, so it might have already been set.
        // Because we use this for statistics, it's ok that we drop the newer value.
        if let Some(previous) = self.execution_summary.get() {
            if previous.root_hash == execution_summary.root_hash
                || previous.root_hash == *ACCUMULATOR_PLACEHOLDER_HASH
            {
                warn!(
                    "Skipping re-inserting execution result, from {:?} to {:?}",
                    previous, execution_summary
                );
            } else {
                error!(
                    "Re-inserting execution result with different root hash: from {:?} to {:?}",
                    previous, execution_summary
                );
            }
        } else {
            self.execution_summary
                .set(execution_summary)
                .expect("inserting into empty execution summary");
        }

        if self.committed_transactions.get().is_some() {
            error!("Re-inserting committed transactions");
        } else {
            info!(
                "Setting committed transactions: ({}, {}) {}",
                self.epoch(),
                self.round(),
                self.id()
            );
            self.committed_transactions
                .set(committed_transactions)
                .expect("inserting into empty committed transactions");
        }

        self
    }

    pub fn set_randomness(&self, randomness: Randomness) {
        assert!(self.randomness.set(randomness).is_ok());
    }

    pub fn set_insertion_time(&self) {
        assert!(self.pipeline_insertion_time.set(Instant::now()).is_ok());
    }
}

impl Debug for PipelinedBlock {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Display for PipelinedBlock {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", self.block())
    }
}

impl PipelinedBlock {
    pub fn new(
        block: Block,
        input_transactions: Vec<SignedTransaction>,
        state_compute_result: StateComputeResult,
    ) -> Self {
        info!(
            "New PipelinedBlock with block_id: {}, parent_id: {}, round: {}, epoch: {}, txns: {}",
            block.id(),
            block.parent_id(),
            block.round(),
            block.epoch(),
            block.payload().map_or(0, |p| p.len())
        );

        Self {
            block,
            block_window: OrderedBlockWindow::new(vec![]),
            input_transactions,
            state_compute_result,
            randomness: OnceCell::new(),
            pipeline_insertion_time: OnceCell::new(),
            execution_summary: Arc::new(OnceCell::new()),
            committed_transactions: Arc::new(OnceCell::new()),
        }
    }

    pub fn new_ordered(block: Block, window: OrderedBlockWindow) -> Self {
        info!(
            "New Ordered PipelinedBlock with block_id: {}, parent_id: {}, round: {}, epoch: {}, txns: {}",
            block.id(),
            block.parent_id(),
            block.round(),
            block.epoch(),
            block.payload().map_or(0, |p| p.len())
        );
        Self {
            block,
            block_window: window,
            input_transactions: vec![],
            state_compute_result: StateComputeResult::new_dummy(),
            randomness: OnceCell::new(),
            pipeline_insertion_time: OnceCell::new(),
            execution_summary: Arc::new(OnceCell::new()),
            committed_transactions: Arc::new(OnceCell::new()),
        }
    }

    pub fn block(&self) -> &Block {
        &self.block
    }

    // TODO: make this an Option?
    pub fn block_window(&self) -> &OrderedBlockWindow {
        &self.block_window
    }

    pub fn id(&self) -> HashValue {
        self.block().id()
    }

    pub fn input_transactions(&self) -> &Vec<SignedTransaction> {
        &self.input_transactions
    }

    pub fn epoch(&self) -> u64 {
        self.block.epoch()
    }

    pub fn payload(&self) -> Option<&Payload> {
        self.block().payload()
    }

    pub fn parent_id(&self) -> HashValue {
        self.block.parent_id()
    }

    pub fn quorum_cert(&self) -> &QuorumCert {
        self.block().quorum_cert()
    }

    pub fn round(&self) -> Round {
        self.block().round()
    }

    pub fn validator_txns(&self) -> Option<&Vec<ValidatorTransaction>> {
        self.block().validator_txns()
    }

    pub fn timestamp_usecs(&self) -> u64 {
        self.block().timestamp_usecs()
    }

    pub fn compute_result(&self) -> &StateComputeResult {
        &self.state_compute_result
    }

    pub fn randomness(&self) -> Option<&Randomness> {
        self.randomness.get()
    }

    pub fn has_randomness(&self) -> bool {
        self.randomness.get().is_some()
    }

    pub fn block_info(&self) -> BlockInfo {
        self.block().gen_block_info(
            self.compute_result().root_hash(),
            self.compute_result().version(),
            self.compute_result().epoch_state().clone(),
        )
    }

    pub fn vote_proposal(&self) -> VoteProposal {
        VoteProposal::new(
            self.compute_result().extension_proof(),
            self.block.clone(),
            self.compute_result().epoch_state().clone(),
            true,
        )
    }

    pub fn order_vote_proposal(&self, quorum_cert: Arc<QuorumCert>) -> OrderVoteProposal {
        OrderVoteProposal::new(self.block.clone(), self.block_info(), quorum_cert)
    }

    pub fn subscribable_events(&self) -> Vec<ContractEvent> {
        // reconfiguration suffix don't count, the state compute result is carried over from parents
        if self.is_reconfiguration_suffix() {
            return vec![];
        }
        self.state_compute_result.subscribable_events().to_vec()
    }

    /// The block is suffix of a reconfiguration block if the state result carries over the epoch state
    /// from parent but has no transaction.
    pub fn is_reconfiguration_suffix(&self) -> bool {
        self.state_compute_result.has_reconfiguration()
            && self
                .state_compute_result
                .compute_status_for_input_txns()
                .is_empty()
    }

    pub fn elapsed_in_pipeline(&self) -> Option<Duration> {
        self.pipeline_insertion_time.get().map(|t| t.elapsed())
    }

    pub fn get_execution_summary(&self) -> Option<ExecutionSummary> {
        self.execution_summary.get().cloned()
    }

    pub fn wait_for_committed_transactions(&self) -> &[HashValue] {
        if self.block().is_genesis_block() || self.block.is_nil_block() {
            return &[];
        }

        info!(
            "Waiting for committed transactions: ({}, {}) {}",
            self.epoch(),
            self.round(),
            self.id()
        );
        let result = self.committed_transactions.wait();
        info!(
            "Done waiting for committed transactions: ({}, {}) {}",
            self.epoch(),
            self.round(),
            self.id()
        );
        result
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExecutionSummary {
    pub payload_len: u64,
    pub to_commit: u64,
    pub to_retry: u64,
    pub execution_time: Duration,
    pub root_hash: HashValue,
}
