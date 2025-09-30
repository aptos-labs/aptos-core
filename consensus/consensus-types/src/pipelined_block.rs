// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block::Block,
    common::{Payload, Round},
    order_vote_proposal::OrderVoteProposal,
    pipeline::commit_vote::CommitVote,
    quorum_cert::QuorumCert,
    vote_proposal::VoteProposal,
    wrapped_ledger_info::WrappedLedgerInfo,
};
use anyhow::Error;
use aptos_crypto::hash::{HashValue, ACCUMULATOR_PLACEHOLDER_HASH};
use aptos_executor_types::{
    state_compute_result::StateComputeResult, ExecutorError, ExecutorResult,
};
use aptos_infallible::Mutex;
use aptos_logger::{error, info, warn};
use aptos_types::{
    block_info::BlockInfo,
    contract_event::ContractEvent,
    ledger_info::LedgerInfoWithSignatures,
    randomness::Randomness,
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction, SignedTransaction,
        TransactionStatus,
    },
    validator_txn::ValidatorTransaction,
};
use derivative::Derivative;
use futures::future::{join5, BoxFuture, Shared};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{
    fmt::{Debug, Display, Formatter},
    sync::{Arc, Weak},
    time::{Duration, Instant},
};
use tokio::{
    sync::oneshot,
    task::{AbortHandle, JoinError},
};

#[derive(Clone, Debug)]
pub enum TaskError {
    JoinError(Arc<JoinError>),
    InternalError(Arc<Error>),
    PropagatedError(Box<TaskError>),
}

impl Display for TaskError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskError::JoinError(e) => write!(f, "JoinError: {}", e),
            TaskError::InternalError(e) => write!(f, "InternalError: {}", e),
            TaskError::PropagatedError(e) => write!(f, "PropagatedError: {}", e),
        }
    }
}

impl From<Error> for TaskError {
    fn from(value: Error) -> Self {
        Self::InternalError(Arc::new(value))
    }
}
pub type TaskResult<T> = Result<T, TaskError>;
pub type TaskFuture<T> = Shared<BoxFuture<'static, TaskResult<T>>>;

pub type PrepareResult = (Arc<Vec<SignatureVerifiedTransaction>>, Option<u64>);
// First Option is whether randomness is enabled
// Second Option is whether randomness is skipped
pub type RandResult = (Option<Option<Randomness>>, bool);
pub type ExecuteResult = Duration;
pub type LedgerUpdateResult = (StateComputeResult, Duration, Option<u64>);
pub type PostLedgerUpdateResult = ();
pub type CommitVoteResult = CommitVote;
pub type PreCommitResult = StateComputeResult;
pub type NotifyStateSyncResult = ();
pub type CommitLedgerResult = Option<LedgerInfoWithSignatures>;
pub type PostCommitResult = ();

#[derive(Clone)]
pub struct PipelineFutures {
    pub prepare_fut: TaskFuture<PrepareResult>,
    pub rand_check_fut: TaskFuture<RandResult>,
    pub execute_fut: TaskFuture<ExecuteResult>,
    pub ledger_update_fut: TaskFuture<LedgerUpdateResult>,
    pub post_ledger_update_fut: TaskFuture<PostLedgerUpdateResult>,
    pub commit_vote_fut: TaskFuture<CommitVoteResult>,
    pub pre_commit_fut: TaskFuture<PreCommitResult>,
    pub notify_state_sync_fut: TaskFuture<NotifyStateSyncResult>,
    pub commit_ledger_fut: TaskFuture<CommitLedgerResult>,
    pub post_commit_fut: TaskFuture<PostCommitResult>,
}

impl PipelineFutures {
    // Wait for futures involved executor/state sync to complete
    pub async fn wait_until_finishes(self) {
        let _ = join5(
            self.execute_fut,
            self.ledger_update_fut,
            self.pre_commit_fut,
            self.commit_ledger_fut,
            self.notify_state_sync_fut,
        )
        .await;
    }
}

pub struct PipelineInputTx {
    pub qc_tx: Option<oneshot::Sender<Arc<QuorumCert>>>,
    pub rand_tx: Option<oneshot::Sender<Option<Randomness>>>,
    pub order_vote_tx: Option<oneshot::Sender<()>>,
    pub order_proof_tx: Option<oneshot::Sender<WrappedLedgerInfo>>,
    pub commit_proof_tx: Option<oneshot::Sender<LedgerInfoWithSignatures>>,
}

pub struct PipelineInputRx {
    pub qc_rx: oneshot::Receiver<Arc<QuorumCert>>,
    pub rand_rx: oneshot::Receiver<Option<Randomness>>,
    pub order_vote_rx: oneshot::Receiver<()>,
    pub order_proof_fut: TaskFuture<WrappedLedgerInfo>,
    pub commit_proof_fut: TaskFuture<LedgerInfoWithSignatures>,
}

/// A window of blocks that are needed for execution with the execution pool, EXCLUDING the current block
#[derive(Clone)]
pub struct OrderedBlockWindow {
    /// `block_id` (HashValue) helps with logging in the unlikely case there are issues upgrading
    /// the `Weak` pointer (we can use `block_id`)
    blocks: Vec<(HashValue, Weak<PipelinedBlock>)>,
}

impl OrderedBlockWindow {
    pub fn new(blocks: Vec<Arc<PipelinedBlock>>) -> Self {
        Self {
            blocks: blocks
                .iter()
                .map(|x| (x.id(), Arc::downgrade(x)))
                .collect::<Vec<(HashValue, Weak<PipelinedBlock>)>>(),
        }
    }

    pub fn empty() -> Self {
        Self { blocks: vec![] }
    }

    /// The blocks stored in `OrderedBlockWindow` use [`Weak`](Weak) pointers
    ///
    /// if the `PipelinedBlock` still exists
    ///      `upgraded_block` will be `Some(PipelinedBlock)`, and included in `blocks`
    /// else it will panic
    pub fn blocks(&self) -> Vec<Block> {
        let mut blocks: Vec<Block> = vec![];
        for (block_id, block) in self.blocks.iter() {
            let upgraded_block = block.upgrade();
            if let Some(block) = upgraded_block {
                blocks.push(block.block().clone())
            } else {
                panic!(
                    "Block with id: {} not found during upgrade in OrderedBlockWindow::blocks()",
                    block_id
                )
            }
        }
        blocks
    }

    pub fn pipelined_blocks(&self) -> Vec<Arc<PipelinedBlock>> {
        let mut blocks: Vec<Arc<PipelinedBlock>> = Vec::new();
        for (block_id, block) in self.blocks.iter() {
            if let Some(block) = block.upgrade() {
                blocks.push(block);
            } else {
                panic!(
                    "Block with id: {} not found during upgrade in OrderedBlockWindow::pipelined_blocks()",
                    block_id
                )
            }
        }
        blocks
    }
}

/// A representation of a block that has been added to the execution pipeline. It might either be in ordered
/// or in executed state. In the ordered state, the block is waiting to be executed. In the executed state,
/// the block has been executed and the output is available.
/// This struct is not Cloneable, use Arc to share it.
#[derive(Derivative)]
pub struct PipelinedBlock {
    /// Block data that cannot be regenerated.
    block: Block,
    /// A window of blocks that are needed for execution with the execution pool, EXCLUDING the current block
    block_window: OrderedBlockWindow,
    /// Input transactions in the order of execution. DEPRECATED stay for serialization compatibility.
    input_transactions: Vec<SignedTransaction>,
    /// The state_compute_result is calculated for all the pending blocks prior to insertion to
    /// the tree. The execution results are not persisted: they're recalculated again for the
    /// pending blocks upon restart.
    state_compute_result: Mutex<StateComputeResult>,
    randomness: OnceCell<Randomness>,
    pipeline_insertion_time: OnceCell<Instant>,
    execution_summary: OnceCell<ExecutionSummary>,
    /// pipeline related fields
    pipeline_futs: Mutex<Option<PipelineFutures>>,
    pipeline_tx: Mutex<Option<PipelineInputTx>>,
    pipeline_abort_handle: Mutex<Option<Vec<AbortHandle>>>,
    block_qc: Mutex<Option<Arc<QuorumCert>>>,
}

impl PartialEq for PipelinedBlock {
    fn eq(&self, other: &Self) -> bool {
        self.block == other.block
            && self.input_transactions == other.input_transactions
            && self.randomness.get() == other.randomness.get()
    }
}
impl Eq for PipelinedBlock {}

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
            randomness: Option<&'a Randomness>,
        }

        let serialized = SerializedBlock {
            block: &self.block,
            input_transactions: &self.input_transactions,
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
            randomness: Option<Randomness>,
        }

        let SerializedBlock {
            block,
            input_transactions,
            randomness,
        } = SerializedBlock::deserialize(deserializer)?;
        let block = PipelinedBlock::new(block, input_transactions, StateComputeResult::new_dummy());
        if let Some(r) = randomness {
            block.set_randomness(r);
        }
        Ok(block)
    }
}

impl PipelinedBlock {
    pub fn set_compute_result(
        &self,
        state_compute_result: StateComputeResult,
        execution_time: Duration,
    ) {
        let mut to_commit = 0;
        let mut to_retry = 0;
        for txn in state_compute_result.compute_status_for_input_txns() {
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
            root_hash: state_compute_result.root_hash(),
            gas_used: state_compute_result
                .execution_output
                .block_end_info
                .as_ref()
                .map(|info| info.block_effective_gas_units()),
        };
        *self.state_compute_result.lock() = state_compute_result;

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
    }

    pub fn set_randomness(&self, randomness: Randomness) {
        assert!(self.randomness.set(randomness.clone()).is_ok());
    }

    pub fn set_insertion_time(&self) {
        assert!(self.pipeline_insertion_time.set(Instant::now()).is_ok());
    }

    pub fn set_qc(&self, qc: Arc<QuorumCert>) {
        *self.block_qc.lock() = Some(qc.clone());
        if let Some(tx) = self.pipeline_tx().lock().as_mut() {
            tx.qc_tx.take().map(|tx| tx.send(qc));
        }
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

/// Safeguard to ensure that the pipeline is aborted when the block is dropped.
impl Drop for PipelinedBlock {
    fn drop(&mut self) {
        let _ = self.abort_pipeline();
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
            block_window: OrderedBlockWindow::empty(),
            input_transactions,
            state_compute_result: Mutex::new(state_compute_result),
            randomness: OnceCell::new(),
            pipeline_insertion_time: OnceCell::new(),
            execution_summary: OnceCell::new(),
            pipeline_futs: Mutex::new(None),
            pipeline_tx: Mutex::new(None),
            pipeline_abort_handle: Mutex::new(None),
            block_qc: Mutex::new(None),
        }
    }

    pub fn with_block_window(self, window: OrderedBlockWindow) -> Self {
        let mut block = self;
        block.block_window = window;
        block
    }

    pub fn new_ordered(block: Block, window: OrderedBlockWindow) -> Self {
        let input_transactions = Vec::new();
        let state_compute_result = StateComputeResult::new_dummy();
        Self::new(block, input_transactions, state_compute_result).with_block_window(window)
    }

    pub fn block(&self) -> &Block {
        &self.block
    }

    pub fn block_window(&self) -> &OrderedBlockWindow {
        &self.block_window
    }

    pub fn id(&self) -> HashValue {
        self.block().id()
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

    pub fn compute_result(&self) -> StateComputeResult {
        self.state_compute_result.lock().clone()
    }

    pub fn randomness(&self) -> Option<&Randomness> {
        self.randomness.get()
    }

    pub fn has_randomness(&self) -> bool {
        self.randomness.get().is_some()
    }

    pub fn block_info(&self) -> BlockInfo {
        let compute_result = self.compute_result();
        self.block().gen_block_info(
            compute_result.root_hash(),
            compute_result.last_version_or_0(),
            compute_result.epoch_state().clone(),
        )
    }

    pub fn vote_proposal(&self) -> VoteProposal {
        let compute_result = self.compute_result();
        VoteProposal::new(
            compute_result.extension_proof(),
            self.block.clone(),
            compute_result.epoch_state().clone(),
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
        self.compute_result().subscribable_events().to_vec()
    }

    /// The block is suffix of a reconfiguration block if the state result carries over the epoch state
    /// from parent but has no transaction.
    pub fn is_reconfiguration_suffix(&self) -> bool {
        let state_compute_result = self.compute_result();
        state_compute_result.has_reconfiguration()
            && state_compute_result
                .compute_status_for_input_txns()
                .is_empty()
    }

    pub fn elapsed_in_pipeline(&self) -> Option<Duration> {
        self.pipeline_insertion_time.get().map(|t| t.elapsed())
    }

    pub fn get_execution_summary(&self) -> Option<ExecutionSummary> {
        self.execution_summary.get().cloned()
    }

    pub fn qc(&self) -> Option<Arc<QuorumCert>> {
        self.block_qc.lock().clone()
    }
}

/// Pipeline related functions
impl PipelinedBlock {
    pub fn pipeline_futs(&self) -> Option<PipelineFutures> {
        self.pipeline_futs.lock().clone()
    }

    pub fn set_pipeline_futs(&self, pipeline_futures: PipelineFutures) {
        *self.pipeline_futs.lock() = Some(pipeline_futures);
    }

    pub fn set_pipeline_tx(&self, pipeline_tx: PipelineInputTx) {
        *self.pipeline_tx.lock() = Some(pipeline_tx);
    }

    pub fn set_pipeline_abort_handles(&self, abort_handles: Vec<AbortHandle>) {
        *self.pipeline_abort_handle.lock() = Some(abort_handles);
    }

    pub fn pipeline_tx(&self) -> &Mutex<Option<PipelineInputTx>> {
        &self.pipeline_tx
    }

    pub fn abort_pipeline(&self) -> Option<PipelineFutures> {
        if let Some(abort_handles) = self.pipeline_abort_handle.lock().take() {
            let mut aborted = false;
            for handle in abort_handles {
                if !handle.is_finished() {
                    handle.abort();
                    aborted = true;
                }
            }
            if aborted {
                info!(
                    "[Pipeline] Aborting pipeline for block {} {} {}",
                    self.id(),
                    self.epoch(),
                    self.round()
                );
            }
        }
        self.pipeline_futs.lock().take()
    }

    pub async fn wait_for_compute_result(&self) -> ExecutorResult<(StateComputeResult, Duration)> {
        self.pipeline_futs()
            .ok_or(ExecutorError::InternalError {
                error: "Pipeline aborted".to_string(),
            })?
            .ledger_update_fut
            .await
            .map(|(compute_result, execution_time, _)| (compute_result, execution_time))
            .map_err(|e| ExecutorError::InternalError {
                error: e.to_string(),
            })
    }

    pub async fn wait_for_commit_ledger(&self) {
        // may be aborted (e.g. by reset)
        if let Some(fut) = self.pipeline_futs() {
            // this may be cancelled
            let _ = fut.commit_ledger_fut.await;
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExecutionSummary {
    pub payload_len: u64,
    pub to_commit: u64,
    pub to_retry: u64,
    pub execution_time: Duration,
    pub root_hash: HashValue,
    pub gas_used: Option<u64>,
}
