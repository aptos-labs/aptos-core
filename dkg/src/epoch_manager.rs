// Copyright © Aptos Foundation

use crate::{
    dkg_manager::{agg_trx_producer::RealAggTranscriptProducer, DKGManager},
    dummy_dkg::DummyDKG,
    network::{IncomingRpcRequest, NetworkReceivers, NetworkSender},
    network_interface::DKGNetworkClient,
    DKGMessage,
};
use anyhow::Result;
use aptos_bounded_executor::BoundedExecutor;
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_config::config::IdentityBlob;
use aptos_event_notifications::{
    EventNotification, EventNotificationListener, ReconfigNotification,
    ReconfigNotificationListener,
};
use aptos_logger::error;
use aptos_network::{application::interface::NetworkClient, protocols::network::Event};
use aptos_reliable_broadcast::ReliableBroadcast;
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
use std::{sync::Arc, time::Duration};
use tokio_retry::strategy::ExponentialBackoff;

#[allow(dead_code)]
pub struct EpochManager<P: OnChainConfigProvider> {
    // Some useful metadata
    my_addr: AccountAddress,
    epoch_state: Option<Arc<EpochState>>,

    // some DKG private params
    identity_blob: Arc<IdentityBlob>,

    // Inbound events
    reconfig_events: ReconfigNotificationListener<P>,
    dkg_start_events: EventNotificationListener,
    vtxn_pull_notification_rx_from_pool: vtxn_pool::PullNotificationReceiver,

    // Msgs to DKG manager
    dkg_rpc_msg_tx: Option<aptos_channel::Sender<(), (AccountAddress, IncomingRpcRequest)>>,
    dkg_manager_close_tx: Option<oneshot::Sender<oneshot::Sender<()>>>,
    dkg_start_event_tx: Option<aptos_channel::Sender<(), DKGStartEvent>>,
    vtxn_pull_notification_tx_to_dkgmgr: Option<vtxn_pool::PullNotificationSender>,
    vtxn_pool_write_cli: Arc<vtxn_pool::SingleTopicWriteClient>,

    // Network utils
    self_sender: aptos_channels::Sender<Event<DKGMessage>>,
    network_sender: DKGNetworkClient<NetworkClient<DKGMessage>>,
}

impl<P: OnChainConfigProvider> EpochManager<P> {
    pub fn new(
        my_addr: AccountAddress,
        identity_blob: Arc<IdentityBlob>,
        reconfig_events: ReconfigNotificationListener<P>,
        dkg_start_events: EventNotificationListener,
        self_sender: aptos_channels::Sender<Event<DKGMessage>>,
        network_sender: DKGNetworkClient<NetworkClient<DKGMessage>>,
        vtxn_pool_write_cli: vtxn_pool::SingleTopicWriteClient,
        vtxn_pull_notification_rx: vtxn_pool::PullNotificationReceiver,
    ) -> Self {
        Self {
            my_addr,
            identity_blob,
            epoch_state: None,
            reconfig_events,
            dkg_start_events,
            dkg_rpc_msg_tx: None,
            dkg_manager_close_tx: None,
            self_sender,
            network_sender,
            vtxn_pool_write_cli: Arc::new(vtxn_pool_write_cli),
            vtxn_pull_notification_rx_from_pool: vtxn_pull_notification_rx,
            vtxn_pull_notification_tx_to_dkgmgr: None,
            dkg_start_event_tx: None,
        }
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
                notification = self.dkg_start_events.select_next_some() => {
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

        let epoch_state = Arc::new(EpochState {
            epoch: payload.epoch(),
            verifier: (&validator_set).into(),
        });
        self.epoch_state = Some(epoch_state.clone());

        let features = payload.get::<Features>().unwrap_or_default();

        if features.is_enabled(FeatureFlag::RECONFIGURE_WITH_DKG) {
            let DKGState {
                in_progress: in_progress_session,
                ..
            } = payload.get::<DKGState>().unwrap_or_default();

            let network_sender = self.create_network_sender();
            let rb = ReliableBroadcast::new(
                epoch_state.verifier.get_ordered_account_addresses(),
                Arc::new(network_sender),
                ExponentialBackoff::from_millis(5),
                aptos_time_service::TimeService::real(),
                Duration::from_millis(1000),
                BoundedExecutor::new(8, tokio::runtime::Handle::current()),
            );
            let agg_trx_producer = RealAggTranscriptProducer::new(rb);

            let (dkg_start_event_tx, dkg_start_event_rx) =
                aptos_channel::new(QueueStyle::KLAST, 1, None);
            self.dkg_start_event_tx = Some(dkg_start_event_tx);

            let (dkg_rpc_msg_tx, dkg_rpc_msg_rx) = aptos_channel::new::<
                (),
                (AccountAddress, IncomingRpcRequest),
            >(QueueStyle::FIFO, 100, None);
            self.dkg_rpc_msg_tx = Some(dkg_rpc_msg_tx);
            let (dkg_manager_close_tx, dkg_manager_close_rx) = oneshot::channel();
            self.dkg_manager_close_tx = Some(dkg_manager_close_tx);

            let dkg_manager = DKGManager::<DummyDKG, _>::new(
                self.identity_blob.clone(),
                self.my_addr,
                epoch_state,
                Arc::new(agg_trx_producer),
                self.vtxn_pool_write_cli.clone(),
            );
            let (vtxn_pull_notification_tx, vtxn_pull_notification_rx) =
                aptos_channel::new(QueueStyle::KLAST, 1, None);
            self.vtxn_pull_notification_tx_to_dkgmgr = Some(vtxn_pull_notification_tx);
            tokio::spawn(dkg_manager.run(
                in_progress_session,
                dkg_start_event_rx,
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

    fn create_network_sender(&self) -> NetworkSender {
        NetworkSender::new(
            self.my_addr,
            self.network_sender.clone(),
            self.self_sender.clone(),
        )
    }
}
