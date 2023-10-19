// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use crate::{
    block_storage::tracing::{observe_block, BlockStage},
    counters,
    experimental::{
        commit_reliable_broadcast::DropGuard,
        buffer_manager::{ResetRequest, ResetAck, create_channel},
    },
    monitor,
    network::{NetworkSender, IncomingRandRequest},
    network_interface::ConsensusMsg,
    randomness::{block_queue::BlockQueueItem, types::DeltaMsg}, logging::{LogEvent, LogSchema},
};
use anyhow::bail;
use aptos_consensus_types::common::{Author, Round};
use aptos_crypto::HashValue;
use aptos_dkg::weighted_vuf::traits::WeightedVUF;
use aptos_logger::prelude::*;
use aptos_network::protocols::network::RpcError;
use aptos_reliable_broadcast::{ReliableBroadcast, RBNetworkSender};
use aptos_time_service::TimeService;
use aptos_types::{
    account_address::AccountAddress, validator_verifier::ValidatorVerifier, randomness::{RandDecision, RandConfig, Randomness, Mode, RandMetadata, Delta, WVUF},
};
use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    future::{AbortHandle, Abortable},
    FutureExt, SinkExt, StreamExt,
};
use itertools::Itertools;
use tokio::time::{Duration, Instant};
use tokio_retry::strategy::ExponentialBackoff;

use super::{block_queue::{OrderedBlocks, RandReadyBlocks}, types::{RandMessage, RandShare, ShareAckState}, rand_store::{RandStore, AddDecisionResult}};

// rand todo: parameters. These parameters can sometimes pass smoke test.
pub const RAND_SHARE_BROADCAST_INTERVAL_MS: u64 = 1_000;
pub const RAND_SHARE_REBROADCAST_INTERVAL_MS: u64 = 3_000;
pub const REBROADCAST_LOOP_INTERVAL_MS: u64 = 1_000;
pub const GARBAGE_COLLECT_LOOP_INTERVAL_MS: u64 = 10_000;
pub const DELTA_SENDING_TIMEOUT: u64 = 3_000;

pub type Sender<T> = UnboundedSender<T>;
pub type Receiver<T> = UnboundedReceiver<T>;

pub enum BufferManagerEvent {
    Commit(Round),
}

pub fn log_rand_event(event: LogEvent, author: Author, remote_peer: Option<Author>, id: HashValue, round: Round) {
    let mut log = LogSchema::new(event).author(author).id(id).round(round);
    if let Some(peer) = remote_peer {
        log = log.remote_peer(peer);
    }
    info!(log);
}

pub struct RandManager {
    author: Author,
    epoch: u64,
    verifier: ValidatorVerifier,

    rand_store: RandStore,

    previous_dequeue_time: Instant,

    // channel for receiving ordered blocks from ordering state computer
    ordered_block_rx: Receiver<OrderedBlocks>,
    // channel for sending execution ready blocks to buffer manager
    rand_ready_block_tx: Sender<RandReadyBlocks>,
    // channel for receiving commit events from buffer manager
    buffer_manager_rx: Receiver<BufferManagerEvent>,
    // channel for receiving reset events from state sync
    reset_rx: Receiver<ResetRequest>,
    stop: bool,

    // channels for randomness messages
    network_sender: Arc<NetworkSender>,
    rand_msg_rx: aptos_channels::aptos_channel::Receiver<AccountAddress, IncomingRandRequest>,

    // local channels
    acked_rand_decision_tx: Sender<RandDecision>,
    acked_rand_decision_rx: Receiver<RandDecision>,
    send_delta_request_tx: Sender<Author>,
    send_delta_request_rx: Receiver<Author>,

    reliable_broadcast: Arc<ReliableBroadcast<RandMessage, ExponentialBackoff>>,
}

impl RandManager {
    pub fn new(
        author: Author,
        epoch: u64,
        verifier: ValidatorVerifier,
        rand_config: Option<RandConfig>,
        gc_gap_below: Round,
        gc_gap_above: Round,
        ordered_block_rx: UnboundedReceiver<OrderedBlocks>,
        rand_ready_block_tx: UnboundedSender<RandReadyBlocks>,
        buffer_manager_rx: Receiver<BufferManagerEvent>,
        reset_rx: UnboundedReceiver<ResetRequest>,
        network_sender: Arc<NetworkSender>,
        rand_msg_rx: aptos_channels::aptos_channel::Receiver<AccountAddress, IncomingRandRequest>,
    ) -> Self {
        let rand_store = RandStore::new(author, rand_config, gc_gap_below, gc_gap_above);

        // rand todo: parameters
        let rb_backoff_policy = ExponentialBackoff::from_millis(2).factor(50).max_delay(Duration::from_secs(5));
        let reliable_broadcast = ReliableBroadcast::new(
            verifier.get_ordered_account_addresses(),
            network_sender.clone(),
            rb_backoff_policy,
            TimeService::real(),
            Duration::from_millis(RAND_SHARE_BROADCAST_INTERVAL_MS),
        );

        let (acked_rand_decision_tx, acked_rand_decision_rx) =
            create_channel::<RandDecision>();

        let (send_delta_request_tx, send_delta_request_rx) = create_channel::<Author>();

        Self {
            author,
            epoch,
            verifier,
            rand_store,
            previous_dequeue_time: Instant::now(),
            ordered_block_rx,
            rand_ready_block_tx,
            buffer_manager_rx,
            reset_rx,
            stop: false,
            network_sender,
            rand_msg_rx,
            acked_rand_decision_tx,
            acked_rand_decision_rx,
            send_delta_request_tx,
            send_delta_request_rx,
            reliable_broadcast: Arc::new(reliable_broadcast),
        }
    }

    fn do_reliable_broadcast(&self, shares: RandShare) -> DropGuard {
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let task = self.reliable_broadcast.broadcast(
            shares,
            ShareAckState::new(self.verifier.get_ordered_account_addresses_iter(), self.rand_store.rand_config().unwrap().clone(), self.acked_rand_decision_tx.clone(), self.send_delta_request_tx.clone()),
        );
        tokio::spawn(Abortable::new(task, abort_registration));
        DropGuard::new(abort_handle)
    }

    async fn process_decision(&mut self, decision: RandDecision) {
        match self.rand_store.add_decision(decision.clone()) {
            Ok(AddDecisionResult::NewRandReadyBlock) => self.try_dequeue().await,
            Ok(AddDecisionResult::None) => (),
            Err(e) => {
                warn!("[RandManager] error when processing decision: {}", e);
            },
        }
    }

    async fn process_acked_decision(&mut self, decision: RandDecision) {
        log_rand_event(LogEvent::ReceiveAckedRandDecision, self.author, None, decision.block_id(), decision.round());

        observe_block(decision.timestamp(), BlockStage::RAND_RCV_ACK_DECISION);

        debug!(
            "[RandManager] Received acked decision for round {:?}",
            decision.round(),
        );

        self.process_decision(decision).await;
    }

    async fn send_apk_delta(&mut self, peer: Author) -> anyhow::Result<()> {
        debug!("[RandManager] Sending delta to peer {:?}", peer);
        let delta = self.rand_store.rand_config().unwrap().get_delta(&self.author, &Mode::Fallback).cloned().expect("[RandManager] No local delta to send to peer!");
        let delta_msg = DeltaMsg::new(self.author, delta);
        self.network_sender.send_rb_rpc(peer, RandMessage::Delta(delta_msg), Duration::from_millis(DELTA_SENDING_TIMEOUT)).await.map(|_| ())
    }

    fn process_apk_delta(&mut self, peer: &AccountAddress, delta: Delta) -> anyhow::Result<()> {
        if let Some(rand_config) = self.rand_store.rand_config.as_mut() {
            rand_config.add_delta(peer, delta, &Mode::Fallback)
        } else {
            bail!("[RandManager] No rand_config!");
        }
    }

    async fn process_share(&mut self, share: RandShare) -> anyhow::Result<RandMessage> {
        log_rand_event(LogEvent::ReceiveRandShare, self.author, Some(*share.author()), share.id(), share.round());

        if share.apk_delta().is_some() {
            self.process_apk_delta(share.author(), share.apk_delta().clone().unwrap())?;
        }
        match self.rand_store.add_share(share.clone()) {
            Ok((maybe_decision, result)) => {
                // Ack back decision if available
                // rand todo: prevent DDoS
                match result {
                    AddDecisionResult::NewRandReadyBlock => self.try_dequeue().await,
                    AddDecisionResult::None => (),
                };

                Ok(RandMessage::ShareAck(maybe_decision))
            },
            Err(e) => {
                bail!("[RandManager] error when processing share: {}", e);
            },
        }
    }

    async fn process_rand_msg(&mut self, rand_msg: IncomingRandRequest) -> anyhow::Result<()> {
        let IncomingRandRequest {
            req,
            protocol,
            response_sender,
        } = rand_msg;

        let rand_msg = match req {
            RandMessage::Share(share) => self.process_share(share).await?,
            RandMessage::Delta(delta) => {
                self.process_apk_delta(&delta.author, delta.delta)?;
                RandMessage::DeltaAck(())
            },
            _ => {
                bail!("[RandManager] error unknown rpc message {:?}", req)
            },
        };

        let response = ConsensusMsg::RandMessage(Box::new(rand_msg));

        match protocol.to_bytes(&response) {
            Ok(bytes) => {
                response_sender.send(Ok(bytes.into())).map_err(|_| anyhow::anyhow!("[RandManager] error unable to respond to rpc"))
            },
            Err(e) => {
                response_sender.send(Err(RpcError::ApplicationError(e))).map_err(|_| anyhow::anyhow!("[RandManager] error unable to respond to rpc"))
            },
        }
    }

    /// process incoming ordered blocks
    /// push them into the randomness queue and broadcast the randomness share
    async fn process_ordered_blocks(&mut self, blocks: OrderedBlocks) {
        // Note: We assume the randomness is built on top of dag,
        // so execution pipeline should process a single block every time and there is no NIL block
        
        // let mut maybe_randomness = blocks.maybe_randomness;
        // if maybe_randomness.is_none() {
        //     maybe_randomness = self.rand_store.get_randomness(&blocks.ordered_blocks.last().unwrap().round()).cloned();
        //     if maybe_randomness.is_some() {
        //         debug!("[RandManager] Round {} received blocks with fallback rand", maybe_randomness.as_ref().unwrap().round());
        //     } else {
        //         debug!("[RandManager] Round {} received blocks with no rand", blocks.ordered_blocks.last().unwrap().round());
        //     }
        // } else {
        //     debug!("[RandManager] Round {} received blocks with optimistic rand", maybe_randomness.as_ref().unwrap().round());
        // }

        observe_block(blocks.ordered_blocks.last().unwrap().timestamp_usecs(), BlockStage::RAND_ENTER);
        
        let maybe_randomness = blocks.maybe_randomness.or_else(|| {
            self.rand_store.get_randomness(&blocks.ordered_blocks.last()?.round()).cloned()
        });
        
        let block = match maybe_randomness {
            Some(randomness) => {
                let blocks = RandReadyBlocks {
                    ordered_blocks: blocks.ordered_blocks,
                    ordered_proof: blocks.ordered_proof,
                    callback: blocks.callback,
                    randomness,
                };
                BlockQueueItem::RandReady(Box::new(blocks))
            },
            None => {
                if self.rand_store.rand_config().is_none() {
                    // If no rand_config, generate a dummy randomness
                    // It happens only on first epoch which has no DKG
                    // rand todo: hard code DKG for the first epoch on forge
                    debug!("[RandManager] No rand_config, generate a dummy randomness for round {} block", blocks.ordered_blocks.last().unwrap().round()); 
                    let blocks = RandReadyBlocks {
                        ordered_blocks: blocks.ordered_blocks,
                        ordered_proof: blocks.ordered_proof,
                        callback: blocks.callback,
                        randomness: Randomness::default(),
                    };
                    BlockQueueItem::RandReady(Box::new(blocks))
                } else {
                    let block = blocks.ordered_blocks.last().unwrap();
                    let metadata = RandMetadata::new(block.epoch(), block.round(), block.id(), block.timestamp_usecs());
                    let ask = &self.rand_store.rand_config().as_ref().unwrap().keys_f.ask;
                    let proof = <WVUF as WeightedVUF>::create_share(&ask, metadata.to_bytes().as_slice());
                    let mut apk_delta = None;
                    if block.round() <= 1 {
                        // Sending apk_delta along with the share for the first round
                        // If a node has no apk_delta, it will send MissingDelta in ShareAck
                        apk_delta = self.rand_store.rand_config().unwrap().get_delta(&self.rand_store.rand_config().unwrap().author, &Mode::Fallback).cloned();
                    }
                    let share = RandShare::new(self.author, Mode::Fallback, metadata, proof, apk_delta);

                    observe_block(share.timestamp(), BlockStage::RAND_BC_SHARE);
                    log_rand_event(LogEvent::BroadcastRandShare, self.author, None, share.id(), share.round());

                    let drop_guard = self.do_reliable_broadcast(share);
                    let blocks = OrderedBlocks {
                        ordered_blocks: blocks.ordered_blocks,
                        ordered_proof: blocks.ordered_proof,
                        callback: blocks.callback,
                        maybe_randomness: None,
                        timed_drop_guard: Some((Instant::now(), drop_guard)),
                    };

                    BlockQueueItem::Ordered(Box::new(blocks))
                }
            },
        };
        let mut try_dequeue = !block.is_ordered();
        match self.rand_store.add_block(block) {
            Ok((_, AddDecisionResult::NewRandReadyBlock)) => {
                try_dequeue = true;
            },
            Ok(_) => (),
            Err(e) => {
                error!("[RandManager] error when processing ordered blocks: {}", e);
            },
        }
        if try_dequeue {
            self.try_dequeue().await;
        }
    }

    async fn try_dequeue(&mut self) {
        let rand_ready_blocks = self.rand_store.dequeue_rand_ready_prefix();
        if rand_ready_blocks.is_empty() {
            return;
        }
        self.previous_dequeue_time = Instant::now();
        debug!("[RandManager] Dequeue {} blocks of epoch {} and rounds {:?}, block_queue: {:?}", rand_ready_blocks.len(), self.epoch, rand_ready_blocks.iter().map(|blocks| blocks.ordered_blocks.last().unwrap().round()).collect_vec(), self.rand_store.block_queue());
        for block in rand_ready_blocks {
            self.rand_ready_block_tx
                .send(block)
                .await
                .expect("[RandManager] error failed to send execution ready blocks to buffer manager");
        }
    }

    /// Reset any request in rand manager, this is important to avoid race condition with state sync.
    /// Incoming ordered blocks are pulled, it should only have existing blocks but no new blocks until reset finishes.
    async fn reset(&mut self) {
        self.rand_store.reset();
        self.previous_dequeue_time = Instant::now();
        // purge the incoming blocks queue
        while let Ok(Some(_)) = self.ordered_block_rx.try_next() {}
    }

    async fn process_reset_request(&mut self, request: ResetRequest) {
        let ResetRequest { tx, stop } = request;
        info!("[RandManager] Reset received");

        self.stop = stop;
        self.reset().await;

        tx.send(ResetAck::default()).unwrap();
        info!("[RandManager] Reset finishes");
    }

    async fn rebroadcast_rand_shares(&mut self) {
        if self.previous_dequeue_time.elapsed()
            < Duration::from_millis(RAND_SHARE_BROADCAST_INTERVAL_MS)
        {
            return;
        }
        let rebroadcast_rounds = self.rand_store.rebroadcast_rounds(Duration::from_millis(RAND_SHARE_REBROADCAST_INTERVAL_MS));

        if !rebroadcast_rounds.is_empty() {
            // rand todo: add counters
            info!("[RandManager] Re-broadcast randomness shares of rounds {:?}", rebroadcast_rounds);
        }

        for round in rebroadcast_rounds {
            let share = self.rand_store.get_my_share(&round).unwrap().clone();
            observe_block(share.timestamp(), BlockStage::RAND_RE_BC_SHARE);

            self.rand_store.update_guard(round, self.do_reliable_broadcast(share));
        }
    }

    fn update_rand_manager_metrics(&self) {
        let mut pending_ordered = 0;
        let mut pending_rand_ready = 0;

        for (_, item) in self.rand_store.block_queue() {
            match item {
                BlockQueueItem::Ordered(_) => pending_ordered += 1,
                BlockQueueItem::RandReady(_) => pending_rand_ready += 1,
            }
        }

        counters::NUM_BLOCKS_IN_PIPELINE
            .with_label_values(&["ordered"])
            .set(pending_ordered as i64);
        counters::NUM_BLOCKS_IN_PIPELINE
            .with_label_values(&["rand_ready"])
            .set(pending_rand_ready as i64);
    }

    fn print_rand_store(&self) {
        if !self.rand_store.block_queue().is_empty() || !self.rand_store.rand_map().is_empty() {
            debug!("[RandManager] printing rand_store of epoch {}: block queue: {:?}", self.epoch, self.rand_store.block_queue());
        }
    }

    pub async fn start(mut self) {
        info!("Randomness manager starts.");
        let mut rebroadcast_interval = tokio::time::interval(Duration::from_millis(REBROADCAST_LOOP_INTERVAL_MS));
        let mut garbage_collect_interval = tokio::time::interval(Duration::from_millis(GARBAGE_COLLECT_LOOP_INTERVAL_MS));

        while !self.stop {
            // If no rand_config, only process ordered_block_rx
            if self.rand_store.rand_config().is_none() {
                ::futures::select! {
                    blocks = self.ordered_block_rx.select_next_some() => {
                        monitor!("rand_manager_process_ordered", {
                            self.process_ordered_blocks(blocks).await;
                        });
                    },
                    reset_event = self.reset_rx.select_next_some() => {
                        monitor!("rand_manager_process_reset",
                        self.process_reset_request(reset_event).await);
                    },
                }
            } else {
                ::futures::select! {
                    blocks = self.ordered_block_rx.select_next_some() => {
                        monitor!("rand_manager_process_ordered", {
                            self.process_ordered_blocks(blocks).await;
                        });
                    },
                    rand_msg = self.rand_msg_rx.select_next_some() => {
                        monitor!("rand_manager_process_rand_msg", {
                            if let Err(e) = self.process_rand_msg(rand_msg).await {
                                warn!(error = ?e, "[RandManager] error processing rand msg");
                            }
                        });
                    }
                    rand_decision = self.acked_rand_decision_rx.select_next_some() => {
                        monitor!("rand_manager_process_rand_decision", {
                            self.process_acked_decision(rand_decision).await;
                        });
                    },
                    peer = self.send_delta_request_rx.select_next_some() => {
                        monitor!("rand_manager_send_apk_delta", {
                            if let Err(e) = self.send_apk_delta(peer.clone()).await {
                                warn!(error = ?e, "[RandManager] error sending apk delta to peer {:?}", peer);
                            }
                        });
                    },
                    buffer_manager_event = self.buffer_manager_rx.select_next_some() => {
                        monitor!("rand_manager_process_buffer_manager_event", {
                        match buffer_manager_event {
                            BufferManagerEvent::Commit(committed_round) => {
                                self.rand_store.update_rounds(committed_round);
                            },
                        }
                        });
                    },
                    reset_event = self.reset_rx.select_next_some() => {
                        monitor!("rand_manager_process_reset",
                        self.process_reset_request(reset_event).await);
                    },
                    _ = rebroadcast_interval.tick().fuse() => {
                        self.print_rand_store();
    
                        monitor!("rand_manager_process_rebroadcast_interval_tick", {
                        self.update_rand_manager_metrics();
                        self.rebroadcast_rand_shares().await
                        });
                    },
                    _ = garbage_collect_interval.tick().fuse() => {
                        monitor!("rand_manager_process_garbage_collect_interval_tick", {
                        self.rand_store.garbage_collect();
                        });
                    },
                    // no else branch here because interval.tick will always be available
                }
            }
        }
        info!("Randomness manager stops.");
    }
}
