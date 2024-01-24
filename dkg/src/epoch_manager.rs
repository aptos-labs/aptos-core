// Copyright © Aptos Foundation

use crate::{
    agg_trx_producer::AggTranscriptProducer,
    dkg_manager::DKGManager,
    network::{IncomingRpcRequest, NetworkReceivers, NetworkSender},
    network_interface::DKGNetworkClient,
    DKGMessage,
};
use anyhow::Result;
use aptos_bounded_executor::BoundedExecutor;
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_event_notifications::{
    EventNotification, EventNotificationListener, ReconfigNotification,
    ReconfigNotificationListener,
};
use aptos_logger::{debug, error};
use aptos_network::{application::interface::NetworkClient, protocols::network::Event};
use aptos_reliable_broadcast::ReliableBroadcast;
use aptos_types::{
    account_address::AccountAddress,
    dkg::{DKGStartEvent, DKGState, DKGTrait, DefaultDKG},
    epoch_state::EpochState,
    on_chain_config::{
        FeatureFlag, Features, OnChainConfigPayload, OnChainConfigProvider, ValidatorSet,
    },
};
use aptos_validator_transaction_pool::VTxnPoolState;
use futures::StreamExt;
use futures_channel::oneshot;
use std::{sync::Arc, time::Duration};
use tokio_retry::strategy::ExponentialBackoff;

pub struct EpochManager<P: OnChainConfigProvider> {
    dkg_dealer_sk: Arc<<DefaultDKG as DKGTrait>::DealerPrivateKey>,
    // Some useful metadata
    my_addr: AccountAddress,
    epoch_state: Option<Arc<EpochState>>,

    // Inbound events
    reconfig_events: ReconfigNotificationListener<P>,
    dkg_start_events: EventNotificationListener,

    // Msgs to DKG manager
    dkg_rpc_msg_tx:
        Option<aptos_channel::Sender<AccountAddress, (AccountAddress, IncomingRpcRequest)>>,
    dkg_manager_close_tx: Option<oneshot::Sender<oneshot::Sender<()>>>,
    dkg_start_event_tx: Option<oneshot::Sender<DKGStartEvent>>,
    vtxn_pool: VTxnPoolState,

    // Network utils
    self_sender: aptos_channels::Sender<Event<DKGMessage>>,
    network_sender: DKGNetworkClient<NetworkClient<DKGMessage>>,
}

impl<P: OnChainConfigProvider> EpochManager<P> {
    pub fn new(
        my_addr: AccountAddress,
        dkg_dealer_sk: <DefaultDKG as DKGTrait>::DealerPrivateKey,
        reconfig_events: ReconfigNotificationListener<P>,
        dkg_start_events: EventNotificationListener,
        self_sender: aptos_channels::Sender<Event<DKGMessage>>,
        network_sender: DKGNetworkClient<NetworkClient<DKGMessage>>,
        vtxn_pool: VTxnPoolState,
    ) -> Self {
        Self {
            dkg_dealer_sk: Arc::new(dkg_dealer_sk),
            my_addr,
            epoch_state: None,
            reconfig_events,
            dkg_start_events,
            dkg_rpc_msg_tx: None,
            dkg_manager_close_tx: None,
            self_sender,
            network_sender,
            vtxn_pool,
            dkg_start_event_tx: None,
        }
    }

    fn process_rpc_request(
        &mut self,
        peer_id: AccountAddress,
        dkg_request: IncomingRpcRequest,
    ) -> Result<()> {
        if Some(dkg_request.msg.epoch()) == self.epoch_state.as_ref().map(|s| s.epoch) {
            // Forward to DKGManager if it is alive.
            if let Some(tx) = &self.dkg_rpc_msg_tx {
                let _ = tx.push(peer_id, (peer_id, dkg_request));
            }
        }
        Ok(())
    }

    fn on_dkg_start_notification(&mut self, notification: EventNotification) -> Result<()> {
        if let Some(tx) = self.dkg_start_event_tx.take() {
            let EventNotification {
                subscribed_events, ..
            } = notification;
            for event in subscribed_events {
                if let Ok(dkg_start_event) = DKGStartEvent::try_from(&event) {
                    let _ = tx.send(dkg_start_event);
                    return Ok(());
                } else {
                    debug!("[DKG] on_dkg_start_notification: failed in converting a contract event to a dkg start event!");
                }
            }
        }
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
        let my_index = epoch_state
            .verifier
            .address_to_validator_index()
            .get(&self.my_addr)
            .copied();

        let features = payload.get::<Features>().unwrap_or_default();

        if let (true, Some(my_index)) = (
            features.is_enabled(FeatureFlag::RECONFIGURE_WITH_DKG),
            my_index,
        ) {
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
            let agg_trx_producer = AggTranscriptProducer::new(rb);

            let (dkg_start_event_tx, dkg_start_event_rx) = oneshot::channel();
            self.dkg_start_event_tx = Some(dkg_start_event_tx);

            let (dkg_rpc_msg_tx, dkg_rpc_msg_rx) = aptos_channel::new::<
                AccountAddress,
                (AccountAddress, IncomingRpcRequest),
            >(QueueStyle::FIFO, 100, None);
            self.dkg_rpc_msg_tx = Some(dkg_rpc_msg_tx);
            let (dkg_manager_close_tx, dkg_manager_close_rx) = oneshot::channel();
            self.dkg_manager_close_tx = Some(dkg_manager_close_tx);

            let dkg_manager = DKGManager::<DefaultDKG>::new(
                self.dkg_dealer_sk.clone(),
                my_index,
                self.my_addr,
                epoch_state,
                Arc::new(agg_trx_producer),
                self.vtxn_pool.clone(),
            );
            tokio::spawn(dkg_manager.run(
                in_progress_session,
                dkg_start_event_rx,
                dkg_rpc_msg_rx,
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
