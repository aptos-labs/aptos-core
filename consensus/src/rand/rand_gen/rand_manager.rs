// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    network::{IncomingRandGenRequest, NetworkSender, TConsensusMsg},
    pipeline::buffer_manager::{OrderedBlocks, ResetAck, ResetRequest},
    rand::rand_gen::{
        aug_data_store::AugDataStore,
        block_queue::QueueItem,
        network_messages::{RandMessage, RpcRequest},
        rand_store::RandStore,
        storage::interface::{AugDataStorage, RandStorage},
        types::{AugmentedData, Proof, RandConfig, RandDecision, Share},
    },
};
use aptos_bounded_executor::BoundedExecutor;
use aptos_consensus_types::common::Author;
use aptos_logger::{error, info, spawn_named, warn};
use aptos_network::{protocols::network::RpcError, ProtocolId};
use aptos_reliable_broadcast::ReliableBroadcast;
use aptos_time_service::TimeService;
use aptos_types::{epoch_state::EpochState, validator_signer::ValidatorSigner};
use bytes::Bytes;
use futures_channel::oneshot;
use std::{sync::Arc, time::Duration};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_retry::strategy::ExponentialBackoff;

pub type Sender<T> = UnboundedSender<T>;
pub type Receiver<T> = UnboundedReceiver<T>;

pub struct RandManager<S: Share, P: Proof<Share = S>, D: AugmentedData, Storage> {
    author: Author,
    epoch_state: Arc<EpochState>,
    stop: bool,
    config: RandConfig,
    reliable_broadcast: Arc<ReliableBroadcast<RandMessage<S, P, D>, ExponentialBackoff>>,

    // local channels
    rand_decision_tx: Sender<RandDecision<P>>,
    rand_decision_rx: Receiver<RandDecision<P>>,

    // downstream channels
    outgoing_blocks: Sender<OrderedBlocks>,
    // local state
    rand_store: RandStore<S, P, Storage>,
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
        let (rand_decision_tx, rand_decision_rx) = tokio::sync::mpsc::unbounded_channel();
        let rb_backoff_policy = ExponentialBackoff::from_millis(2)
            .factor(100)
            .max_delay(Duration::from_secs(10));
        let reliable_broadcast = Arc::new(ReliableBroadcast::new(
            epoch_state.verifier.get_ordered_account_addresses(),
            network_sender.clone(),
            rb_backoff_policy,
            TimeService::real(),
            Duration::from_secs(10),
        ));
        let rand_store = RandStore::new(epoch_state.epoch, author, config.clone(), db.clone());
        let aug_data_store = AugDataStore::new(epoch_state.epoch, signer, config.clone(), db);

        Self {
            author,
            epoch_state,
            stop: false,
            config,
            reliable_broadcast,

            rand_decision_tx,
            rand_decision_rx,
            outgoing_blocks,

            rand_store,
            aug_data_store,
        }
    }

    fn process_incoming_blocks(&mut self, blocks: OrderedBlocks) {
        let queue_item = QueueItem::new(blocks, None);
        self.rand_store.add_blocks(queue_item);
    }

    fn process_ready_blocks(&mut self, ready_blocks: Vec<OrderedBlocks>) {
        for blocks in ready_blocks {
            let _ = self.outgoing_blocks.send(blocks);
        }
    }

    fn process_reset(&mut self, request: ResetRequest) {
        let ResetRequest { tx, stop } = request;
        self.rand_store.reset();
        self.stop = stop;
        let _ = tx.send(ResetAck::default());
    }

    fn process_response(
        &self,
        protocol: ProtocolId,
        sender: oneshot::Sender<Result<Bytes, RpcError>>,
        message: RandMessage<S, P, D>,
    ) {
        let msg = message.into_network_message();
        let _ = sender.send(Ok(protocol.to_bytes(&msg).unwrap().into()));
    }

    pub async fn start(
        mut self,
        mut incoming_blocks: Receiver<OrderedBlocks>,
        mut incoming_rpc_request: Receiver<IncomingRandGenRequest>,
        mut reset_rx: Receiver<ResetRequest>,
        bounded_executor: BoundedExecutor,
    ) {
        info!("RandManager started");
        let (verified_msg_tx, mut verified_msg_rx) = tokio::sync::mpsc::unbounded_channel();
        let epoch_state = self.epoch_state.clone();
        let rand_config = self.config.clone();
        spawn_named!("rand manager verification", async move {
            while let Some(rand_gen_msg) = incoming_rpc_request.recv().await {
                let tx = verified_msg_tx.clone();
                let epoch_state_clone = epoch_state.clone();
                let config_clone = rand_config.clone();
                bounded_executor
                    .spawn(async move {
                        match bcs::from_bytes::<RandMessage<S, P, D>>(rand_gen_msg.req.data()) {
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
        });
        while !self.stop {
            tokio::select! {
                Some(blocks) = incoming_blocks.recv() => {
                    self.process_incoming_blocks(blocks);
                }
                Some(request) = verified_msg_rx.recv() => {
                    let RpcRequest {
                        req: rand_gen_msg,
                        protocol,
                        response_sender,
                    } = request;
                    match rand_gen_msg {
                        RandMessage::Share(share) => {
                            match self.rand_store.add_share(share) {
                                Ok(share_ack) => self.process_response(protocol, response_sender, RandMessage::ShareAck(share_ack)),
                                Err(e) => error!("[RandManager] Failed to add share: {}", e),
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
                Some(decision) = self.rand_decision_rx.recv() => {
                    if let Err(e) = self.rand_store.add_decision(decision) {
                        error!("[RandManager] Failed to add decision: {}", e);
                    }
                }
                Some(reset) = reset_rx.recv() => {
                    while let Ok(_) = incoming_blocks.try_recv() {}
                    self.process_reset(reset);
                }
            }
            if let Some(ready_blocks) = self.rand_store.try_dequeue_rand_ready_prefix() {
                self.process_ready_blocks(ready_blocks);
            }
        }
        info!("RandManager stopped");
    }
}
