// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use futures::{
    channel::{
        mpsc::{UnboundedReceiver, UnboundedSender},
        oneshot,
    },
    FutureExt, SinkExt, StreamExt,
};
use tokio::time::{Duration, Instant};

use aptos_logger::prelude::*;
use aptos_types::{
    account_address::AccountAddress, ledger_info::LedgerInfoWithSignatures,
    validator_verifier::ValidatorVerifier,
};
use consensus_types::{common::Author, executed_block::ExecutedBlock};

use crate::{
    block_storage::tracing::{observe_block, BlockStage},
    counters,
    experimental::{
        buffer::{Buffer, Cursor},
        buffer_item::BufferItem,
        execution_phase::{ExecutionRequest, ExecutionResponse},
        persisting_phase::PersistingRequest,
        pipeline_phase::CountedRequest,
        signing_phase::{SigningRequest, SigningResponse},
    },
    network::NetworkSender,
    round_manager::VerifiedEvent,
    state_replication::StateComputerCommitCallBackType,
};
use aptos_crypto::HashValue;
use aptos_types::epoch_change::EpochChangeProof;
use futures::channel::mpsc::unbounded;
use once_cell::sync::OnceCell;

pub const COMMIT_VOTE_REBROADCAST_INTERVAL_MS: u64 = 1500;
pub const LOOP_INTERVAL_MS: u64 = 1500;

pub type ResetAck = ();

pub fn sync_ack_new() -> ResetAck {}

pub struct ResetRequest {
    pub tx: oneshot::Sender<ResetAck>,
    pub stop: bool,
}

pub struct OrderedBlocks {
    pub ordered_blocks: Vec<ExecutedBlock>,
    pub ordered_proof: LedgerInfoWithSignatures,
    pub callback: StateComputerCommitCallBackType,
}

pub type BufferItemRootType = Cursor;
pub type Sender<T> = UnboundedSender<T>;
pub type Receiver<T> = UnboundedReceiver<T>;

pub fn create_channel<T>() -> (Sender<T>, Receiver<T>) {
    unbounded::<T>()
}

/// BufferManager handles the states of ordered blocks and
/// interacts with the execution phase, the signing phase, and
/// the persisting phase.
pub struct BufferManager {
    author: Author,

    buffer: Buffer<BufferItem>,

    // the roots point to the first *unprocessed* item.
    // None means no items ready to be processed (either all processed or no item finishes previous stage)
    execution_root: BufferItemRootType,
    execution_phase_tx: Sender<CountedRequest<ExecutionRequest>>,
    execution_phase_rx: Receiver<ExecutionResponse>,

    signing_root: BufferItemRootType,
    signing_phase_tx: Sender<CountedRequest<SigningRequest>>,
    signing_phase_rx: Receiver<SigningResponse>,

    commit_msg_tx: NetworkSender,
    commit_msg_rx: channel::aptos_channel::Receiver<AccountAddress, VerifiedEvent>,

    // we don't hear back from the persisting phase
    persisting_phase_tx: Sender<CountedRequest<PersistingRequest>>,

    block_rx: UnboundedReceiver<OrderedBlocks>,
    reset_rx: UnboundedReceiver<ResetRequest>,
    stop: bool,

    verifier: ValidatorVerifier,

    ongoing_tasks: Arc<AtomicU64>,
    // Since proposal_generator is not aware of reconfiguration any more, the suffix blocks
    // will not have the same timestamp as the reconfig block which violates the invariant
    // that block.timestamp == state.timestamp because no txn is executed in suffix blocks.
    // We change the timestamp field of the block info to maintain the invariant.
    // If the executed blocks are b1 <- b2 <- r <- b4 <- b5 with timestamp t1..t5
    // we replace t5 with t3 (from reconfiguration block) since that's the last timestamp
    // being updated on-chain.
    end_epoch_timestamp: OnceCell<u64>,
    previous_commit_time: Instant,
}

impl BufferManager {
    pub fn new(
        author: Author,
        execution_phase_tx: Sender<CountedRequest<ExecutionRequest>>,
        execution_phase_rx: Receiver<ExecutionResponse>,
        signing_phase_tx: Sender<CountedRequest<SigningRequest>>,
        signing_phase_rx: Receiver<SigningResponse>,
        commit_msg_tx: NetworkSender,
        commit_msg_rx: channel::aptos_channel::Receiver<AccountAddress, VerifiedEvent>,
        persisting_phase_tx: Sender<CountedRequest<PersistingRequest>>,
        block_rx: UnboundedReceiver<OrderedBlocks>,
        reset_rx: UnboundedReceiver<ResetRequest>,
        verifier: ValidatorVerifier,
        ongoing_tasks: Arc<AtomicU64>,
    ) -> Self {
        let buffer = Buffer::<BufferItem>::new();

        Self {
            author,

            buffer,

            execution_root: None,
            execution_phase_tx,
            execution_phase_rx,

            signing_root: None,
            signing_phase_tx,
            signing_phase_rx,

            commit_msg_tx,
            commit_msg_rx,

            persisting_phase_tx,

            block_rx,
            reset_rx,
            stop: false,

            verifier,
            ongoing_tasks,
            end_epoch_timestamp: OnceCell::new(),
            previous_commit_time: Instant::now(),
        }
    }

    fn create_new_request<Request>(&self, req: Request) -> CountedRequest<Request> {
        CountedRequest::new(req, self.ongoing_tasks.clone())
    }

    fn spawn_retry_request<T: Send + 'static>(
        mut sender: Sender<T>,
        request: T,
        duration: Duration,
    ) {
        counters::BUFFER_MANAGER_RETRY_COUNT.inc();
        spawn_named!(&"retry request", async move {
            tokio::time::sleep(duration).await;
            sender
                .send(request)
                .await
                .expect("Failed to send retry request");
        });
    }

    /// process incoming ordered blocks
    /// push them into the buffer and update the roots if they are none.
    fn process_ordered_blocks(&mut self, ordered_blocks: OrderedBlocks) {
        let OrderedBlocks {
            ordered_blocks,
            ordered_proof,
            callback,
        } = ordered_blocks;

        info!(
            "Receive ordered block {}, the queue size is {}",
            ordered_proof.commit_info(),
            self.buffer.len() + 1,
        );
        let item = BufferItem::new_ordered(ordered_blocks, ordered_proof, callback);
        self.buffer.push_back(item);
    }

    /// Set the execution root to the first not executed item (Ordered) and send execution request
    /// Set to None if not exist
    async fn advance_execution_root(&mut self) {
        let cursor = self.execution_root;
        self.execution_root = self
            .buffer
            .find_elem_from(cursor.or_else(|| *self.buffer.head_cursor()), |item| {
                item.is_ordered()
            });
        info!(
            "Advance execution root from {:?} to {:?}",
            cursor, self.execution_root
        );
        if self.execution_root.is_some() {
            let ordered_blocks = self.buffer.get(&self.execution_root).get_blocks().clone();
            let request = self.create_new_request(ExecutionRequest { ordered_blocks });
            if cursor == self.execution_root {
                let sender = self.execution_phase_tx.clone();
                Self::spawn_retry_request(sender, request, Duration::from_millis(100));
            } else {
                self.execution_phase_tx
                    .send(request)
                    .await
                    .expect("Failed to send execution request")
            }
        }
    }

    /// Set the signing root to the first not signed item (Executed) and send execution request
    /// Set to None if not exist
    async fn advance_signing_root(&mut self) {
        let cursor = self.signing_root;
        self.signing_root = self
            .buffer
            .find_elem_from(cursor.or_else(|| *self.buffer.head_cursor()), |item| {
                item.is_executed()
            });
        info!(
            "Advance signing root from {:?} to {:?}",
            cursor, self.signing_root
        );
        if self.signing_root.is_some() {
            let item = self.buffer.get(&self.signing_root);
            let executed_item = item.unwrap_executed_ref();
            let request = self.create_new_request(SigningRequest {
                ordered_ledger_info: executed_item.ordered_proof.clone(),
                commit_ledger_info: executed_item.partial_commit_proof.ledger_info().clone(),
            });
            if cursor == self.signing_root {
                let sender = self.signing_phase_tx.clone();
                Self::spawn_retry_request(sender, request, Duration::from_millis(100));
            } else {
                self.signing_phase_tx
                    .send(request)
                    .await
                    .expect("Failed to send signing request");
            }
        }
    }

    /// Pop the prefix of buffer items until (including) target_block_id
    /// Send persist request.
    async fn advance_head(&mut self, target_block_id: HashValue) {
        let mut blocks_to_persist: Vec<Arc<ExecutedBlock>> = vec![];

        while let Some(item) = self.buffer.pop_front() {
            blocks_to_persist.extend(
                item.get_blocks()
                    .iter()
                    .map(|eb| Arc::new(eb.clone()))
                    .collect::<Vec<Arc<ExecutedBlock>>>(),
            );
            if self.signing_root == Some(item.block_id()) {
                self.signing_root = None;
            }
            if self.execution_root == Some(item.block_id()) {
                self.execution_root = None;
            }
            if item.block_id() == target_block_id {
                let aggregated_item = item.unwrap_aggregated();
                let block = aggregated_item.executed_blocks.last().unwrap().block();
                observe_block(block.timestamp_usecs(), BlockStage::COMMIT_CERTIFIED);
                // if we're the proposer for the block, we're responsible to broadcast the commit decision.
                if block.author() == Some(self.author) {
                    self.commit_msg_tx
                        .broadcast_commit_proof(aggregated_item.commit_proof.clone())
                        .await;
                }
                if aggregated_item.commit_proof.ledger_info().ends_epoch() {
                    self.commit_msg_tx
                        .send_epoch_change(EpochChangeProof::new(
                            vec![aggregated_item.commit_proof.clone()],
                            false,
                        ))
                        .await;
                    // the epoch ends, reset to avoid executing more blocks, execute after
                    // this persisting request will result in BlockNotFound
                    self.reset().await;
                }
                self.persisting_phase_tx
                    .send(self.create_new_request(PersistingRequest {
                        blocks: blocks_to_persist,
                        commit_ledger_info: aggregated_item.commit_proof,
                        // we use the last callback
                        // this is okay because the callback function (from BlockStore::commit)
                        // takes in the actual blocks and ledger info from the state computer
                        // the encoded values are references to the block_tree, storage, and a commit root
                        // the block_tree and storage are the same for all the callbacks in the current epoch
                        // the commit root is used in logging only.
                        callback: aggregated_item.callback,
                    }))
                    .await
                    .expect("Failed to send persist request");
                info!("Advance head to {:?}", self.buffer.head_cursor());
                self.previous_commit_time = Instant::now();
                return;
            }
        }
        unreachable!("Aggregated item not found in the list");
    }

    /// Reset any request in buffer manager, this is important to avoid race condition with state sync.
    /// Internal requests are managed with ongoing_tasks.
    /// Incoming ordered blocks are pulled, it should only have existing blocks but no new blocks until reset finishes.
    async fn reset(&mut self) {
        self.buffer = Buffer::new();
        self.execution_root = None;
        self.signing_root = None;
        self.previous_commit_time = Instant::now();
        // purge the incoming blocks queue
        while let Ok(Some(_)) = self.block_rx.try_next() {}
        // Wait for ongoing tasks to finish before sending back ack.
        while self.ongoing_tasks.load(Ordering::SeqCst) > 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    /// It pops everything in the buffer and if reconfig flag is set, it stops the main loop
    async fn process_reset_request(&mut self, request: ResetRequest) {
        let ResetRequest { tx, stop } = request;
        info!("Receive reset");

        self.stop = stop;
        self.reset().await;
        tx.send(sync_ack_new()).unwrap();
        info!("Reset finishes");
    }

    /// If the response is successful, advance the item to Executed, otherwise panic (TODO fix).
    async fn process_execution_response(&mut self, response: ExecutionResponse) {
        let ExecutionResponse { block_id, inner } = response;
        // find the corresponding item, may not exist if a reset or aggregated happened
        let current_cursor = self.buffer.find_elem_by_key(self.execution_root, block_id);
        if current_cursor.is_none() {
            return;
        }

        let executed_blocks = match inner {
            Ok(result) => result,
            Err(e) => {
                error!("Execution error {:?}", e);
                return;
            }
        };
        info!(
            "Receive executed response {}",
            executed_blocks.last().unwrap().block_info()
        );

        // Handle reconfiguration timestamp reconciliation.
        // end epoch timestamp is set to the first block that causes the reconfiguration.
        // once it's set, any subsequent block commit info will be set to this timestamp.
        if self.end_epoch_timestamp.get().is_none() {
            let maybe_reconfig_timestamp = executed_blocks
                .iter()
                .find(|b| b.block_info().has_reconfiguration())
                .map(|b| b.timestamp_usecs());
            if let Some(timestamp) = maybe_reconfig_timestamp {
                debug!("Reconfig happens, set epoch end timestamp to {}", timestamp);
                self.end_epoch_timestamp
                    .set(timestamp)
                    .expect("epoch end timestamp should only be set once");
            }
        }

        let item = self.buffer.take(&current_cursor);
        let new_item = item.advance_to_executed_or_aggregated(
            executed_blocks,
            &self.verifier,
            self.end_epoch_timestamp.get().cloned(),
        );
        let aggregated = new_item.is_aggregated();
        self.buffer.set(&current_cursor, new_item);
        if aggregated {
            self.advance_head(block_id).await;
        }
    }

    /// If the signing response is successful, advance the item to Signed and broadcast commit votes.
    async fn process_signing_response(&mut self, response: SigningResponse) {
        let SigningResponse {
            signature_result,
            commit_ledger_info,
        } = response;
        let signature = match signature_result {
            Ok(sig) => sig,
            Err(e) => {
                error!("Signing failed {:?}", e);
                return;
            }
        };
        info!(
            "Receive signing response {}",
            commit_ledger_info.commit_info()
        );
        // find the corresponding item, may not exist if a reset or aggregated happened
        let current_cursor = self
            .buffer
            .find_elem_by_key(self.signing_root, commit_ledger_info.commit_info().id());
        if current_cursor.is_some() {
            let item = self.buffer.take(&current_cursor);
            // it is possible that we already signed this buffer item (double check after the final integration)
            if item.is_executed() {
                // we have found the buffer item
                let signed_item = item.advance_to_signed(self.author, signature);
                let maybe_proposer = signed_item
                    .unwrap_signed_ref()
                    .executed_blocks
                    .last()
                    .unwrap()
                    .block()
                    .author();
                let commit_vote = signed_item.unwrap_signed_ref().commit_vote.clone();

                self.buffer.set(&current_cursor, signed_item);
                if let Some(proposer) = maybe_proposer {
                    self.commit_msg_tx
                        .send_commit_vote(commit_vote, proposer)
                        .await;
                } else {
                    self.commit_msg_tx.broadcast_commit_vote(commit_vote).await;
                }
            } else {
                self.buffer.set(&current_cursor, item);
            }
        }
    }

    /// process the commit vote messages
    /// it scans the whole buffer for a matching blockinfo
    /// if found, try advancing the item to be aggregated
    fn process_commit_message(&mut self, commit_msg: VerifiedEvent) -> Option<HashValue> {
        match commit_msg {
            VerifiedEvent::CommitVote(vote) => {
                // find the corresponding item
                info!(
                    "Receive commit vote {} from {}",
                    vote.commit_info(),
                    vote.author()
                );
                let target_block_id = vote.commit_info().id();
                let current_cursor = self
                    .buffer
                    .find_elem_by_key(*self.buffer.head_cursor(), target_block_id);
                if current_cursor.is_some() {
                    let mut item = self.buffer.take(&current_cursor);
                    let new_item = match item.add_signature_if_matched(*vote) {
                        Ok(()) => item.try_advance_to_aggregated(&self.verifier),
                        Err(e) => {
                            error!("Failed to add commit vote {:?}", e);
                            item
                        }
                    };
                    self.buffer.set(&current_cursor, new_item);
                    if self.buffer.get(&current_cursor).is_aggregated() {
                        return Some(target_block_id);
                    }
                }
            }
            VerifiedEvent::CommitDecision(commit_proof) => {
                let target_block_id = commit_proof.ledger_info().commit_info().id();
                info!(
                    "Receive commit decision {}",
                    commit_proof.ledger_info().commit_info()
                );
                let cursor = self
                    .buffer
                    .find_elem_by_key(*self.buffer.head_cursor(), target_block_id);
                if cursor.is_some() {
                    let item = self.buffer.take(&cursor);
                    let new_item = item.try_advance_to_aggregated_with_ledger_info(
                        commit_proof.ledger_info().clone(),
                    );
                    let aggregated = new_item.is_aggregated();
                    self.buffer.set(&cursor, new_item);
                    if aggregated {
                        return Some(target_block_id);
                    }
                }
            }
            _ => {
                unreachable!();
            }
        }
        None
    }

    /// this function retries all the items until the signing root
    /// note that there might be other signed items after the signing root
    async fn rebroadcast_commit_votes_if_needed(&mut self) {
        if self.previous_commit_time.elapsed()
            < Duration::from_millis(COMMIT_VOTE_REBROADCAST_INTERVAL_MS)
        {
            return;
        }
        let mut cursor = *self.buffer.head_cursor();
        let mut count = 0;
        while cursor.is_some() {
            {
                let item = self.buffer.get(&cursor);
                if !item.is_signed() {
                    break;
                }
                let signed_item = item.unwrap_signed_ref();
                self.commit_msg_tx
                    .broadcast_commit_vote(signed_item.commit_vote.clone())
                    .await;
                count += 1;
            }
            cursor = self.buffer.get_next(&cursor);
        }
        if count > 0 {
            info!("Rebroadcasting {} commit votes", count);
        }
    }

    fn update_buffer_manager_metrics(&self) {
        let mut cursor = *self.buffer.head_cursor();
        let mut pending_ordered = 0;
        let mut pending_executed = 0;
        let mut pending_signed = 0;
        let mut pending_aggregated = 0;

        while cursor.is_some() {
            match self.buffer.get(&cursor) {
                BufferItem::Ordered(_) => {
                    pending_ordered += 1;
                }
                BufferItem::Executed(_) => {
                    pending_executed += 1;
                }
                BufferItem::Signed(_) => {
                    pending_signed += 1;
                }
                BufferItem::Aggregated(_) => {
                    pending_aggregated += 1;
                }
            }
            cursor = self.buffer.get_next(&cursor);
        }

        counters::NUM_BLOCKS_IN_PIPELINE
            .with_label_values(&["ordered"])
            .set(pending_ordered as i64);
        counters::NUM_BLOCKS_IN_PIPELINE
            .with_label_values(&["executed"])
            .set(pending_executed as i64);
        counters::NUM_BLOCKS_IN_PIPELINE
            .with_label_values(&["signed"])
            .set(pending_signed as i64);
        counters::NUM_BLOCKS_IN_PIPELINE
            .with_label_values(&["aggregated"])
            .set(pending_aggregated as i64);
    }

    pub async fn start(mut self) {
        info!("Buffer manager starts.");
        let mut interval = tokio::time::interval(Duration::from_millis(LOOP_INTERVAL_MS));
        while !self.stop {
            // advancing the root will trigger sending requests to the pipeline
            ::futures::select! {
                blocks = self.block_rx.select_next_some() => {
                    self.process_ordered_blocks(blocks);
                    if self.execution_root.is_none() {
                        self.advance_execution_root().await;
                    }
                },
                reset_event = self.reset_rx.select_next_some() => {
                    self.process_reset_request(reset_event).await;
                },
                response = self.execution_phase_rx.select_next_some() => {
                    self.process_execution_response(response).await;
                    self.advance_execution_root().await;
                    if self.signing_root.is_none() {
                        self.advance_signing_root().await;
                    }
                },
                response = self.signing_phase_rx.select_next_some() => {
                    self.process_signing_response(response).await;
                    self.advance_signing_root().await;
                },
                commit_msg = self.commit_msg_rx.select_next_some() => {
                    if let Some(aggregated_block_id) = self.process_commit_message(commit_msg) {
                        self.advance_head(aggregated_block_id).await;
                        if self.execution_root.is_none() {
                            self.advance_execution_root().await;
                        }
                        if self.signing_root.is_none() {
                            self.advance_signing_root().await;
                        }
                    }
                },
                _ = interval.tick().fuse() => {
                    self.update_buffer_manager_metrics();
                    self.rebroadcast_commit_votes_if_needed().await;
                },
                // no else branch here because interval.tick will always be available
            }
        }
        info!("Buffer manager stops.");
    }
}
