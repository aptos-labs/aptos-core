// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters::RAND_QUEUE_SIZE,
    logging::{LogEvent, LogSchema},
    network::{IncomingRandGenRequest, NetworkSender, TConsensusMsg},
    pipeline::buffer_manager::{OrderedBlocks, ResetAck, ResetRequest, ResetSignal},
    rand::rand_gen::{
        aug_data_store::AugDataStore,
        block_queue::{BlockQueue, QueueItem},
        network_messages::{RandMessage, RpcRequest},
        rand_store::RandStore,
        reliable_broadcast_state::{
            AugDataCertBuilder, CertifiedAugDataAckState, ShareAggregateState,
        },
        storage::interface::RandStorage,
        types::{FastShare, PathType, RandConfig, RequestShare, TAugmentedData, TShare},
    },
};
use aptos_bounded_executor::BoundedExecutor;
use aptos_channels::aptos_channel;
use aptos_config::config::ReliableBroadcastConfig;
use aptos_consensus_types::common::{Author, Round};
use aptos_infallible::Mutex;
use aptos_logger::{error, info, spawn_named, trace, warn};
use aptos_network::{protocols::network::RpcError, ProtocolId};
use aptos_reliable_broadcast::{DropGuard, ReliableBroadcast};
use aptos_time_service::TimeService;
use aptos_types::{
    epoch_state::EpochState,
    randomness::{FullRandMetadata, RandMetadata, Randomness},
    validator_signer::ValidatorSigner,
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

pub type Sender<T> = UnboundedSender<T>;
pub type Receiver<T> = UnboundedReceiver<T>;

pub struct RandManager<S: TShare, D: TAugmentedData> {
    author: Author,
    epoch_state: Arc<EpochState>,
    stop: bool,
    config: RandConfig,
    reliable_broadcast: Arc<ReliableBroadcast<RandMessage<S, D>, ExponentialBackoff>>,
    network_sender: Arc<NetworkSender>,

    // local channel received from rand_store
    decision_rx: Receiver<Randomness>,
    // downstream channels
    outgoing_blocks: Sender<OrderedBlocks>,
    // local state
    rand_store: Arc<Mutex<RandStore<S>>>,
    aug_data_store: AugDataStore<D>,
    block_queue: BlockQueue,

    // for randomness fast path
    fast_config: Option<RandConfig>,
}

impl<S: TShare, D: TAugmentedData> RandManager<S, D> {
    pub fn new(
        author: Author,
        epoch_state: Arc<EpochState>,
        signer: Arc<ValidatorSigner>,
        config: RandConfig,
        fast_config: Option<RandConfig>,
        outgoing_blocks: Sender<OrderedBlocks>,
        network_sender: Arc<NetworkSender>,
        db: Arc<dyn RandStorage<D>>,
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
        let rand_store = Arc::new(Mutex::new(RandStore::new(
            epoch_state.epoch,
            author,
            config.clone(),
            fast_config.clone(),
            decision_tx,
        )));
        let aug_data_store = AugDataStore::new(
            epoch_state.epoch,
            signer,
            config.clone(),
            fast_config.clone(),
            db,
        );

        Self {
            author,
            epoch_state,
            stop: false,
            config,
            reliable_broadcast,
            network_sender,

            decision_rx,
            outgoing_blocks,

            rand_store,
            aug_data_store,
            block_queue: BlockQueue::new(),

            fast_config,
        }
    }

    fn process_incoming_blocks(&mut self, blocks: OrderedBlocks) {
        let rounds: Vec<u64> = blocks.ordered_blocks.iter().map(|b| b.round()).collect();
        info!(rounds = rounds, "Processing incoming blocks.");
        let broadcast_handles: Vec<_> = blocks
            .ordered_blocks
            .iter()
            .map(|block| FullRandMetadata::from(block.block()))
            .map(|metadata| self.process_incoming_metadata(metadata))
            .collect();
        let queue_item = QueueItem::new(blocks, Some(broadcast_handles));
        self.block_queue.push_back(queue_item);
    }

    fn process_incoming_metadata(&self, metadata: FullRandMetadata) -> DropGuard {
        let self_share = S::generate(&self.config, metadata.metadata.clone());
        info!(LogSchema::new(LogEvent::BroadcastRandShare)
            .epoch(self.epoch_state.epoch)
            .author(self.author)
            .round(metadata.round()));
        let mut rand_store = self.rand_store.lock();
        rand_store.update_highest_known_round(metadata.round());
        rand_store
            .add_share(self_share.clone(), PathType::Slow)
            .expect("Add self share should succeed");

        if let Some(fast_config) = &self.fast_config {
            let self_fast_share =
                FastShare::new(S::generate(fast_config, metadata.metadata.clone()));
            rand_store
                .add_share(self_fast_share.rand_share(), PathType::Fast)
                .expect("Add self share for fast path should succeed");
        }

        rand_store.add_rand_metadata(metadata.clone());
        self.network_sender
            .broadcast_without_self(RandMessage::<S, D>::Share(self_share).into_network_message());
        self.spawn_aggregate_shares_task(metadata.metadata)
    }

    fn process_ready_blocks(&mut self, ready_blocks: Vec<OrderedBlocks>) {
        let rounds: Vec<u64> = ready_blocks
            .iter()
            .flat_map(|b| b.ordered_blocks.iter().map(|b3| b3.round()))
            .collect();
        fail_point!("rand_manager::process_ready_blocks", |_| {});
        info!(rounds = rounds, "Processing rand-ready blocks.");

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
        self.rand_store
            .lock()
            .update_highest_known_round(target_round);
        self.stop = matches!(signal, ResetSignal::Stop);
        let _ = tx.send(ResetAck::default());
    }

    fn process_randomness(&mut self, randomness: Randomness) {
        let rand = hex::encode(randomness.randomness());
        info!(
            metadata = randomness.metadata(),
            rand = rand,
            "Processing decisioned randomness."
        );
        if let Some(block) = self.block_queue.item_mut(randomness.round()) {
            block.set_randomness(randomness.round(), randomness);
        }
    }

    fn process_response(
        &self,
        protocol: ProtocolId,
        sender: oneshot::Sender<Result<Bytes, RpcError>>,
        message: RandMessage<S, D>,
    ) {
        let msg = message.into_network_message();
        let _ = sender.send(Ok(protocol
            .to_bytes(&msg)
            .expect("Message should be serializable into protocol")
            .into()));
    }

    async fn verification_task(
        epoch_state: Arc<EpochState>,
        mut incoming_rpc_request: aptos_channel::Receiver<Author, IncomingRandGenRequest>,
        verified_msg_tx: UnboundedSender<RpcRequest<S, D>>,
        rand_config: RandConfig,
        fast_rand_config: Option<RandConfig>,
        bounded_executor: BoundedExecutor,
    ) {
        while let Some(rand_gen_msg) = incoming_rpc_request.next().await {
            let tx = verified_msg_tx.clone();
            let epoch_state_clone = epoch_state.clone();
            let config_clone = rand_config.clone();
            let fast_config_clone = fast_rand_config.clone();
            bounded_executor
                .spawn(async move {
                    match bcs::from_bytes::<RandMessage<S, D>>(rand_gen_msg.req.data()) {
                        Ok(msg) => {
                            if msg
                                .verify(
                                    &epoch_state_clone,
                                    &config_clone,
                                    &fast_config_clone,
                                    rand_gen_msg.sender,
                                )
                                .is_ok()
                            {
                                let _ = tx.unbounded_send(RpcRequest {
                                    req: msg,
                                    protocol: rand_gen_msg.protocol,
                                    response_sender: rand_gen_msg.response_sender,
                                });
                            }
                        },
                        Err(e) => {
                            warn!("Invalid rand gen message: {}", e);
                        },
                    }
                })
                .await;
        }
    }

    fn spawn_aggregate_shares_task(&self, metadata: RandMetadata) -> DropGuard {
        let rb = self.reliable_broadcast.clone();
        let aggregate_state = Arc::new(ShareAggregateState::new(
            self.rand_store.clone(),
            metadata.clone(),
            self.config.clone(),
        ));
        let epoch_state = self.epoch_state.clone();
        let round = metadata.round;
        let rand_store = self.rand_store.clone();
        let task = async move {
            tokio::time::sleep(Duration::from_millis(300)).await;
            let maybe_existing_shares = rand_store.lock().get_all_shares_authors(round);
            if let Some(existing_shares) = maybe_existing_shares {
                let epoch = epoch_state.epoch;
                let request = RequestShare::new(metadata.clone());
                let targets = epoch_state
                    .verifier
                    .get_ordered_account_addresses_iter()
                    .filter(|author| !existing_shares.contains(author))
                    .collect::<Vec<_>>();
                info!(
                    epoch = epoch,
                    round = round,
                    "[RandManager] Start broadcasting share request for {}",
                    targets.len(),
                );
                rb.multicast(request, aggregate_state, targets)
                    .await
                    .expect("Broadcast cannot fail");
                info!(
                    epoch = epoch,
                    round = round,
                    "[RandManager] Finish broadcasting share request",
                );
            }
        };
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        tokio::spawn(Abortable::new(task, abort_registration));
        DropGuard::new(abort_handle)
    }

    async fn broadcast_aug_data(&mut self) -> DropGuard {
        let data = self
            .aug_data_store
            .get_my_aug_data()
            .unwrap_or_else(|| D::generate(&self.config, &self.fast_config));
        // Add it synchronously to avoid race that it sends to others but panics before it persists locally.
        self.aug_data_store
            .add_aug_data(data.clone())
            .expect("Add self aug data should succeed");
        let aug_ack = AugDataCertBuilder::new(data.clone(), self.epoch_state.clone());
        let rb = self.reliable_broadcast.clone();
        let rb2 = self.reliable_broadcast.clone();
        let validators = self.epoch_state.verifier.get_ordered_account_addresses();
        let maybe_existing_certified_data = self.aug_data_store.get_my_certified_aug_data();
        let phase1 = async move {
            if let Some(certified_data) = maybe_existing_certified_data {
                info!("[RandManager] Already have certified aug data");
                return certified_data;
            }
            info!("[RandManager] Start broadcasting aug data");
            info!(LogSchema::new(LogEvent::BroadcastAugData)
                .author(*data.author())
                .epoch(data.epoch()));
            let certified_data = rb.broadcast(data, aug_ack).await.expect("cannot fail");
            info!("[RandManager] Finish broadcasting aug data");
            certified_data
        };
        let ack_state = Arc::new(CertifiedAugDataAckState::new(validators.into_iter()));
        let task = phase1.then(|certified_data| async move {
            info!(LogSchema::new(LogEvent::BroadcastCertifiedAugData)
                .author(*certified_data.author())
                .epoch(certified_data.epoch()));
            info!("[RandManager] Start broadcasting certified aug data");
            rb2.broadcast(certified_data, ack_state)
                .await
                .expect("Broadcast cannot fail");
            info!("[RandManager] Finish broadcasting certified aug data");
        });
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        tokio::spawn(Abortable::new(task, abort_registration));
        DropGuard::new(abort_handle)
    }

    pub async fn start(
        mut self,
        mut incoming_blocks: Receiver<OrderedBlocks>,
        incoming_rpc_request: aptos_channel::Receiver<Author, IncomingRandGenRequest>,
        mut reset_rx: Receiver<ResetRequest>,
        bounded_executor: BoundedExecutor,
        highest_known_round: Round,
    ) {
        info!("RandManager started");
        let (verified_msg_tx, mut verified_msg_rx) = unbounded();
        let epoch_state = self.epoch_state.clone();
        let rand_config = self.config.clone();
        let fast_rand_config = self.fast_config.clone();
        self.rand_store
            .lock()
            .update_highest_known_round(highest_known_round);
        spawn_named!(
            "rand manager verification",
            Self::verification_task(
                epoch_state,
                incoming_rpc_request,
                verified_msg_tx,
                rand_config,
                fast_rand_config,
                bounded_executor,
            )
        );

        let _guard = self.broadcast_aug_data().await;
        let mut interval = tokio::time::interval(Duration::from_millis(5000));
        while !self.stop {
            tokio::select! {
                Some(blocks) = incoming_blocks.next(), if self.aug_data_store.my_certified_aug_data_exists() => {
                    self.process_incoming_blocks(blocks);
                }
                Some(reset) = reset_rx.next() => {
                    while matches!(incoming_blocks.try_next(), Ok(Some(_))) {}
                    self.process_reset(reset);
                }
                Some(randomness) = self.decision_rx.next()  => {
                    self.process_randomness(randomness);
                }
                Some(request) = verified_msg_rx.next() => {
                    let RpcRequest {
                        req: rand_gen_msg,
                        protocol,
                        response_sender,
                    } = request;
                    match rand_gen_msg {
                        RandMessage::RequestShare(request) => {
                            let result = self.rand_store.lock().get_self_share(request.rand_metadata());
                            match result {
                                Ok(maybe_share) => {
                                    let share = maybe_share.unwrap_or_else(|| {
                                        // reproduce previous share if not found
                                        let share = S::generate(&self.config, request.rand_metadata().clone());
                                        self.rand_store.lock().add_share(share.clone(), PathType::Slow).expect("Add self share should succeed");
                                        share
                                    });
                                    self.process_response(protocol, response_sender, RandMessage::Share(share));
                                },
                                Err(e) => {
                                    warn!("[RandManager] Failed to get share: {}", e);
                                }
                            }
                        }
                        RandMessage::Share(share) => {
                            trace!(LogSchema::new(LogEvent::ReceiveProactiveRandShare)
                                .author(self.author)
                                .epoch(share.epoch())
                                .round(share.metadata().round)
                                .remote_peer(*share.author()));

                            if let Err(e) = self.rand_store.lock().add_share(share, PathType::Slow) {
                                warn!("[RandManager] Failed to add share: {}", e);
                            }
                        }
                        RandMessage::FastShare(share) => {
                            trace!(LogSchema::new(LogEvent::ReceiveRandShareFastPath)
                                .author(self.author)
                                .epoch(share.epoch())
                                .round(share.metadata().round)
                                .remote_peer(*share.share.author()));

                            if let Err(e) = self.rand_store.lock().add_share(share.rand_share(), PathType::Fast) {
                                warn!("[RandManager] Failed to add share for fast path: {}", e);
                            }
                        }
                        RandMessage::AugData(aug_data) => {
                            info!(LogSchema::new(LogEvent::ReceiveAugData)
                                .author(self.author)
                                .epoch(aug_data.epoch())
                                .remote_peer(*aug_data.author()));
                            match self.aug_data_store.add_aug_data(aug_data) {
                                Ok(sig) => self.process_response(protocol, response_sender, RandMessage::AugDataSignature(sig)),
                                Err(e) => {
                                    if e.to_string().contains("[AugDataStore] equivocate data") {
                                        warn!("[RandManager] Failed to add aug data: {}", e);
                                    } else {
                                        error!("[RandManager] Failed to add aug data: {}", e);
                                    }
                                },
                            }
                        }
                        RandMessage::CertifiedAugData(certified_aug_data) => {
                            info!(LogSchema::new(LogEvent::ReceiveCertifiedAugData)
                                .author(self.author)
                                .epoch(certified_aug_data.epoch())
                                .remote_peer(*certified_aug_data.author()));
                            match self.aug_data_store.add_certified_aug_data(certified_aug_data) {
                                Ok(ack) => self.process_response(protocol, response_sender, RandMessage::CertifiedAugDataAck(ack)),
                                Err(e) => error!("[RandManager] Failed to add certified aug data: {}", e),
                            }
                        }
                        _ => unreachable!("[RandManager] Unexpected message type after verification"),
                    }
                }
                _ = interval.tick().fuse() => {
                    self.observe_queue();
                },
            }
            let maybe_ready_blocks = self.block_queue.dequeue_rand_ready_prefix();
            if !maybe_ready_blocks.is_empty() {
                self.process_ready_blocks(maybe_ready_blocks);
            }
        }
        info!("RandManager stopped");
    }

    pub fn observe_queue(&self) {
        let queue = &self.block_queue.queue();
        RAND_QUEUE_SIZE.set(queue.len() as i64);
    }
}
