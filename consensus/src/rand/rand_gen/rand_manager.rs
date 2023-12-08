// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    network::{IncomingRandGenRequest, NetworkSender},
    pipeline::buffer_manager::{OrderedBlocks, ResetRequest},
    rand::rand_gen::{
        network_messages::{RandMessage, RpcRequest},
        rand_store::RandStore,
        types::{AugmentedData, Proof, RandConfig, RandDecision, Share},
    },
};
use aptos_bounded_executor::BoundedExecutor;
use aptos_consensus_types::common::Author;
use aptos_logger::{info, spawn_named, warn};
use aptos_reliable_broadcast::ReliableBroadcast;
use aptos_time_service::TimeService;
use aptos_types::{epoch_state::EpochState, validator_signer::ValidatorSigner};
use std::{sync::Arc, time::Duration};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_retry::strategy::ExponentialBackoff;

pub type Sender<T> = UnboundedSender<T>;
pub type Receiver<T> = UnboundedReceiver<T>;

pub struct RandManager<S: Share, P: Proof<Share = S>, D: AugmentedData> {
    author: Author,
    epoch_state: Arc<EpochState>,
    stop: bool,
    signer: Arc<ValidatorSigner>,
    config: RandConfig,
    reliable_broadcast: Arc<ReliableBroadcast<RandMessage<S, P, D>, ExponentialBackoff>>,

    // local channels
    rand_decision_tx: Sender<RandDecision<P>>,
    rand_decision_rx: Receiver<RandDecision<P>>,

    // downstream channels
    outgoing_blocks: Sender<OrderedBlocks>,
}

impl<S: Share, P: Proof<Share = S>, D: AugmentedData> RandManager<S, P, D> {
    pub fn new(
        author: Author,
        epoch_state: Arc<EpochState>,
        signer: Arc<ValidatorSigner>,
        config: RandConfig,
        outgoing_blocks: Sender<OrderedBlocks>,
        network_sender: Arc<NetworkSender>,
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

        Self {
            author,
            epoch_state,
            stop: false,
            signer,
            config,
            reliable_broadcast,

            rand_decision_tx,
            rand_decision_rx,
            outgoing_blocks,
        }
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
                                if msg.verify(&epoch_state_clone, &config_clone).is_ok() {
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
                Some(_blocks) = incoming_blocks.recv() => {

                }
                Some(_request) = verified_msg_rx.recv() => {

                }
                Some(_reset) = reset_rx.recv() => {
                    self.stop = true;
                }
            }
        }
        info!("RandManager stopped");
    }
}
