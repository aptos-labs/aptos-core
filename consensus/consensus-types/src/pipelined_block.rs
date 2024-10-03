// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block::Block,
    common::{Payload, Round},
    order_vote_proposal::OrderVoteProposal,
    pipeline_execution_result::PipelineExecutionResult,
    quorum_cert::QuorumCert,
    vote_proposal::VoteProposal,
};
use aptos_crypto::hash::{HashValue, ACCUMULATOR_PLACEHOLDER_HASH};
use aptos_executor_types::{ExecutorResult, StateComputeResult};
use aptos_infallible::Mutex;
use aptos_logger::{error, warn};
use aptos_types::{
    block_info::BlockInfo,
    contract_event::ContractEvent,
    randomness::Randomness,
    transaction::{SignedTransaction, TransactionStatus},
    validator_txn::ValidatorTransaction,
};
use derivative::Derivative;
use futures::future::BoxFuture;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    fmt::{Debug, Display, Formatter},
    sync::Arc,
    time::{Duration, Instant},
};

/// A representation of a block that has been added to the execution pipeline. It might either be in ordered
/// or in executed state. In the ordered state, the block is waiting to be executed. In the executed state,
/// the block has been executed and the output is available.
#[derive(Derivative, Clone)]
#[derivative(Eq, PartialEq)]
pub struct PipelinedBlock {
    /// Block data that cannot be regenerated.
    block: Block,
    /// Input transactions in the order of execution
    input_transactions: Vec<SignedTransaction>,
    /// The state_compute_result is calculated for all the pending blocks prior to insertion to
    /// the tree. The execution results are not persisted: they're recalculated again for the
    /// pending blocks upon restart.
    state_compute_result: StateComputeResult,
    randomness: OnceCell<Randomness>,
    pipeline_insertion_time: OnceCell<Instant>,
    execution_summary: Arc<OnceCell<ExecutionSummary>>,
    #[derivative(PartialEq = "ignore")]
    pre_commit_fut: Arc<Mutex<Option<BoxFuture<'static, ExecutorResult<()>>>>>,
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
            input_transactions: &'a Vec<SignedTransaction>,
            state_compute_result: &'a StateComputeResult,
            randomness: Option<&'a Randomness>,
        }

        let serialized = SerializedBlock {
            block: &self.block,
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
            input_transactions: Vec<SignedTransaction>,
            state_compute_result: StateComputeResult,
            randomness: Option<Randomness>,
        }

        let SerializedBlock {
            block,
            input_transactions,
            state_compute_result,
            randomness,
        } = SerializedBlock::deserialize(deserializer)?;

        let block = PipelinedBlock {
            block,
            input_transactions,
            state_compute_result,
            randomness: OnceCell::new(),
            pipeline_insertion_time: OnceCell::new(),
            execution_summary: Arc::new(OnceCell::new()),
            pre_commit_fut: Arc::new(Mutex::new(None)),
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
        pipeline_execution_result: PipelineExecutionResult,
    ) -> Self {
        let PipelineExecutionResult {
            input_txns,
            result,
            execution_time,
            pre_commit_fut,
        } = pipeline_execution_result;

        self.state_compute_result = result;
        self.input_transactions = input_txns;
        self.pre_commit_fut = Arc::new(Mutex::new(Some(pre_commit_fut)));

        let mut to_commit = 0;
        let mut to_retry = 0;
        for txn in self.state_compute_result.compute_status_for_input_txns() {
            match txn {
                TransactionStatus::Keep(_) => to_commit += 1,
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
        self
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn mark_successful_pre_commit_for_test(&self) {
        *self.pre_commit_fut.lock() = Some(Box::pin(async { Ok(()) }));
    }

    pub fn set_randomness(&self, randomness: Randomness) {
        assert!(self.randomness.set(randomness).is_ok());
    }

    pub fn set_insertion_time(&self) {
        assert!(self.pipeline_insertion_time.set(Instant::now()).is_ok());
    }

    pub fn take_pre_commit_fut(&self) -> BoxFuture<'static, ExecutorResult<()>> {
        self.pre_commit_fut
            .lock()
            .take()
            .expect("pre_commit_result_rx missing.")
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
        Self {
            block,
            input_transactions,
            state_compute_result,
            randomness: OnceCell::new(),
            pipeline_insertion_time: OnceCell::new(),
            execution_summary: Arc::new(OnceCell::new()),
            pre_commit_fut: Arc::new(Mutex::new(None)),
        }
    }

    pub fn new_ordered(block: Block) -> Self {
        Self {
            block,
            input_transactions: vec![],
            state_compute_result: StateComputeResult::new_dummy(),
            randomness: OnceCell::new(),
            pipeline_insertion_time: OnceCell::new(),
            execution_summary: Arc::new(OnceCell::new()),
            pre_commit_fut: Arc::new(Mutex::new(None)),
        }
    }

    pub fn block(&self) -> &Block {
        &self.block
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
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExecutionSummary {
    pub payload_len: u64,
    pub to_commit: u64,
    pub to_retry: u64,
    pub execution_time: Duration,
    pub root_hash: HashValue,
}
