// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

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
    monitor,
    network::NetworkSender,
    round_manager::VerifiedEvent,
    state_replication::StateComputerCommitCallBackType, logging::{LogEvent, LogSchema},
};
use aptos_consensus_types::{common::Author, executed_block::ExecutedBlock, experimental::{rand_decision::RandDecisions, rand_share::RandShares}};
use aptos_crypto::HashValue;
use aptos_logger::prelude::*;
use aptos_types::{
    account_address::AccountAddress, epoch_change::EpochChangeProof,
    ledger_info::LedgerInfoWithSignatures, validator_verifier::ValidatorVerifier,
};
use futures::{
    channel::{
        mpsc::{unbounded, UnboundedReceiver, UnboundedSender},
        oneshot,
    },
    FutureExt, SinkExt, StreamExt,
};
use once_cell::sync::OnceCell;
use std::{sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
}, collections::{HashMap, HashSet}};
use tokio::time::{Duration, Instant};

pub const COMMIT_VOTE_REBROADCAST_INTERVAL_MS: u64 = 1500;
pub const RAND_SHARE_REBROADCAST_INTERVAL_MS: u64 = 3000;
pub const RAND_DECISION_REBROADCAST_INTERVAL_MS: u64 = 3000;

pub const LOOP_INTERVAL_MS: u64 = 1500;

// If the randomness shares are broadcasted to all validators, or sent through the leaders
pub const BROADCAST_SHARES: bool = true;
// Number of leaders that aggregates the randomness shares
pub const NUM_LEADERS: usize = 1;

// Each validator will send a randomness share of size rand_size * rand_num / 100 (assuming 100 validators and even distribution)
pub const RAND_SIZE: usize = 96;
pub const RAND_NUM: usize = 1000;
pub const SHARE_SIZE: usize = RAND_SIZE * RAND_NUM / 100;
pub const DECISION_SIZE: usize = RAND_SIZE;

#[derive(Debug, Default)]
pub struct ResetAck {}

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


    // map from item_id to the authors of the randomness shares
    item_to_rand_authors_map: HashMap<HashValue, HashSet<Author>>,
    // map from item_id to the partially aggregated randomness
    item_to_rand_shares_map: HashMap<HashValue, HashMap<Author, RandShares>>,

    rand_msg_tx: NetworkSender,
    rand_msg_rx: aptos_channels::aptos_channel::Receiver<AccountAddress, VerifiedEvent>,

    // the roots point to the first *unprocessed* item that has randomness.
    // None means no items ready to be processed (either all processed or no item finishes previous stage)
    execution_root: BufferItemRootType,
    execution_phase_tx: Sender<CountedRequest<ExecutionRequest>>,
    execution_phase_rx: Receiver<ExecutionResponse>,

    signing_root: BufferItemRootType,
    signing_phase_tx: Sender<CountedRequest<SigningRequest>>,
    signing_phase_rx: Receiver<SigningResponse>,

    commit_msg_tx: NetworkSender,
    commit_msg_rx: aptos_channels::aptos_channel::Receiver<AccountAddress, VerifiedEvent>,

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
        rand_msg_tx: NetworkSender,
        rand_msg_rx: aptos_channels::aptos_channel::Receiver<AccountAddress, VerifiedEvent>,
        execution_phase_tx: Sender<CountedRequest<ExecutionRequest>>,
        execution_phase_rx: Receiver<ExecutionResponse>,
        signing_phase_tx: Sender<CountedRequest<SigningRequest>>,
        signing_phase_rx: Receiver<SigningResponse>,
        commit_msg_tx: NetworkSender,
        commit_msg_rx: aptos_channels::aptos_channel::Receiver<AccountAddress, VerifiedEvent>,
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
            item_to_rand_authors_map: HashMap::new(),
            item_to_rand_shares_map: HashMap::new(),

            rand_msg_tx,
            rand_msg_rx,

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
        spawn_named!("retry request", async move {
            tokio::time::sleep(duration).await;
            sender
                .send(request)
                .await
                .expect("Failed to send retry request");
        });
    }

    /// process incoming ordered blocks
    /// push them into the buffer and update the roots if they are none.
    async fn process_ordered_blocks(&mut self, blocks: OrderedBlocks) {
        let OrderedBlocks {
            ordered_blocks,
            ordered_proof,
            callback,
        } = blocks;

        info!(
            "Receive ordered block {}, the queue size is {}",
            ordered_proof.commit_info(),
            self.buffer.len() + 1,
        );
        let item = BufferItem::new_ordered(ordered_blocks.clone(), ordered_proof.clone(), callback);
        let item_hash = item.get_hash();

        // Disseminate the VRF shares for the ordered blocks
        // Happy path: each validator sends its VRF shares to the leader
        // Unhappy path: if the leader timeout, broadcast the randomness share

        // todo: if the unhappy path is too bad, can try multiple proposers or retry with more proposers


        // Send the randomness shares through the first k proposers,
        // otherwise all blocks are Nil/genesis blocks that do not need randomness
        let rand_shares = RandShares::new(item_hash, self.author, item.epoch(), item.gen_dummy_rand_share_vec(self.author));
        if BROADCAST_SHARES {
            info!(
                self.new_log(LogEvent::BCastRandToAll)
                    .item_id(item_hash),
                "item id {} broadcast to all, item size {}", item_hash, item.get_blocks().len()
            );
            self.rand_msg_tx
                    .broadcast_rand_shares(rand_shares)
                    .await;
        } else {
            let leaders = item.get_k_leaders(NUM_LEADERS, &self.verifier);
            if leaders.len() == 0 {
                self.buffer.push_back(item.try_advance_to_execution_ready());
                return;
            } else {
                for leader in leaders {
                    info!(
                        self.new_log(LogEvent::SendRandToLeader)
                            .remote_peer(leader)
                            .item_id(item_hash),
                        "item id {} send to leader {}, item size {}", item_hash, leader, item.get_blocks().len()
                    );
                    for block in item.get_blocks() {
                        observe_block(block.timestamp_usecs(), BlockStage::RAND_SENT);
                    }
                    println!("[rand debug] share {} sent by {} to leader {}", rand_shares.item_id(), self.author, leader);
                    self.rand_msg_tx
                    .send_rand_shares(rand_shares.clone(), leader)
                    .await;
                }
            }
        }

        self.buffer.push_back(item);
    }

    /// process the VRF share messages
    /// once receiving rand shares, aggregate to the existing ones
    /// once the randomness is aggregated
    /// it scans the whole buffer for a matching blockinfo
    /// if found, try adding randomness to the item
    async fn process_rand_message(&mut self, rand_msg: VerifiedEvent) -> bool {
        match rand_msg {
            VerifiedEvent::RandShareMsg(rand_shares) => {
                let item_id = rand_shares.item_id();
                info!("Receive random shares for item {:?}", item_id);

                info!(
                    self.new_log(LogEvent::LeaderReceiveRand)
                        .remote_peer(rand_shares.author())
                        .item_id(item_id),
                    "item id {} from node {}, rand size {}", item_id, rand_shares.author(), rand_shares.shares().len()
                );
                println!("[rand debug] share {} from {} by leader {}", rand_shares.item_id(), rand_shares.author(), self.author);

                // todo: verify rand message, ignore invalid ones

                let shares = self.item_to_rand_shares_map.entry(item_id).or_insert(HashMap::new());
                (*shares).insert(rand_shares.author(), *rand_shares.clone());

                let authors = self.item_to_rand_authors_map.entry(item_id).or_default();
                (*authors).insert(rand_shares.author());

                if self.verifier.check_voting_power(authors.iter()).is_ok() {
                    // enough randomness shares, can produce randomness for the block
                    let current_cursor = self.buffer.find_elem_by_key(*self.buffer.head_cursor(), item_id);
                    if current_cursor.is_some() {
                        let mut item = self.buffer.take(&current_cursor);
                        if item.is_ordered() && item.aggregate_rand_shares(self.item_to_rand_shares_map.get(&item_id).unwrap().clone()).is_ok() {
                            let rand_decisions = RandDecisions::new(item_id, rand_shares.epoch(), item.gen_dummy_rand_decision_vec());
                            item.update_rand_decisions(rand_decisions.clone());

                            if item.get_blocks().len() != rand_decisions.decisions().len() {
                                println!("unequal length on generated rand {} != {}", item.get_blocks().len(), rand_decisions.decisions().len());
                            }

                            if !BROADCAST_SHARES {
                                // if we're one of the proposer for the first k proposal block,
                                // we're responsible to broadcast the randomness decision
                                if item.get_k_leaders(NUM_LEADERS, &self.verifier).contains(&self.author) {
                                    info!(
                                        self.new_log(LogEvent::LeaderBCastRand).item_id(item_id),
                                        "item id {} broadcast by {}, item size {}", item_id, self.author, item.get_blocks().len()
                                    );
                                    for block in item.get_blocks() {
                                        observe_block(block.timestamp_usecs(), BlockStage::RAND_AGGREGATED);
                                    }

                                    // let peers = self.verifier.get_ordered_account_addresses();
                                    // let retry_interval = Duration::from_millis(500 as u64);
                                    // let rpc_timeout = Duration::from_millis(1000 as u64);
                                    // let rand_msg_tx = self.rand_msg_tx.clone();
                                    // let author = self.author;

                                    // tokio::spawn(async move {
                                    //     monitor!("batch_request", {
                                    //         let mut interval = time::interval(retry_interval);
                                    //         let mut futures = FuturesUnordered::new();
                                    //         for peer in peers {
                                    //             if peer == author {
                                    //                 continue;
                                    //             }
                                    //             info!("leader send {} item to {}", item_id, peer);
                                    //             futures.push(rand_msg_tx.rpc_send_rand_decisions(rand_decisions.clone(), peer, rpc_timeout));
                                    //         }
                                    //         loop {
                                    //             tokio::select! {
                                    //                 _ = interval.tick() => {
                                    //                     if futures.is_empty() {
                                    //                         // end the loop when the futures are drained
                                    //                         break;
                                    //                     }
                                    //                 }
                                    //                 Some(response) = futures.next() => {
                                    //                     if let Err(peer) = response {
                                    //                         info!("leader resend {} item to {}", item_id, peer);
                                    //                         futures.push(rand_msg_tx.rpc_send_rand_decisions(rand_decisions.clone(), peer, rpc_timeout));
                                    //                     }
                                    //                 },
                                    //             }
                                    //         }
                                    //     })
                                    // });

                                    println!("[rand debug] share {} decision broadcasted by leader {}", rand_shares.item_id(), self.author);

                                    self.rand_msg_tx
                                        .broadcast_rand_decisions(rand_decisions)
                                        .await;
                                }
                            }

                            self.buffer.set(&current_cursor, item.try_advance_to_execution_ready());
                            return true;
                        }
                        self.buffer.set(&current_cursor, item);
                    }
                }
            },
            VerifiedEvent::RandDecisionMsg(rand_decisions) => {
                let item_id = rand_decisions.item_id();
                // info!("Receive random decision for item {:?}", item_id);
                println!("[rand debug] share {} decision received at node {}", rand_decisions.item_id(), self.author);

                info!(
                    self.new_log(LogEvent::ReceiveRand).item_id(item_id),
                    "item id {}, item size {}", item_id, rand_decisions.decisions().len()
                );

                // todo: verify rand message, ignore invalid ones

                let current_cursor = self.buffer.find_elem_by_key(*self.buffer.head_cursor(), item_id);
                if current_cursor.is_some() {
                    let mut item = self.buffer.take(&current_cursor);
                    if item.is_ordered() {
                        for block in item.get_blocks() {
                            observe_block(block.timestamp_usecs(), BlockStage::RAND_RECEIVED);
                        }

                        // add the randomness to block
                        item.update_rand_decisions(*rand_decisions.clone());
                        if item.get_blocks().len() != rand_decisions.decisions().len() {
                            println!("unequal length on received rand {} != {}", item.get_blocks().len(), rand_decisions.decisions().len());
                        }
                        self.buffer.set(&current_cursor, item.try_advance_to_execution_ready());
                        return true;
                    }
                    self.buffer.set(&current_cursor, item);
                }
            },
            _ => {
                unreachable!("[Buffer Manager] Processing randomness message of wrong format");
            },
        }
        false
    }

    fn print_blocks(&mut self, blocks: Vec<ExecutedBlock>) {
        for block in blocks {
            print!(" {} ", block.id());
        }
        println!();
    }

    // helper function to prints the buffer_manager
    fn print_buffer(&mut self) {
        let mut current = *self.buffer.head_cursor();
        println!("================= start printing current buffer =================");
        while current.is_some() {
            match self.buffer.get(&current) {
                BufferItem::Ordered(item) => { print!("Ordered "); self.print_blocks(item.ordered_blocks.clone()); }
                BufferItem::ExecutionReady(item) => { print!("ExecutionReady "); self.print_blocks(item.ordered_blocks.clone()); }
                BufferItem::Executed(item) => { print!("Executed "); self.print_blocks(item.executed_blocks.clone()); }
                BufferItem::Signed(item) => { print!("Signed "); self.print_blocks(item.executed_blocks.clone()); }
                BufferItem::Aggregated(item) => { print!("Aggregated "); self.print_blocks(item.executed_blocks.clone()); }
            }
            current = self.buffer.get_next(&current);
        }
        println!("================= end printing current buffer =================");
        println!();
    }

    /// Set the execution root to the first execution ready item (but no ordered item before it) and send execution request
    /// Set to None if not exist
    async fn advance_execution_root(&mut self) {
        let cursor = self.execution_root;
        self.execution_root = self
            .buffer
            .find_elem_from_with_prefix(cursor.or_else(|| *self.buffer.head_cursor()), |item| {
                item.is_execution_ready()
            }, |item| {
                item.is_ordered()
            }
        );
        println!(
            "Advance execution root from {:?} to {:?}",
            cursor, self.execution_root
        );
        // self.print_buffer();

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
        println!(
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
                println!("Advance head to {:?}", self.buffer.head_cursor());
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
        tx.send(ResetAck::default()).unwrap();
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
            },
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
            println!("aggregated advance head!");
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
            },
        };
        println!(
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
                let author = vote.author();
                let commit_info = vote.commit_info().clone();
                println!("Receive commit vote {} from {}", commit_info, author);
                let target_block_id = vote.commit_info().id();
                let current_cursor = self
                    .buffer
                    .find_elem_by_key(*self.buffer.head_cursor(), target_block_id);
                if current_cursor.is_some() {
                    let mut item = self.buffer.take(&current_cursor);
                    let new_item = match item.add_signature_if_matched(*vote) {
                        Ok(()) => item.try_advance_to_aggregated(&self.verifier),
                        Err(e) => {
                            error!(
                                error = ?e,
                                author = author,
                                commit_info = commit_info,
                                "Failed to add commit vote",
                            );
                            item
                        },
                    };
                    self.buffer.set(&current_cursor, new_item);
                    if self.buffer.get(&current_cursor).is_aggregated() {
                        return Some(target_block_id);
                    }
                }
            },
            VerifiedEvent::CommitDecision(commit_proof) => {
                let target_block_id = commit_proof.ledger_info().commit_info().id();
                println!(
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
            },
            _ => {
                unreachable!("[Buffer Manager] Processing commit message of wrong format");
            },
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
        // println!("rebroadcast_commit_votes_if_needed");
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

    async fn rebroadcast_rand_share_if_needed(&mut self) {
        if self.previous_commit_time.elapsed()
            < Duration::from_millis(RAND_SHARE_REBROADCAST_INTERVAL_MS)
        {
            return;
        }
        let mut cursor = *self.buffer.head_cursor();
        let mut count = 0;

        while cursor.is_some() {
            let item = self.buffer.get(&cursor);
            if !item.is_ordered() {
                cursor = self.buffer.get_next(&cursor);
                continue;
            }
            println!("rebroadcast_rand_share_if_needed for item {:?}", cursor);

            let rand_shares = RandShares::new(cursor.unwrap(), self.author, item.epoch(), item.gen_dummy_rand_share_vec(self.author));

            self.rand_msg_tx
                .broadcast_rand_shares(rand_shares)
                .await;
            count += 1;
            cursor = self.buffer.get_next(&cursor);
        }
        if count > 0 {
            info!("Rebroadcasting {} randomness shares", count);
        }
    }

    async fn rebroadcast_rand_decision_if_needed(&mut self) {
        if self.previous_commit_time.elapsed()
            < Duration::from_millis(RAND_DECISION_REBROADCAST_INTERVAL_MS)
        {
            return;
        }

        // Need to rebroadcast randomness decisions for any non-committed execution ready block
        let mut cursor = *self.buffer.head_cursor();
        let mut count = 0;

        while cursor.is_some() {
            let item = self.buffer.get(&cursor);
            // ordered item or NIL blocks
            if item.is_ordered() || item.get_k_leaders(1, &self.verifier).is_empty() {
                cursor = self.buffer.get_next(&cursor);
                continue;
            }
            println!("rebroadcast_rand_decision_if_needed for item {:?}", cursor);
            let rand_decision = RandDecisions::new(cursor.unwrap(), item.epoch(), item.gen_dummy_rand_decision_vec());

            self.rand_msg_tx
                .broadcast_rand_decisions(rand_decision)
                .await;
            count += 1;
            cursor = self.buffer.get_next(&cursor);
        }
        if count > 0 {
            info!("Rebroadcasting {} randomness decisions", count);
        }
    }

    fn update_buffer_manager_metrics(&self) {
        let mut cursor = *self.buffer.head_cursor();
        let mut pending_ordered = 0;
        let mut pending_execution_ready = 0;
        let mut pending_executed = 0;
        let mut pending_signed = 0;
        let mut pending_aggregated = 0;

        while cursor.is_some() {
            match self.buffer.get(&cursor) {
                BufferItem::Ordered(_) => {
                    pending_ordered += 1;
                },
                BufferItem::ExecutionReady(_) => {
                    pending_execution_ready += 1;
                }
                BufferItem::Executed(_) => {
                    pending_executed += 1;
                },
                BufferItem::Signed(_) => {
                    pending_signed += 1;
                },
                BufferItem::Aggregated(_) => {
                    pending_aggregated += 1;
                },
            }
            cursor = self.buffer.get_next(&cursor);
        }

        counters::NUM_BLOCKS_IN_PIPELINE
            .with_label_values(&["ordered"])
            .set(pending_ordered as i64);
        counters::NUM_BLOCKS_IN_PIPELINE
            .with_label_values(&["execution_ready"])
            .set(pending_execution_ready as i64);
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

    fn new_log(&self, event: LogEvent) -> LogSchema {
        LogSchema::new(event)
    }

    pub async fn start(mut self) {
        info!("Buffer manager starts.");
        let mut interval = tokio::time::interval(Duration::from_millis(LOOP_INTERVAL_MS));
        while !self.stop {
            // advancing the root will trigger sending requests to the pipeline
            ::futures::select! {
                blocks = self.block_rx.select_next_some() => {
                    monitor!("buffer_manager_process_ordered", {
                        self.process_ordered_blocks(blocks).await;
                    });
                },
                rand_msg = self.rand_msg_rx.select_next_some() => {
                    monitor!("buffer_manager_process_rand", {
                        self.process_rand_message(rand_msg).await;
                        if self.execution_root.is_none() {
                            self.advance_execution_root().await;
                        }
                    });
                },
                reset_event = self.reset_rx.select_next_some() => {
                    monitor!("buffer_manager_process_reset",
                    self.process_reset_request(reset_event).await);
                },
                response = self.execution_phase_rx.select_next_some() => {
                    monitor!("buffer_manager_process_execution_response", {
                    self.process_execution_response(response).await;
                    self.advance_execution_root().await;
                    if self.signing_root.is_none() {
                        self.advance_signing_root().await;
                    }});
                },
                response = self.signing_phase_rx.select_next_some() => {
                    monitor!("buffer_manager_process_signing_response", {
                    self.process_signing_response(response).await;
                    self.advance_signing_root().await
                    })
                },
                commit_msg = self.commit_msg_rx.select_next_some() => {
                    monitor!("buffer_manager_process_commit_message",
                    if let Some(aggregated_block_id) = self.process_commit_message(commit_msg) {
                        self.advance_head(aggregated_block_id).await;
                        if self.execution_root.is_none() {
                            self.advance_execution_root().await;
                        }
                        if self.signing_root.is_none() {
                            self.advance_signing_root().await;
                        }
                    });
                },
                _ = interval.tick().fuse() => {
                    self.print_buffer();
                    self.update_buffer_manager_metrics();
                    monitor!("buffer_manager_process_rebroadcast_commit_vote", {
                    self.rebroadcast_commit_votes_if_needed().await
                    });
                    // unhappy path, keep broadcasting randomness decisions or randomness shares for non-committed blocks
                    monitor!("buffer_manager_process_rebroadcast_rand_share", {
                    self.rebroadcast_rand_share_if_needed().await;
                    });
                    monitor!("buffer_manager_process_rebroadcast_rand_decision", {
                    self.rebroadcast_rand_decision_if_needed().await;
                    });
                },
                // no else branch here because interval.tick will always be available
            }
        }
        info!("Buffer manager stops.");
    }
}
