// Copyright © Aptos Foundation

use crate::{
    dkg_manager::{agg_node_producer::DummyAggNodeProducer, DKGManager},
    network::{IncomingRpcRequest, NetworkReceivers},
    network_interface::DKGNetworkClient,
    DKGMessage,
};
use anyhow::Result;
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_config::config::NodeConfig;
use aptos_crypto::bls12381;
use aptos_event_notifications::{
    EventNotification, EventNotificationListener, ReconfigNotification,
    ReconfigNotificationListener,
};
use aptos_logger::error;
use aptos_network::{application::interface::NetworkClient, protocols::network::Event};
use aptos_types::{
    account_address::AccountAddress,
    dkg::{DKGStartEvent, DKGState},
    epoch_state::EpochState,
    on_chain_config::{
        FeatureFlag, Features, OnChainConfigPayload, OnChainConfigProvider, ValidatorSet,
    },
    validator_txn::ValidatorTransaction,
};
use aptos_validator_transaction_pool as vtxn_pool;
use futures::StreamExt;
use futures_channel::oneshot;
use std::sync::Arc;

#[allow(dead_code)]
pub struct EpochManager<P: OnChainConfigProvider> {
    sk: Option<bls12381::PrivateKey>,
    my_addr: AccountAddress,
    epoch_state: Option<Arc<EpochState>>,
    reconfig_events: ReconfigNotificationListener<P>,
    start_dkg_events: EventNotificationListener,
    dkg_rpc_msg_tx: Option<aptos_channel::Sender<(), (AccountAddress, IncomingRpcRequest)>>,
    dkg_manager_close_tx: Option<oneshot::Sender<oneshot::Sender<()>>>,

    self_sender: aptos_channels::Sender<Event<DKGMessage>>,
    network_sender: DKGNetworkClient<NetworkClient<DKGMessage>>,
    start_dkg_event_tx: Option<aptos_channel::Sender<(), DKGStartEvent>>,
    vtxn_pool_write_cli: Arc<vtxn_pool::SingleTopicWriteClient>,
    vtxn_pull_notification_rx_from_pool: vtxn_pool::PullNotificationReceiver,
    vtxn_pull_notification_tx_to_dkgmgr: Option<vtxn_pool::PullNotificationSender>,
}

impl<P: OnChainConfigProvider> EpochManager<P> {
    pub fn new(
        node_config: &NodeConfig,
        reconfig_events: ReconfigNotificationListener<P>,
        start_dkg_events: EventNotificationListener,
        self_sender: aptos_channels::Sender<Event<DKGMessage>>,
        network_sender: DKGNetworkClient<NetworkClient<DKGMessage>>,
        vtxn_pool_write_cli: vtxn_pool::SingleTopicWriteClient,
        vtxn_pull_notification_rx: vtxn_pool::PullNotificationReceiver,
    ) -> Self {
        let my_addr = node_config.validator_network.as_ref().unwrap().peer_id();
        Self {
            sk: None, //TODO: load from storage
            my_addr,
            epoch_state: None,
            reconfig_events,
            start_dkg_events,
            dkg_rpc_msg_tx: None,
            dkg_manager_close_tx: None,
            self_sender,
            network_sender,
            vtxn_pool_write_cli: Arc::new(vtxn_pool_write_cli),
            vtxn_pull_notification_rx_from_pool: vtxn_pull_notification_rx,
            vtxn_pull_notification_tx_to_dkgmgr: None,
            start_dkg_event_tx: None,
        }
    }

    fn epoch_state(&self) -> &EpochState {
        self.epoch_state
            .as_ref()
            .expect("EpochManager not started yet")
    }

    fn process_rpc_request(
        &mut self,
        _peer_id: AccountAddress,
        _dkg_request: IncomingRpcRequest,
    ) -> Result<()> {
        //TODO
        Ok(())
    }

    fn on_dkg_start_notification(&mut self, _notification: EventNotification) -> Result<()> {
        //TODO
        Ok(())
    }

    fn process_vtxn_pull_notification(
        &mut self,
        _pulled_txn: Arc<ValidatorTransaction>,
    ) -> Result<()> {
        //TODO
        Ok(())
    }

    pub async fn start(mut self, mut network_receivers: NetworkReceivers) {
        self.await_reconfig_notification().await;
        loop {
            let handling_result = tokio::select! {
                notification = self.start_dkg_events.select_next_some() => {
                    self.on_dkg_start_notification(notification)
                },
                reconfig_notification = self.reconfig_events.select_next_some() => {
                    self.on_new_epoch(reconfig_notification).await
                },
                (peer, rpc_request) = network_receivers.rpc_rx.select_next_some() => {
                    self.process_rpc_request(peer, rpc_request)
                },
                msg = self.vtxn_pull_notification_rx_from_pool.select_next_some() => {
                    self.process_vtxn_pull_notification(msg)
                }
            };

            if let Err(e) = handling_result {
                error!("{}", e);
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
        let validator_set: ValidatorSet = payload
            .get()
            .expect("failed to get ValidatorSet from payload");

        let epoch_state = EpochState {
            epoch: payload.epoch(),
            verifier: (&validator_set).into(),
        };
        self.epoch_state = Some(Arc::new(epoch_state.clone()));

        let features = payload.get::<Features>().unwrap_or_default();

        if features.is_enabled(FeatureFlag::RECONFIGURE_WITH_DKG) {
            let DKGState {
                in_progress: in_progress_session,
                ..
            } = payload.get::<DKGState>().unwrap_or_default();

            let agg_node_producer = DummyAggNodeProducer {}; //TODO: replace with real

            let (start_dkg_event_tx, start_dkg_event_rx) =
                aptos_channel::new(QueueStyle::KLAST, 1, None);
            self.start_dkg_event_tx = Some(start_dkg_event_tx);

            let (dkg_rpc_msg_tx, dkg_rpc_msg_rx) = aptos_channel::new::<
                (),
                (AccountAddress, IncomingRpcRequest),
            >(QueueStyle::FIFO, 100, None);
            self.dkg_rpc_msg_tx = Some(dkg_rpc_msg_tx);
            let (dkg_manager_close_tx, dkg_manager_close_rx) = oneshot::channel();
            self.dkg_manager_close_tx = Some(dkg_manager_close_tx);

            let dkg_manager = DKGManager::new(
                self.my_addr,
                self.epoch_state().clone(),
                Arc::new(agg_node_producer),
                self.vtxn_pool_write_cli.clone(),
            );
            let (vtxn_pull_notification_tx, vtxn_pull_notification_rx) =
                aptos_channel::new(QueueStyle::KLAST, 1, None);
            self.vtxn_pull_notification_tx_to_dkgmgr = Some(vtxn_pull_notification_tx);
            tokio::spawn(dkg_manager.run(
                in_progress_session,
                start_dkg_event_rx,
                dkg_rpc_msg_rx,
                vtxn_pull_notification_rx,
                dkg_manager_close_rx,
            ));
        }
    }

    async fn on_new_epoch(&mut self, reconfig_notification: ReconfigNotification<P>) -> Result<()> {
        self.shutdown_current_processor().await;
        self.start_new_epoch(reconfig_notification.on_chain_configs)
            .await;
        Ok(())
    }

    async fn shutdown_current_processor(&mut self) {
        if let Some(tx) = self.dkg_manager_close_tx.take() {
            let (ack_tx, ack_rx) = oneshot::channel();
            tx.send(ack_tx).unwrap();
            ack_rx.await.unwrap();
        }
    }
}
