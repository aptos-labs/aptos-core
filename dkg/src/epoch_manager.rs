// Copyright © Aptos Foundation

use crate::{
    dkg_manager::DKGManager,
    network::{IncomingDKGRequest, IncomingRpcRequest, NetworkReceivers, NetworkSender},
    network_interface::{DKGMsg, DKGNetworkClient},
};
use anyhow::anyhow;
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_config::config::{NodeConfig, SecureBackend};
use aptos_consensus_types::common::Author;
use aptos_crypto::bls12381;
use aptos_event_notifications::{
    EventNotification, EventNotificationListener, ReconfigNotification,
    ReconfigNotificationListener,
};
use aptos_global_constants::CONSENSUS_KEY;
use aptos_logger::{debug, error};
use aptos_network::{application::interface::NetworkClient, protocols::network::Event};
use aptos_reliable_broadcast::ReliableBroadcast;
use aptos_secure_storage::{KVStorage, Storage};
use aptos_types::{
    dkg::StartDKGEvent,
    epoch_state::EpochState,
    on_chain_config::{
        FeatureFlag, Features, OnChainConfigPayload, OnChainConfigProvider, ValidatorSet,
    },
    validator_signer::ValidatorSigner,
};
use aptos_validator_transaction_pool as vtxn_pool;
use futures::StreamExt;
use futures_channel::oneshot;
use move_core_types::account_address::AccountAddress;
use std::{sync::Arc, time::Duration};
use tokio_retry::strategy::ExponentialBackoff;

pub struct EpochManager<P: OnChainConfigProvider> {
    node_config: NodeConfig,
    author: AccountAddress,
    epoch_state: Option<Arc<EpochState>>,
    reconfig_events: ReconfigNotificationListener<P>,
    start_dkg_events: EventNotificationListener,
    dkg_rpc_msg_tx: Option<aptos_channel::Sender<u64, (AccountAddress, IncomingDKGRequest)>>,
    dkg_manager_close_tx: Option<oneshot::Sender<oneshot::Sender<()>>>,

    self_sender: aptos_channels::Sender<Event<DKGMsg>>,
    network_sender: DKGNetworkClient<NetworkClient<DKGMsg>>,
    start_dkg_event_tx: Option<aptos_channel::Sender<u64, StartDKGEvent>>,
    dkg_txn_writer: Arc<vtxn_pool::WriteClient>,
    dkg_txn_pulled_rx_from_pool: vtxn_pool::NotificationReceiver,
    dkg_txn_pulled_tx_to_dkg_mgr: Option<vtxn_pool::NotificationSender>,
}

impl<P: OnChainConfigProvider> EpochManager<P> {
    pub fn new(
        node_config: &NodeConfig,
        reconfig_events: ReconfigNotificationListener<P>,
        start_dkg_events: EventNotificationListener,
        self_sender: aptos_channels::Sender<Event<DKGMsg>>,
        network_sender: DKGNetworkClient<NetworkClient<DKGMsg>>,
        dkg_txn_writer: vtxn_pool::WriteClient,
        dkg_pulled_rx: vtxn_pool::NotificationReceiver,
    ) -> Self {
        let author = node_config.validator_network.as_ref().unwrap().peer_id();
        Self {
            node_config: node_config.clone(),
            author,
            epoch_state: None,
            reconfig_events,
            start_dkg_events,
            dkg_rpc_msg_tx: None,
            dkg_manager_close_tx: None,
            self_sender,
            network_sender,
            dkg_txn_writer: Arc::new(dkg_txn_writer),
            dkg_txn_pulled_rx_from_pool: dkg_pulled_rx,
            dkg_txn_pulled_tx_to_dkg_mgr: None,
            start_dkg_event_tx: None,
        }
    }

    fn epoch_state(&self) -> &EpochState {
        self.epoch_state
            .as_ref()
            .expect("EpochManager not started yet")
    }

    fn epoch(&self) -> u64 {
        self.epoch_state().epoch
    }

    fn process_rpc_request(
        &mut self,
        peer_id: Author,
        request: IncomingRpcRequest,
    ) -> anyhow::Result<()> {
        match request {
            IncomingRpcRequest::DKG(dkg_request) => {
                let ret = if dkg_request.req.epoch == self.epoch() {
                    debug!("[DKG] EpochManager: request is for current epoch.");
                    if let Some(tx) = &self.dkg_rpc_msg_tx {
                        debug!(
                            "[DKG] EpochManager::process_rpc_request: forwarding to DKGManager."
                        );
                        tx.push(0, (peer_id, dkg_request))
                    } else {
                        Err(anyhow!("DKGManager not started."))
                    }
                } else {
                    debug!(
                        "[DKG] EpochManager::process_rpc_request: request is for another epoch."
                    );
                    Err(anyhow!("request isi for another epoch."))
                    // self.process_different_epoch(req.req.epoch, peer_id)
                };
                debug!("[DKG] EpochManager::process_rpc_request: END");
                ret
            },
        }
    }

    async fn on_start_dkg_notification(&mut self, notification: EventNotification) {
        let EventNotification {
            subscribed_events, ..
        } = notification;
        for event in subscribed_events {
            let start_dkg_event = StartDKGEvent::try_from(&event).unwrap();
            if let Some(tx) = self.start_dkg_event_tx.as_ref() {
                tx.push(0, start_dkg_event).unwrap();
            }
        }
    }

    pub async fn start(mut self, mut network_receivers: NetworkReceivers) {
        self.await_reconfig_notification().await;
        loop {
            tokio::select! {
                notification = self.start_dkg_events.select_next_some() => {
                    self.on_start_dkg_notification(notification).await;
                },
                reconfig_notification = self.reconfig_events.select_next_some() => {
                    self.on_new_epoch(reconfig_notification).await;
                },
                (peer, rpc_request) = network_receivers.rpc_rx.select_next_some() => {
                    if let Err(e) = self.process_rpc_request(peer, rpc_request) {
                        error!("error={}", e);
                    }
                },
                msg = self.dkg_txn_pulled_rx_from_pool.select_next_some() => {
                    if let Some(tx) = self.dkg_txn_pulled_tx_to_dkg_mgr.as_mut() {
                        tx.push((), msg).unwrap(); // Forward to DKGManager.
                    }
                }
            }
        }
    }

    async fn await_reconfig_notification(&mut self) {
        let reconfig_notification = self
            .reconfig_events
            .next()
            .await
            .expect("Reconfig sender dropped, unable to start new epoch");
        self.start_new_epoch(reconfig_notification.on_chain_configs)
            .await;
    }

    async fn start_new_epoch(&mut self, payload: OnChainConfigPayload<P>) {
        debug!("[DKG] start_new_epoch: BEGIN");
        let validator_set: ValidatorSet = payload
            .get()
            .expect("failed to get ValidatorSet from payload");

        let epoch_state = EpochState {
            epoch: payload.epoch(),
            verifier: (&validator_set).into(),
        };
        self.epoch_state = Some(Arc::new(epoch_state.clone()));
        debug!("[DKG] start_new_epoch: new_epoch={}", epoch_state.epoch);

        let features = payload.get::<Features>().unwrap_or_default();

        if features.is_enabled(FeatureFlag::RECONFIGURE_WITH_DKG) {
            debug!(
                "[DKG] DKG manager init, epoch={}",
                self.epoch_state.as_ref().unwrap().epoch
            );
            let network_sender = self.create_network_sender(&epoch_state);
            let dkg_rb = Arc::new(ReliableBroadcast::new(
                epoch_state.verifier.get_ordered_account_addresses(),
                Arc::new(network_sender),
                ExponentialBackoff::from_millis(5),
                aptos_time_service::TimeService::real(),
                Duration::from_millis(1000),
            ));

            let (start_dkg_event_tx, start_dkg_event_rx) =
                aptos_channel::new(QueueStyle::KLAST, 1, None);
            self.start_dkg_event_tx = Some(start_dkg_event_tx);

            let (dkg_rpc_msg_tx, dkg_rpc_msg_rx) = aptos_channel::new::<
                u64,
                (AccountAddress, IncomingDKGRequest),
            >(QueueStyle::FIFO, 100, None);
            self.dkg_rpc_msg_tx = Some(dkg_rpc_msg_tx);
            let (dkg_manager_close_tx, dkg_manager_close_rx) = oneshot::channel();
            self.dkg_manager_close_tx = Some(dkg_manager_close_tx);

            let dkg_manager = DKGManager::new(
                self.author,
                self.epoch_state().clone(),
                self.private_key(),
                dkg_rb,
                self.dkg_txn_writer.clone(),
            );
            let (tx, dkg_txn_pulled_rx) = aptos_channel::new(QueueStyle::KLAST, 1, None);
            self.dkg_txn_pulled_tx_to_dkg_mgr = Some(tx);
            tokio::spawn(dkg_manager.run(
                start_dkg_event_rx,
                dkg_rpc_msg_rx,
                dkg_txn_pulled_rx,
                dkg_manager_close_rx,
            ));
        }
        debug!("[DKG] start_new_epoch: END");
    }

    async fn on_new_epoch(&mut self, reconfig_notification: ReconfigNotification<P>) {
        debug!("[DKG] on_new_epoch: BEGIN");
        self.shutdown_current_processor().await;
        self.start_new_epoch(reconfig_notification.on_chain_configs)
            .await;
        debug!("[DKG] on_new_epoch: END");
    }

    async fn shutdown_current_processor(&mut self) {
        debug!("[DKG] shutdown_current_processor: BEGIN");
        if let Some(tx) = self.dkg_manager_close_tx.take() {
            let (ack_tx, ack_rx) = oneshot::channel();
            tx.send(ack_tx).unwrap();
            ack_rx.await.unwrap();
        }

        debug!("[DKG] shutdown_current_processor: END");
    }

    fn create_network_sender(&mut self, epoch_state: &EpochState) -> NetworkSender {
        NetworkSender::new(
            self.author,
            self.network_sender.clone(),
            self.self_sender.clone(),
            epoch_state.verifier.clone(),
        )
    }

    fn private_key(&self) -> bls12381::PrivateKey {
        // get private key as signing key for PVSS
        let backend = &self.node_config.consensus.safety_rules.backend;
        let storage: Storage = backend.try_into().expect("Unable to initialize storage");
        if let Err(error) = storage.available() {
            panic!("Storage is not available: {:?}", error);
        }
        let private_key: bls12381::PrivateKey = storage
            .get(CONSENSUS_KEY)
            .map(|v| v.value)
            .expect("Unable to get private key");

        private_key
    }
}

#[allow(dead_code)]
fn new_signer_from_storage(author: Author, backend: &SecureBackend) -> Arc<ValidatorSigner> {
    let storage: Storage = backend.try_into().expect("Unable to initialize storage");
    if let Err(error) = storage.available() {
        panic!("Storage is not available: {:?}", error);
    }
    let private_key = storage
        .get(CONSENSUS_KEY)
        .map(|v| v.value)
        .expect("Unable to get private key");
    Arc::new(ValidatorSigner::new(author, private_key))
}
