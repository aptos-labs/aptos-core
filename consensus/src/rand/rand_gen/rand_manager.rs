// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
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
        storage::interface::AugDataStorage,
        types::{AugmentedData, CertifiedAugData, RandConfig, RequestShare, Share},
    },
};
use aptos_bounded_executor::BoundedExecutor;
use aptos_consensus_types::common::Author;
use aptos_infallible::Mutex;
use aptos_logger::{error, info, spawn_named, warn};
use aptos_network::{protocols::network::RpcError, ProtocolId};
use aptos_reliable_broadcast::{DropGuard, ReliableBroadcast};
use aptos_time_service::TimeService;
use aptos_types::{
    epoch_state::EpochState,
    randomness::{RandMetadata, Randomness},
    validator_signer::ValidatorSigner,
};
use bytes::Bytes;
use futures::future::{AbortHandle, Abortable};
use futures_channel::oneshot;
use std::{sync::Arc, time::Duration};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio_retry::strategy::ExponentialBackoff;

pub type Sender<T> = UnboundedSender<T>;
pub type Receiver<T> = UnboundedReceiver<T>;

pub struct RandManager<S: Share, D: AugmentedData, Storage> {
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
    aug_data_store: AugDataStore<D, Storage>,
    block_queue: BlockQueue,
}

impl<S: Share, D: AugmentedData, Storage: AugDataStorage<D>> RandManager<S, D, Storage> {
    pub fn new(
        author: Author,
        epoch_state: Arc<EpochState>,
        signer: Arc<ValidatorSigner>,
        config: RandConfig,
        outgoing_blocks: Sender<OrderedBlocks>,
        network_sender: Arc<NetworkSender>,
        db: Arc<Storage>,
        bounded_executor: BoundedExecutor,
    ) -> Self {
        let rb_backoff_policy = ExponentialBackoff::from_millis(2)
            .factor(100)
            .max_delay(Duration::from_secs(10));
        let reliable_broadcast = Arc::new(ReliableBroadcast::new(
            epoch_state.verifier.get_ordered_account_addresses(),
            network_sender.clone(),
            rb_backoff_policy,
            TimeService::real(),
            Duration::from_secs(10),
            bounded_executor,
        ));
        let (decision_tx, decision_rx) = unbounded_channel();
        let rand_store = Arc::new(Mutex::new(RandStore::new(
            epoch_state.epoch,
            author,
            config.clone(),
            decision_tx,
        )));
        let aug_data_store = AugDataStore::new(epoch_state.epoch, signer, config.clone(), db);

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
        }
    }

    fn process_incoming_blocks(&mut self, blocks: OrderedBlocks) {
        let broadcast_handles: Vec<_> = blocks
            .ordered_blocks
            .iter()
            .map(|block| RandMetadata::from(block.block()))
            .map(|metadata| self.process_incoming_metadata(metadata))
            .collect();
        let queue_item = QueueItem::new(blocks, Some(broadcast_handles));
        self.block_queue.push_back(queue_item);
    }

    fn process_incoming_metadata(&self, metadata: RandMetadata) -> DropGuard {
        let self_share = S::generate(&self.config, metadata.clone());
        let mut rand_store = self.rand_store.lock();
        rand_store.add_rand_metadata(metadata.clone());
        rand_store
            .add_share(self_share.clone())
            .expect("Add self share should succeed");
        self.network_sender
            .broadcast_without_self(RandMessage::<S, D>::Share(self_share).into_network_message());
        self.spawn_aggregate_shares_task(metadata)
    }

    fn process_ready_blocks(&mut self, ready_blocks: Vec<OrderedBlocks>) {
        for blocks in ready_blocks {
            let _ = self.outgoing_blocks.send(blocks);
        }
    }

    fn process_reset(&mut self, request: ResetRequest) {
        let ResetRequest { tx, signal } = request;
        let target_round = match signal {
            ResetSignal::Stop => 0,
            ResetSignal::TargetRound(round) => round,
        };
        self.block_queue = BlockQueue::new();
        self.rand_store.lock().reset(target_round);
        self.stop = matches!(signal, ResetSignal::Stop);
        let _ = tx.send(ResetAck::default());
    }

    fn process_randomness(&mut self, randomness: Randomness) {
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
        let _ = sender.send(Ok(protocol.to_bytes(&msg).unwrap().into()));
    }

    async fn verification_task(
        epoch_state: Arc<EpochState>,
        mut incoming_rpc_request: Receiver<IncomingRandGenRequest>,
        verified_msg_tx: UnboundedSender<RpcRequest<S, D>>,
        rand_config: RandConfig,
        bounded_executor: BoundedExecutor,
    ) {
        while let Some(rand_gen_msg) = incoming_rpc_request.recv().await {
            let tx = verified_msg_tx.clone();
            let epoch_state_clone = epoch_state.clone();
            let config_clone = rand_config.clone();
            bounded_executor
                .spawn(async move {
                    match bcs::from_bytes::<RandMessage<S, D>>(rand_gen_msg.req.data()) {
                        Ok(msg) => {
                            if msg
                                .verify(&epoch_state_clone, &config_clone, rand_gen_msg.sender)
                                .is_ok()
                            {
                                let _ = tx.send(RpcRequest {
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
        let round = metadata.round();
        let rand_store = self.rand_store.clone();
        let task = async move {
            tokio::time::sleep(Duration::from_millis(300)).await;
            let maybe_existing_shares = rand_store.lock().get_all_shares_authors(&metadata);
            if let Some(existing_shares) = maybe_existing_shares {
                let epoch = epoch_state.epoch;
                let request = RequestShare::new(epoch, metadata);
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
                rb.multicast(request, aggregate_state, targets).await;
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

    async fn broadcast_aug_data(&mut self) -> CertifiedAugData<D> {
        if let Some(certified_data) = self.aug_data_store.get_my_certified_aug_data() {
            info!("[RandManager] Already have certified aug data");
            return certified_data;
        }
        let data = self
            .aug_data_store
            .get_my_aug_data()
            .unwrap_or_else(|| D::generate(&self.config));
        // Add it synchronously to avoid race that it sends to others but panics before it persists locally.
        self.aug_data_store
            .add_aug_data(data.clone())
            .expect("Add self aug data should succeed");
        let aug_ack = AugDataCertBuilder::new(data.clone(), self.epoch_state.clone());
        let rb = self.reliable_broadcast.clone();
        info!("[RandManager] Start broadcasting aug data");
        let certified_data = rb.broadcast(data, aug_ack).await;
        info!("[RandManager] Finish broadcasting aug data");
        certified_data
    }

    fn broadcast_certified_aug_data(&mut self, certified_data: CertifiedAugData<D>) -> DropGuard {
        let rb = self.reliable_broadcast.clone();
        let validators = self.epoch_state.verifier.get_ordered_account_addresses();
        // Add it synchronously to be able to sign without a race that we get to sign before the broadcast reaches aug store.
        self.aug_data_store
            .add_certified_aug_data(certified_data.clone())
            .expect("Add self aug data should succeed");
        let task = async move {
            let ack_state = Arc::new(CertifiedAugDataAckState::new(validators.into_iter()));
            info!("[RandManager] Start broadcasting certified aug data");
            rb.broadcast(certified_data, ack_state).await;
            info!("[RandManager] Finish broadcasting certified aug data");
        };
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        tokio::spawn(Abortable::new(task, abort_registration));
        DropGuard::new(abort_handle)
    }

    pub async fn start(
        mut self,
        mut incoming_blocks: Receiver<OrderedBlocks>,
        incoming_rpc_request: Receiver<IncomingRandGenRequest>,
        mut reset_rx: Receiver<ResetRequest>,
        bounded_executor: BoundedExecutor,
    ) {
        info!("RandManager started");
        let (verified_msg_tx, mut verified_msg_rx) = tokio::sync::mpsc::unbounded_channel();
        let epoch_state = self.epoch_state.clone();
        let rand_config = self.config.clone();
        spawn_named!(
            "rand manager verification",
            Self::verification_task(
                epoch_state,
                incoming_rpc_request,
                verified_msg_tx,
                rand_config,
                bounded_executor,
            )
        );

        let certified_data = self.broadcast_aug_data().await;
        let _guard = self.broadcast_certified_aug_data(certified_data);

        while !self.stop {
            tokio::select! {
                Some(blocks) = incoming_blocks.recv() => {
                    self.process_incoming_blocks(blocks);
                }
                Some(reset) = reset_rx.recv() => {
                    while incoming_blocks.try_recv().is_ok() {}
                    self.process_reset(reset);
                }
                Some(randomness) = self.decision_rx.recv()  => {
                    self.process_randomness(randomness);
                }
                Some(request) = verified_msg_rx.recv() => {
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
                                        self.rand_store.lock().add_share(share.clone()).expect("Add self share should succeed");
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
                            if let Err(e) = self.rand_store.lock().add_share(share) {
                                warn!("[RandManager] Failed to add share: {}", e);
                            }
                        }
                        RandMessage::AugData(aug_data) => {
                            match self.aug_data_store.add_aug_data(aug_data) {
                                Ok(sig) => self.process_response(protocol, response_sender, RandMessage::AugDataSignature(sig)),
                                Err(e) => error!("[RandManager] Failed to add aug data: {}", e),
                            }
                        }
                        RandMessage::CertifiedAugData(certified_aug_data) => {
                            match self.aug_data_store.add_certified_aug_data(certified_aug_data) {
                                Ok(ack) => self.process_response(protocol, response_sender, RandMessage::CertifiedAugDataAck(ack)),
                                Err(e) => error!("[RandManager] Failed to add certified aug data: {}", e),
                            }
                        }
                        _ => unreachable!("[RandManager] Unexpected message type after verification"),
                    }
                }
            }
            let maybe_ready_blocks = self.block_queue.dequeue_rand_ready_prefix();
            if !maybe_ready_blocks.is_empty() {
                self.process_ready_blocks(maybe_ready_blocks);
            }
        }
        info!("RandManager stopped");
    }
}
