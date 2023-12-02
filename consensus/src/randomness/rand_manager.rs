// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;
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
use aptos_reliable_broadcast::ReliableBroadcast;
use aptos_time_service::TimeService;
use aptos_types::{
    account_address::AccountAddress, validator_verifier::ValidatorVerifier, randomness::{RandDecision, RandConfig, RandMetadata, Delta, WVUF}, validator_signer::ValidatorSigner,
};
use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    future::{AbortHandle, Abortable},
    FutureExt, SinkExt, StreamExt,
};
use tokio::time::{Duration, Instant};
use tokio_retry::strategy::ExponentialBackoff;

use super::{block_queue::{OrderedBlocks, RandReadyBlocks}, types::{RandMessage, RandShare, ShareAckState, CertifiedDelta, DeltaAck, SignatureBuilder, CertifiedDeltaAckState}, rand_store::{RandStore, AddDecisionResult}};

// rand todo: parameters. These parameters can sometimes pass smoke test.
pub const RAND_SHARE_BROADCAST_INTERVAL_MS: u64 = 1_000;
pub const RAND_SHARE_REBROADCAST_INTERVAL_MS: u64 = 3_000;
pub const REBROADCAST_LOOP_INTERVAL_MS: u64 = 1_000;
pub const GARBAGE_COLLECT_LOOP_INTERVAL_MS: u64 = 10_000;

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
    verifier: Arc<ValidatorVerifier>,
    // for signing delta message
    signer: Arc<ValidatorSigner>,

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
    rand_msg_rx: aptos_channels::aptos_channel::Receiver<AccountAddress, IncomingRandRequest>,

    // local channels
    acked_rand_decision_tx: Sender<RandDecision>,
    acked_rand_decision_rx: Receiver<RandDecision>,

    reliable_broadcast: Arc<ReliableBroadcast<RandMessage, ExponentialBackoff>>,
}

impl RandManager {
    pub fn new(
        author: Author,
        epoch: u64,
        verifier: Arc<ValidatorVerifier>,
        signer: Arc<ValidatorSigner>,
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
        // rand todo: parameters
        let rb_backoff_policy = ExponentialBackoff::from_millis(2).factor(50).max_delay(Duration::from_secs(5));
        let reliable_broadcast = Arc::new(ReliableBroadcast::new(
            verifier.get_ordered_account_addresses(),
            network_sender.clone(),
            rb_backoff_policy,
            TimeService::real(),
            Duration::from_millis(RAND_SHARE_BROADCAST_INTERVAL_MS),
        ));

        let (acked_rand_decision_tx, acked_rand_decision_rx) =
            create_channel::<RandDecision>();

        // reliable broadcast my delta
        let delta_rb_drop_guard = match &rand_config {
            Some(rand_config) => {
                let apk_delta: Delta = rand_config.get_my_delta().clone();
                let delta_msg = DeltaMsg::new(epoch, author, apk_delta);
                Some(Self::reliable_broadcast_delta(delta_msg, reliable_broadcast.clone(), verifier.clone()))
            },
            None => None,
        };

        let rand_store = RandStore::new(author, rand_config, delta_rb_drop_guard, gc_gap_below, gc_gap_above);

        Self {
            author,
            epoch,
            verifier,
            signer,
            rand_store,
            previous_dequeue_time: Instant::now(),
            ordered_block_rx,
            rand_ready_block_tx,
            buffer_manager_rx,
            reset_rx,
            stop: false,
            rand_msg_rx,
            acked_rand_decision_tx,
            acked_rand_decision_rx,
            reliable_broadcast,
        }
    }

    fn reliable_broadcast_delta(delta_msg: DeltaMsg, reliable_broadcast: Arc<ReliableBroadcast<RandMessage, ExponentialBackoff>>, verifier: Arc<ValidatorVerifier>) -> DropGuard {
        let rb = reliable_broadcast.clone();
        let rb2 = reliable_broadcast.clone();
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let signature_builder =
            SignatureBuilder::new(delta_msg.metadata().clone(), verifier.clone());
        let cert_ack_set = CertifiedDeltaAckState::new(verifier.get_ordered_account_addresses_iter());

        let delta_msg_clone = delta_msg.clone();
        let metadata = delta_msg.metadata().clone();
        let delta_broadcast = async move {
            rb.broadcast(delta_msg_clone, signature_builder).await
        };
        let core_task = delta_broadcast.then(move |certificate| {
            let certified_delta =
                CertifiedDelta::new(delta_msg.clone(), certificate.signatures().to_owned());
            rb2.broadcast(certified_delta, cert_ack_set)
        });
        let task = async move {
            debug!("[RandManager] Start reliable broadcast delta {:?}", metadata);
            core_task.await;
            debug!("[RandManager] Finish reliable broadcast delta {:?}", metadata);
        };
        tokio::spawn(Abortable::new(task, abort_registration));
        DropGuard::new(abort_handle)
    }

    fn do_reliable_broadcast(&self, shares: RandShare) -> DropGuard {
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let task = self.reliable_broadcast.broadcast(
            shares,
            ShareAckState::new(self.verifier.get_ordered_account_addresses_iter(), self.rand_store.rand_config().unwrap().clone(), self.acked_rand_decision_tx.clone()),
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
            "[RandManager] process_acked_decision, epoch={} round={}",
            self.epoch,
            decision.round(),
        );

        self.process_decision(decision).await;
    }

    fn process_delta(&mut self, delta_msg: DeltaMsg) -> anyhow::Result<RandMessage> {
        if let Some(rand_config) = self.rand_store.rand_config.as_mut() {
            delta_msg.verify()?;
            // only sign delta once to avoid equivocation
            rand_config.add_signed_delta(delta_msg.author(), delta_msg.delta().clone())?;
            let signature = delta_msg.sign_vote(&self.signer)?;
            let ack = DeltaAck::new(delta_msg.metadata().clone(), signature);
            Ok(RandMessage::DeltaAck(ack))
        } else {
            bail!("[RandManager] No rand_config!");
        }
    }

    fn process_certified_delta(&mut self, certified_delta: CertifiedDelta) -> anyhow::Result<RandMessage> {
        if let Some(rand_config) = self.rand_store.rand_config.as_mut() {
            if rand_config.get_certified_apk(certified_delta.author()).is_none() {
                certified_delta.verify(&self.verifier)?;
                rand_config.add_certified_delta(certified_delta.author(), certified_delta.delta().clone())?;
            }
            Ok(RandMessage::CertifiedDeltaAck(()))
        } else {
            bail!("[RandManager] No rand_config!");
        }
    }


    async fn process_share(&mut self, share: RandShare) -> anyhow::Result<RandMessage> {
        log_rand_event(LogEvent::ReceiveRandShare, self.author, Some(*share.author()), share.id(), share.round());

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
            RandMessage::Delta(delta_msg) => self.process_delta(delta_msg)?,
            RandMessage::CertifiedDelta(certified_delta) => self.process_certified_delta(certified_delta)?,
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

        let OrderedBlocks {
            mut ordered_blocks,
            ordered_proof,
            callback,
            maybe_randomness,
        } = blocks;

        let rounds: Vec<Round> = ordered_blocks.iter().map(|b|b.round()).collect();
        debug!("[RandManager] process_ordered_blocks, epoch={}, rounds={:?}", self.epoch, rounds);

        let num_blocks = ordered_blocks.len();

        let optimistic_rands = if maybe_randomness.is_some() {
            // If optimistic randomness is present, it must be from DAG consensus, which also ensures `num_blocks==1`.
            assert_eq!(1, num_blocks);
            vec![maybe_randomness]
        } else {
            vec![None; num_blocks]
        };

        let mut num_undecided_blocks = num_blocks;
        let mut timed_drop_guards: Vec<Option<(Instant, DropGuard)>> = (0..num_blocks).map(|_|None).collect();

        for (idx, (block, optimistic_rand)) in ordered_blocks.iter_mut().zip(optimistic_rands.into_iter()).enumerate() {
            observe_block(block.timestamp_usecs(), BlockStage::RAND_ENTER);
            let round = block.round();
            let fallback_rand = self.rand_store.get_randomness(&round).map(|r|r.clone());
            assert!(block.randomness.is_none());
            if let Some(r) = optimistic_rand.or(fallback_rand) {
                num_undecided_blocks -= 1;
                block.update_randomness(r);
            } else if let Some(rand_config) = self.rand_store.rand_config.as_ref() {
                let ask = &rand_config.keys.ask;
                let metadata = RandMetadata::new(block.epoch(), block.round(), block.id(), block.timestamp_usecs());
                let proof = <WVUF as WeightedVUF>::create_share(&ask, metadata.to_bytes().as_slice());
                let share = RandShare::new(self.author, metadata, proof);

                observe_block(share.timestamp(), BlockStage::RAND_BC_SHARE);
                log_rand_event(LogEvent::BroadcastRandShare, self.author, None, share.id(), share.round());

                let drop_guard = self.do_reliable_broadcast(share);
                timed_drop_guards[idx] = Some((Instant::now(), drop_guard));
            } else {
                // If no rand_config, generate a dummy randomness
                // It happens only on first epoch which has no DKG
                // rand todo: hard code DKG for the first epoch on forge
                debug!("[RandManager] No randomness for round {} and no rand_config to create one. Give up.", round);
                num_undecided_blocks -= 1;
            }
        }

        let offsets_by_round: BTreeMap<Round, usize> = rounds.iter().enumerate().map(|(idx, round)| (*round, idx)).collect();
        let item = BlockQueueItem {
            ordered_blocks,
            offsets_by_round,
            ordered_proof,
            callback,
            num_undecided_blocks,
            timed_drop_guards,
        };

        let mut try_dequeue = item.num_undecided_blocks == 0;
        match self.rand_store.add_item(item) {
            Ok(results) if results.iter().any(|result| matches!(result, &AddDecisionResult::NewRandReadyBlock)) => {
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
        for block in rand_ready_blocks {
            let rounds: Vec<Round> = block.ordered_blocks.iter().map(|b|b.round()).collect();
            debug!("[RandManager] Dequeuing, epoch={}, rounds={:?}", self.epoch, rounds);
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
            info!("[RandManager] Re-broadcast randomness shares of epoch {} rounds {:?}", self.epoch, rebroadcast_rounds);
        }

        for round in rebroadcast_rounds {
            if let Some(share) = self.rand_store.get_my_share(&round) {
                observe_block(share.timestamp(), BlockStage::RAND_RE_BC_SHARE);
                self.rand_store.update_guard(round, self.do_reliable_broadcast(share.clone()));
            } else {
                debug!("[RandManager] No local share for epoch {} round {} to re-broadcast", self.epoch, round);
            }
        }
    }

    fn update_rand_manager_metrics(&self) {
        let mut pending_ordered = 0;
        let mut pending_rand_ready = 0;

        for (_, item) in self.rand_store.block_queue() {
            pending_ordered += item.num_undecided_blocks;
            pending_rand_ready += item.num_blocks() - item.num_undecided_blocks;
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
