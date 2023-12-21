// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_storage::tracing::{observe_block, BlockStage},
    logging::{LogEvent, LogSchema},
    network::{IncomingRandGenRequest, NetworkSender, TConsensusMsg},
    pipeline::buffer_manager::{OrderedBlocks, ResetAck, ResetRequest, ResetSignal},
    rand::rand_gen::{
        aug_data_store::AugDataStore,
        block_queue::QueueItem,
        network_messages::{RandMessage, RpcRequest},
        rand_store::RandStore,
        reliable_broadcast_state::{
            AugDataCertBuilder, CertifiedAugDataAckState, ShareAggregateState,
        },
        storage::interface::{AugDataStorage, RandStorage},
        types::{AugmentedData, Proof, RandConfig, RequestShare, Share},
    },
};
use aptos_bounded_executor::BoundedExecutor;
use aptos_channels::aptos_channel;
use aptos_consensus_types::{common::Author, randomness::RandMetadata};
use aptos_infallible::Mutex;
use aptos_logger::{error, info, spawn_named, warn};
use aptos_network::{protocols::network::RpcError, ProtocolId};
use aptos_reliable_broadcast::{DropGuard, ReliableBroadcast};
use aptos_time_service::TimeService;
use aptos_types::{epoch_state::EpochState, validator_signer::ValidatorSigner};
use bytes::Bytes;
use futures::{
    future::{AbortHandle, Abortable},
    StreamExt,
};
use futures_channel::{
    mpsc::{unbounded, UnboundedReceiver, UnboundedSender},
    oneshot,
};
use std::{sync::Arc, thread, time::Duration};
use tokio_retry::strategy::ExponentialBackoff;

pub type Sender<T> = UnboundedSender<T>;
pub type Receiver<T> = UnboundedReceiver<T>;

pub struct RandManager<S: Share, P: Proof<Share = S>, D: AugmentedData, Storage> {
    author: Author,
    epoch_state: Arc<EpochState>,
    stop: bool,
    config: RandConfig,
    reliable_broadcast: Arc<ReliableBroadcast<RandMessage<S, D>, ExponentialBackoff>>,
    network_sender: Arc<NetworkSender>,

    // downstream channels
    outgoing_blocks: Sender<OrderedBlocks>,
    // local state
    rand_store: Arc<Mutex<RandStore<S, P, Storage>>>,
    aug_data_store: AugDataStore<D, Storage>,
}

impl<
        S: Share,
        P: Proof<Share = S>,
        D: AugmentedData,
        Storage: RandStorage<S, P> + AugDataStorage<D>,
    > RandManager<S, P, D, Storage>
{
    pub fn new(
        author: Author,
        epoch_state: Arc<EpochState>,
        signer: Arc<ValidatorSigner>,
        config: RandConfig,
        outgoing_blocks: Sender<OrderedBlocks>,
        network_sender: Arc<NetworkSender>,
        db: Arc<Storage>,
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
            BoundedExecutor::new(8, tokio::runtime::Handle::current()),
        ));
        let rand_store = Arc::new(Mutex::new(RandStore::new(
            epoch_state.epoch,
            author,
            config.clone(),
            db.clone(),
        )));
        let aug_data_store = AugDataStore::new(epoch_state.epoch, signer, config.clone(), db);

        Self {
            author,
            epoch_state,
            stop: false,
            config,
            reliable_broadcast,
            network_sender,

            outgoing_blocks,

            rand_store,
            aug_data_store,
        }
    }

    fn process_incoming_blocks(&self, blocks: OrderedBlocks) {
        let broadcast_handles: Vec<_> = blocks
            .ordered_blocks
            .iter()
            .map(|block| {
                RandMetadata::new(
                    block.epoch(),
                    block.round(),
                    block.id(),
                    block.timestamp_usecs(),
                )
            })
            .map(|metadata| self.process_incoming_metadata(metadata))
            .collect();
        let queue_item = QueueItem::new(blocks, Some(broadcast_handles));
        self.rand_store.lock().add_blocks(queue_item);
    }

    fn process_incoming_metadata(&self, metadata: RandMetadata) -> DropGuard {
        let self_share = S::generate(&self.config, metadata.clone());
        observe_block(metadata.timestamp, BlockStage::RAND_BC_SHARE);
        info!(LogSchema::new(LogEvent::BroadcastRandShare)
            .author(self.author)
            .round(metadata.round()));
        self.network_sender.broadcast_without_self(
            RandMessage::<S, D>::Share(self_share.clone()).into_network_message(),
        );
        self.rand_store
            .lock()
            .add_share(self_share)
            .expect("Add self share should succeed");
        self.aggregate_shares_task(metadata)
    }

    fn process_ready_blocks(&mut self, ready_blocks: Vec<OrderedBlocks>) {
        for blocks in ready_blocks {
            for block in blocks.ordered_blocks.iter() {
                observe_block(block.timestamp_usecs(), BlockStage::RAND_READY);
            }
            let _ = self.outgoing_blocks.unbounded_send(blocks);
        }
    }

    fn process_reset(&mut self, request: ResetRequest) {
        let ResetRequest { tx, signal } = request;
        let target_round = match signal {
            ResetSignal::Stop => 0,
            ResetSignal::TargetRound(round) => round,
        };
        self.rand_store.lock().reset(target_round);
        self.stop = matches!(signal, ResetSignal::Stop);
        let _ = tx.send(ResetAck::default());
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
        mut incoming_rpc_request: aptos_channel::Receiver<Author, IncomingRandGenRequest>,
        verified_msg_tx: UnboundedSender<RpcRequest<S, D>>,
        rand_config: RandConfig,
        bounded_executor: BoundedExecutor,
    ) {
        while let Some(rand_gen_msg) = incoming_rpc_request.next().await {
            let tx = verified_msg_tx.clone();
            let epoch_state_clone = epoch_state.clone();
            let config_clone = rand_config.clone();
            bounded_executor
                .spawn(async move {
                    match bcs::from_bytes::<RandMessage<S, D>>(rand_gen_msg.req.data()) {
                        Ok(msg) => {
                            match msg.verify(&epoch_state_clone, &config_clone, rand_gen_msg.sender)
                            {
                                Ok(_) => {
                                    let _ = tx.unbounded_send(RpcRequest {
                                        req: msg,
                                        protocol: rand_gen_msg.protocol,
                                        response_sender: rand_gen_msg.response_sender,
                                    });
                                },
                                Err(e) => warn!("Invalid rand gen message: {}", e),
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

    fn aggregate_shares_task(&self, metadata: RandMetadata) -> DropGuard {
        let rb = self.reliable_broadcast.clone();
        let rand_store = self.rand_store.clone();
        let epoch_state = self.epoch_state.clone();
        let aggregate_state = Arc::new(ShareAggregateState::new(
            self.rand_store.clone(),
            metadata.clone(),
            self.config.clone(),
        ));
        let request = RequestShare::new(self.epoch_state.epoch, metadata.clone());
        let task = async move {
            // Delay request reactive shares in case proactive shares come in
            tokio::time::sleep(Duration::from_millis(300)).await;
            let maybe_existing_authors = rand_store.lock().get_all_shares_author(&metadata);
            if let Some(existing_authors) = maybe_existing_authors {
                let targets: Vec<_> = epoch_state
                    .verifier
                    .get_ordered_account_addresses_iter()
                    .filter(|addr| !existing_authors.contains(addr))
                    .collect();
                info!(
                    epoch = metadata.epoch(),
                    round = metadata.round(),
                    "[RandManager] Start broadcasting share request for {}",
                    targets.len(),
                );
                rb.multicast(request, aggregate_state, targets).await;
                info!(
                    epoch = metadata.epoch(),
                    round = metadata.round(),
                    "[RandManager] Finish broadcasting share request",
                );
            }
        };
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        tokio::spawn(Abortable::new(task, abort_registration));
        DropGuard::new(abort_handle)
    }

    fn broadcast_aug_data(&self) -> DropGuard {
        let data = D::generate(&self.config);
        let aug_ack = Arc::new(AugDataCertBuilder::new(
            data.clone(),
            self.epoch_state.clone(),
        ));
        let rb = self.reliable_broadcast.clone();
        let rb2 = self.reliable_broadcast.clone();
        let first_phase = async move {
            info!(LogSchema::new(LogEvent::BroadcastAugData)
                .author(data.author())
                .epoch(data.epoch()));
            let data = rb.broadcast(data, aug_ack).await;
            info!("[RandManager] Finish broadcasting aug data");
            data
        };
        let validators = self.epoch_state.verifier.get_ordered_account_addresses();
        let task = async move {
            let cert = first_phase.await;
            info!(LogSchema::new(LogEvent::BroadcastCertifiedAugData)
                .author(cert.author())
                .epoch(cert.epoch()));
            let ack_state = Arc::new(CertifiedAugDataAckState::new(validators.into_iter()));
            info!("[RandManager] Start broadcasting certified aug data");
            rb2.broadcast(cert, ack_state).await;
            info!("[RandManager] Finish broadcasting certified aug data");
        };
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
    ) {
        info!("RandManager started");
        let (verified_msg_tx, mut verified_msg_rx) = unbounded();
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

        let _guard = self.broadcast_aug_data();

        while !self.stop {
            tokio::select! {
                Some(blocks) = incoming_blocks.next() => {
                    self.process_incoming_blocks(blocks);
                }
                Some(reset) = reset_rx.next() => {
                    while incoming_blocks.try_next().is_ok() {}
                    self.process_reset(reset);
                }
                Some(request) = verified_msg_rx.next() => {
                    let RpcRequest {
                        req: rand_gen_msg,
                        protocol,
                        response_sender,
                    } = request;
                    match rand_gen_msg {
                        RandMessage::RequestShare(request) => {
                            if let Some(share) = self.rand_store.lock().get_self_share(request.rand_metadata()) {
                                self.process_response(protocol, response_sender, RandMessage::Share(share));
                            }
                        }
                        RandMessage::Share(share) => {
                            info!(
                                LogSchema::new(LogEvent::ReceiveProactiveRandShare)
                                    .author(self.author)
                                    .round(share.metadata().round())
                                    .remote_peer(*share.author())
                            );

                            if let Err(e) = self.rand_store.lock().add_share(share) {
                                warn!("[RandManager] Failed to add share: {}", e);
                            }
                        }
                        RandMessage::AugData(aug_data) => {
                            info!(
                                LogSchema::new(LogEvent::ReceiveAugData)
                                    .author(self.author)
                                    .epoch(aug_data.epoch())
                                    .remote_peer(aug_data.author())
                            );
                            match self.aug_data_store.add_aug_data(aug_data) {
                                Ok(sig) => self.process_response(protocol, response_sender, RandMessage::AugDataSignature(sig)),
                                Err(e) => error!("[RandManager] Failed to add aug data: {}", e),
                            }
                        }
                        RandMessage::CertifiedAugData(certified_aug_data) => {
                            info!(
                                LogSchema::new(LogEvent::ReceiveCertifiedAugData)
                                    .author(self.author)
                                    .epoch(certified_aug_data.epoch())
                                    .remote_peer(certified_aug_data.author())
                            );
                            match self.aug_data_store.add_certified_aug_data(certified_aug_data) {
                                Ok(ack) => self.process_response(protocol, response_sender, RandMessage::CertifiedAugDataAck(ack)),
                                Err(e) => error!("[RandManager] Failed to add certified aug data: {}", e),
                            }
                        }
                        _ => unreachable!("[RandManager] Unexpected message type after verification"),
                    }
                }
            }
            let maybe_ready_blocks = self.rand_store.lock().try_dequeue_rand_ready_prefix();
            if let Some(ready_blocks) = maybe_ready_blocks {
                self.process_ready_blocks(ready_blocks);
            }
        }
        info!("RandManager stopped");
    }
}
