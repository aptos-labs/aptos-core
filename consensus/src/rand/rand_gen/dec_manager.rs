// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters::DEC_QUEUE_SIZE,
    logging::{LogEvent, LogSchema},
    network::{IncomingDecRequest, NetworkSender, TConsensusMsg},
    pipeline::buffer_manager::{OrderedBlocks, ResetAck, ResetRequest, ResetSignal},
    rand::rand_gen::{
        aug_data_store::AugDataStore,
        block_queue::{BlockQueue, QueueItem},
        network_messages::{DecMessage, RpcRequestDecShare},
        dec_store::DecStore,
        reliable_broadcast_state::{
            AugDataCertBuilder, CertifiedAugDataAckState, DecShareAggregateState, ShareAggregateState,
        },
        types::{PathType, RequestDecShare},
    },
};
use aptos_bounded_executor::BoundedExecutor;
use aptos_channels::aptos_channel;
use aptos_config::config::ReliableBroadcastConfig;
use aptos_consensus_types::{common::{Author, Round}, pipelined_block::PipelinedBlock};
use aptos_infallible::Mutex;
use aptos_logger::{error, info, spawn_named, trace, warn};
use aptos_network::{protocols::network::RpcError, ProtocolId};
use aptos_reliable_broadcast::{DropGuard, ReliableBroadcast};
use aptos_time_service::TimeService;
use aptos_types::{
    decryption::{DecConfig, DecKey, DecMetadata, DecShare, FastDecShare}, epoch_state::EpochState, validator_signer::ValidatorSigner
};
use bytes::Bytes;
use fail::fail_point;
use futures::{
    future::{AbortHandle, Abortable},
    FutureExt, StreamExt,
};
use futures_channel::{
    mpsc::{unbounded, UnboundedReceiver, UnboundedSender},
    oneshot,
};
use std::{sync::Arc, time::Duration};
use tokio_retry::strategy::ExponentialBackoff;
use futures::future::join_all;

pub type Sender<T> = UnboundedSender<T>;
pub type Receiver<T> = UnboundedReceiver<T>;

pub struct DecManager {
    author: Author,
    epoch_state: Arc<EpochState>,
    stop: bool,
    config: DecConfig,
    reliable_broadcast: Arc<ReliableBroadcast<DecMessage, ExponentialBackoff>>,
    network_sender: Arc<NetworkSender>,

    // local channel received from dec_store
    decision_rx: Receiver<DecKey>,
    // downstream channels
    outgoing_blocks: Sender<OrderedBlocks>,
    // local state
    dec_store: Arc<Mutex<DecStore>>,
    block_queue: BlockQueue,

    // for decryption fast path
    fast_config: Option<DecConfig>,
}

impl DecManager {
    pub fn new(
        author: Author,
        epoch_state: Arc<EpochState>,
        _signer: Arc<ValidatorSigner>,
        config: DecConfig,
        fast_config: Option<DecConfig>,
        outgoing_blocks: Sender<OrderedBlocks>,
        network_sender: Arc<NetworkSender>,
        bounded_executor: BoundedExecutor,
        rb_config: &ReliableBroadcastConfig,
    ) -> Self {
        let rb_backoff_policy = ExponentialBackoff::from_millis(rb_config.backoff_policy_base_ms)
            .factor(rb_config.backoff_policy_factor)
            .max_delay(Duration::from_millis(rb_config.backoff_policy_max_delay_ms));
        let reliable_broadcast = Arc::new(ReliableBroadcast::new(
            author,
            epoch_state.verifier.get_ordered_account_addresses(),
            network_sender.clone(),
            rb_backoff_policy,
            TimeService::real(),
            Duration::from_millis(rb_config.rpc_timeout_ms),
            bounded_executor,
        ));
        let (decision_tx, decision_rx) = unbounded();

        let dec_store = Arc::new(Mutex::new(DecStore::new(
            epoch_state.epoch,
            author,
            config.clone(),
            fast_config.clone(),
            decision_tx,
        )));

        Self {
            author,
            epoch_state,
            stop: false,
            config,
            reliable_broadcast,
            network_sender,

            decision_rx,
            outgoing_blocks,

            dec_store,
            block_queue: BlockQueue::new(),

            fast_config,
        }
    }

    async fn process_incoming_blocks(&mut self, blocks: OrderedBlocks) {
        let rounds: Vec<u64> = blocks.ordered_blocks.iter().map(|b| b.round()).collect();
        info!(rounds = rounds, "Processing incoming blocks.");

        // let broadcast_handles: Vec<DropGuard> = blocks
        //     .ordered_blocks
        //     .iter()
        //     .filter(|block| block.num_encrypted_txns() > 0)
        //     .map(|block| self.process_incoming_block(block).await)
        //     .collect();

        let mut broadcast_handles = Vec::new();
        for block in blocks
            .ordered_blocks
            .iter()
            .filter(|block| block.num_encrypted_txns() > 0)
        {
            let handle = self.process_incoming_block(block).await;
            broadcast_handles.push(handle);
        }

        let queue_item = QueueItem::new(blocks, Some(broadcast_handles));
        self.block_queue.push_back(queue_item);
    }

    async fn process_incoming_block(&self, block: &PipelinedBlock) -> DropGuard {
        let dec_share = self.derive_self_dec_share(block).await;

        info!(LogSchema::new(LogEvent::BroadcastDecShare)
            .epoch(self.epoch_state.epoch)
            .author(self.author)
            .round(block.round()));

        let maybe_fast_share = if self.fast_config.is_some() {
            Some(self.derive_self_fast_dec_share(block).await)
        } else {
            None
        };

        // Now acquire lock and update store
        {
            let mut dec_store = self.dec_store.lock();
            dec_store.update_highest_known_round(block.round());
            dec_store.add_share(dec_share.clone(), PathType::Slow)
                .expect("Add self dec share should succeed");

            if let Some(fast_share) = maybe_fast_share {
                dec_store
                    .add_share(fast_share.share(), PathType::Fast)
                    .expect("Add self fast dec share should succeed");
            }

            dec_store.add_dec_metadata(dec_share.metadata().clone());
        }

        self.network_sender
            .broadcast_without_self(DecMessage::DecShare(dec_share.clone()).into_network_message());
        self.spawn_aggregate_shares_task(dec_share.metadata().clone())
    }

    async fn derive_self_dec_share(&self, block: &PipelinedBlock) -> DecShare {
        let futures = block.pipeline_futs().unwrap();
        let dec_share = if let Some(fut) = futures.maybe_compute_decryption_share_fut.as_ref() {
            let (share, _) = fut.clone().await.expect("Decryption share computation failed");
            share
        } else {
            panic!(
                "Block {} is encrypted but maybe_compute_decryption_fut is not set",
                block.block().id()
            );
        };

        dec_share
    }

    async fn derive_self_fast_dec_share(&self, block: &PipelinedBlock) -> FastDecShare {
        let futures = block.pipeline_futs().unwrap();
        let fast_dec_share = if let Some(fut) = futures.maybe_compute_decryption_share_fut.as_ref() {
            let (_, fast_share) = fut.clone().await.expect("Decryption share computation failed");
            fast_share
        } else {
            panic!(
                "Block {} is encrypted but maybe_compute_decryption_fut is not set",
                block.block().id()
            );
        };

        fast_dec_share
    }

    fn process_ready_blocks(&mut self, ready_blocks: Vec<OrderedBlocks>) {
        let rounds: Vec<u64> = ready_blocks
            .iter()
            .flat_map(|b| b.ordered_blocks.iter().map(|b3| b3.round()))
            .collect();
        fail_point!("dec_manager::process_ready_blocks", |_| {});
        info!(rounds = rounds, "Processing dec-ready blocks.");

        for blocks in ready_blocks {
            let _ = self.outgoing_blocks.unbounded_send(blocks);
        }
    }

    fn process_reset(&mut self, request: ResetRequest) {
        let ResetRequest { tx, signal } = request;
        let target_round = match signal {
            ResetSignal::Stop => 0,
            ResetSignal::TargetRound(round) => round,
        };
        self.block_queue = BlockQueue::new();
        self.dec_store
            .lock()
            .update_highest_known_round(target_round);
        self.stop = matches!(signal, ResetSignal::Stop);
        let _ = tx.send(ResetAck::default());
    }

    fn process_dec_key(&mut self, dec_key: DecKey) {
        info!(
            metadata = dec_key.metadata,
            dec_key = dec_key.key,
            "Processing decryption key."
        );
        if let Some(item) = self.block_queue.item_mut(dec_key.metadata.round) {
            item.set_dec_key(dec_key.metadata.round, dec_key);
        }
    }

    fn process_response(
        &self,
        protocol: ProtocolId,
        sender: oneshot::Sender<Result<Bytes, RpcError>>,
        message: DecMessage,
    ) {
        let msg = message.into_network_message();
        let _ = sender.send(Ok(protocol
            .to_bytes(&msg)
            .expect("Message should be serializable into protocol")
            .into()));
    }

    async fn verification_task(
        epoch_state: Arc<EpochState>,
        mut incoming_rpc_request: aptos_channel::Receiver<Author, IncomingDecRequest>,
        verified_msg_tx: UnboundedSender<RpcRequestDecShare>,
        dec_config: DecConfig,
        fast_dec_config: Option<DecConfig>,
        bounded_executor: BoundedExecutor,
    ) {
        while let Some(dec_msg) = incoming_rpc_request.next().await {
            let tx = verified_msg_tx.clone();
            let epoch_state_clone = epoch_state.clone();
            let config_clone = dec_config.clone();
            let fast_config_clone = fast_dec_config.clone();
            bounded_executor
                .spawn(async move {
                    match bcs::from_bytes::<DecMessage>(dec_msg.req.data()) {
                        Ok(msg) => {
                            if msg
                                .verify(
                                    &epoch_state_clone,
                                    &config_clone,
                                    &fast_config_clone,
                                )
                                .is_ok()
                            {
                                let _ = tx.unbounded_send(RpcRequestDecShare {
                                    req: msg,
                                    protocol: dec_msg.protocol,
                                    response_sender: dec_msg.response_sender,
                                });
                            }
                        },
                        Err(e) => {
                            warn!("Invalid dec message: {}", e);
                        },
                    }
                })
                .await;
        }
    }

    fn spawn_aggregate_shares_task(&self, metadata: DecMetadata) -> DropGuard {
        let rb = self.reliable_broadcast.clone();
        let aggregate_state = Arc::new(DecShareAggregateState::new(
            self.dec_store.clone(),
            metadata.clone(),
            self.config.clone(),
        ));
        let epoch_state = self.epoch_state.clone();
        let dec_store = self.dec_store.clone();
        let task = async move {
            tokio::time::sleep(Duration::from_millis(300)).await;
            let maybe_existing_shares = dec_store.lock().get_all_shares_authors(&metadata);
            if let Some(existing_shares) = maybe_existing_shares {
                let epoch = epoch_state.epoch;
                let request = RequestDecShare::new(metadata.clone());
                let targets = epoch_state
                    .verifier
                    .get_ordered_account_addresses_iter()
                    .filter(|author| !existing_shares.contains(author))
                    .collect::<Vec<_>>();
                info!(
                    epoch = epoch,
                    round = metadata.round,
                    "[DecManager] Start broadcasting share request for {}",
                    targets.len(),
                );
                rb.multicast(request, aggregate_state, targets)
                    .await
                    .expect("Broadcast cannot fail");
                info!(
                    epoch = epoch,
                    round = metadata.round,
                    "[DecManager] Finish broadcasting share request",
                );
            }
        };
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        tokio::spawn(Abortable::new(task, abort_registration));
        DropGuard::new(abort_handle)
    }

    pub async fn start(
        mut self,
        mut incoming_blocks: Receiver<OrderedBlocks>,
        incoming_rpc_request: aptos_channel::Receiver<Author, IncomingDecRequest>,
        mut reset_rx: Receiver<ResetRequest>,
        bounded_executor: BoundedExecutor,
        highest_known_round: Round,
    ) {
        info!("DecManager started");
        let (verified_msg_tx, mut verified_msg_rx) = unbounded();
        let epoch_state = self.epoch_state.clone();
        let dec_config = self.config.clone();
        let fast_dec_config = self.fast_config.clone();
        {
            self.dec_store
                .lock()
                .update_highest_known_round(highest_known_round);
        }
        spawn_named!(
            "dec manager verification",
            Self::verification_task(
                epoch_state,
                incoming_rpc_request,
                verified_msg_tx,
                dec_config,
                fast_dec_config,
                bounded_executor,
            )
        );

        let mut interval = tokio::time::interval(Duration::from_millis(5000));
        while !self.stop {
            tokio::select! {
                Some(blocks) = incoming_blocks.next() => {
                    self.process_incoming_blocks(blocks).await;
                }
                Some(reset) = reset_rx.next() => {
                    while matches!(incoming_blocks.try_next(), Ok(Some(_))) {}
                    self.process_reset(reset);
                }
                Some(dec_key) = self.decision_rx.next() => {
                    self.process_dec_key(dec_key);
                }
                Some(request) = verified_msg_rx.next() => {
                    let RpcRequestDecShare {
                        req: dec_msg,
                        protocol,
                        response_sender,
                    } = request;
                    match dec_msg {
                        DecMessage::RequestDecShare(request) => {
                            let result = self.dec_store.lock().get_self_share(request.dec_metadata());
                            match result {
                                Ok(maybe_share) => {
                                    // if the block is available
                                    if let Some(block) = self.block_queue.get_block_for_round(request.dec_metadata().round) {
                                        let dec_share = self.derive_self_dec_share(block).await;
                                        self.dec_store.lock().add_share(dec_share.clone(), PathType::Slow).expect("Add self dec share should succeed");
                                        self.process_response(protocol, response_sender, DecMessage::DecShare(dec_share));
                                    } else {
                                        warn!("[DecManager] Block for round {} not found", request.dec_metadata().round);
                                    }
                                },
                                Err(e) => {
                                    warn!("[DecManager] Failed to get share: {}", e);
                                }
                            }
                        }
                        DecMessage::DecShare(share) => {
                            info!(LogSchema::new(LogEvent::ReceiveProactiveDecShare)
                                .author(self.author)
                                .epoch(share.epoch())
                                .round(share.metadata().round)
                                .remote_peer(*share.author()));

                            if let Err(e) = self.dec_store.lock().add_share(share, PathType::Slow) {
                                warn!("[DecManager] Failed to add share: {}", e);
                            }
                        }
                        DecMessage::FastDecShare(share) => {
                            info!(LogSchema::new(LogEvent::ReceiveFastDecShare)
                                .author(self.author)
                                .epoch(share.epoch())
                                .round(share.round())
                                .remote_peer(*share.share.author()));

                            if let Err(e) = self.dec_store.lock().add_share(share.share(), PathType::Fast) {
                                warn!("[DecManager] Failed to add share for fast path: {}", e);
                            }
                        }
                        _ => unreachable!("[DecManager] Unexpected message type after verification"),
                    }
                }
                _ = interval.tick().fuse() => {
                    self.observe_queue();
                },
            }
            let maybe_ready_blocks = self.block_queue.dequeue_dec_ready_prefix();
            if !maybe_ready_blocks.is_empty() {
                self.process_ready_blocks(maybe_ready_blocks);
            }
        }
        info!("DecManager stopped");
    }

    pub fn observe_queue(&self) {
        let queue = &self.block_queue.queue();
        DEC_QUEUE_SIZE.set(queue.len() as i64);
    }
}
