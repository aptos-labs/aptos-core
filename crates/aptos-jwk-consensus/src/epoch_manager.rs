// Copyright © Aptos Foundation

use crate::{
    certified_update_producer::RealCertifiedUpdateProducer,
    jwk_manager::JWKManager,
    network::{IncomingRpcRequest, NetworkReceivers, NetworkSender},
    network_interface::JWKConsensusNetworkClient,
    types::JWKConsensusMsg,
};
use anyhow::Result;
use aptos_bounded_executor::BoundedExecutor;
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_config::config::IdentityBlob;
use aptos_consensus_types::common::Author;
use aptos_event_notifications::{
    EventNotification, EventNotificationListener, ReconfigNotification,
    ReconfigNotificationListener,
};
use aptos_logger::{debug, error, info};
use aptos_network::{application::interface::NetworkClient, protocols::network::Event};
use aptos_reliable_broadcast::ReliableBroadcast;
use aptos_types::{
    account_address::AccountAddress,
    epoch_state::EpochState,
    jwks::{ObservedJWKs, ObservedJWKsUpdated, SupportedOIDCProviders},
    on_chain_config::{
        FeatureFlag, Features, OnChainConfigPayload, OnChainConfigProvider, ValidatorSet,
    },
};
use aptos_validator_transaction_pool as vtxn_pool;
use futures::StreamExt;
use futures_channel::oneshot;
use std::{sync::Arc, time::Duration};
use tokio_retry::strategy::ExponentialBackoff;

pub struct EpochManager<P: OnChainConfigProvider> {
    my_addr: AccountAddress,
    identity_blob: Arc<IdentityBlob>,
    epoch_state: Option<Arc<EpochState>>,
    reconfig_events: ReconfigNotificationListener<P>,
    jwk_updated_events: EventNotificationListener,
    jwk_updated_event_txs: Option<aptos_channel::Sender<(), ObservedJWKsUpdated>>,
    jwk_rpc_msg_tx: Option<aptos_channel::Sender<(), (AccountAddress, IncomingRpcRequest)>>,
    jwk_manager_close_tx: Option<oneshot::Sender<oneshot::Sender<()>>>,

    self_sender: aptos_channels::Sender<Event<JWKConsensusMsg>>,
    network_sender: JWKConsensusNetworkClient<NetworkClient<JWKConsensusMsg>>,

    vtxn_pool_write_cli: Arc<vtxn_pool::SingleTopicWriteClient>,
}

impl<P: OnChainConfigProvider> EpochManager<P> {
    pub fn new(
        my_addr: AccountAddress,
        identity_blob: Arc<IdentityBlob>,
        reconfig_events: ReconfigNotificationListener<P>,
        jwk_updated_events: EventNotificationListener,
        self_sender: aptos_channels::Sender<Event<JWKConsensusMsg>>,
        network_sender: JWKConsensusNetworkClient<NetworkClient<JWKConsensusMsg>>,
        vtxn_pool_write_cli: vtxn_pool::SingleTopicWriteClient,
    ) -> Self {
        Self {
            my_addr,
            identity_blob,
            epoch_state: None,
            reconfig_events,
            jwk_updated_events,
            self_sender,
            network_sender,
            vtxn_pool_write_cli: Arc::new(vtxn_pool_write_cli),
            jwk_updated_event_txs: None,
            jwk_rpc_msg_tx: None,
            jwk_manager_close_tx: None,
        }
    }

    /// On a new RPC request, forward to JWK consensus manager, if it is alive.
    fn process_rpc_request(
        &mut self,
        peer_id: Author,
        rpc_request: IncomingRpcRequest,
    ) -> anyhow::Result<()> {
        if Some(rpc_request.msg.epoch()) == self.epoch_state.as_ref().map(|s| s.epoch) {
            if let Some(tx) = &self.jwk_rpc_msg_tx {
                let _ = tx.push((), (peer_id, rpc_request));
            }
        }
        Ok(())
    }

    /// On a on-chain JWK updated events, forward to JWK consensus manager if it is alive.
    fn process_onchain_event(&mut self, notification: EventNotification) -> Result<()> {
        let EventNotification {
            subscribed_events, ..
        } = notification;
        for event in subscribed_events {
            let jwk_event = ObservedJWKsUpdated::try_from(&event).unwrap();
            if let Some(tx) = self.jwk_updated_event_txs.as_ref() {
                let _ = tx.push((), jwk_event);
            }
        }
        Ok(())
    }

    pub async fn start(mut self, mut network_receivers: NetworkReceivers) {
        self.await_reconfig_notification().await;
        loop {
            let handle_result = tokio::select! {
                reconfig_notification = self.reconfig_events.select_next_some() => {
                    self.on_new_epoch(reconfig_notification).await
                },
                event = self.jwk_updated_events.select_next_some() => {
                    self.process_onchain_event(event)
                },
                (peer, rpc_request) = network_receivers.rpc_rx.select_next_some() => {
                    self.process_rpc_request(peer, rpc_request)
                }
            };

            if let Err(e) = handle_result {
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
        debug!("[JWK] start_new_epoch: new_epoch={}", epoch_state.epoch);

        let features = payload.get::<Features>().unwrap_or_default();

        if features.is_enabled(FeatureFlag::JWK_CONSENSUS) {
            let onchain_oidc_provider_set = payload.get::<SupportedOIDCProviders>().ok();
            let onchain_observed_jwks = payload.get::<ObservedJWKs>().ok();
            debug!(
                "[JWK] JWK manager init, epoch={}",
                self.epoch_state.as_ref().unwrap().epoch
            );
            let network_sender = NetworkSender::new(
                self.my_addr,
                self.network_sender.clone(),
                self.self_sender.clone(),
            );
            let rb = ReliableBroadcast::new(
                epoch_state.verifier.get_ordered_account_addresses(),
                Arc::new(network_sender),
                ExponentialBackoff::from_millis(5),
                aptos_time_service::TimeService::real(),
                Duration::from_millis(1000),
                BoundedExecutor::new(8, tokio::runtime::Handle::current()),
            );
            let qc_update_producer = RealCertifiedUpdateProducer::new(self.my_addr, rb);

            let jwk_consensus_manager = JWKManager::new(
                self.identity_blob.clone(),
                self.my_addr,
                epoch_state.clone(),
                Arc::new(qc_update_producer),
                self.vtxn_pool_write_cli.clone(),
            );

            let (jwk_event_tx, jwk_event_rx) = aptos_channel::new(QueueStyle::KLAST, 1, None);
            self.jwk_updated_event_txs = Some(jwk_event_tx);
            let (jwk_rpc_msg_tx, jwk_rpc_msg_rx) = aptos_channel::new(QueueStyle::FIFO, 100, None);

            let (jwk_manager_close_tx, jwk_manager_close_rx) = oneshot::channel();
            self.jwk_rpc_msg_tx = Some(jwk_rpc_msg_tx);
            self.jwk_manager_close_tx = Some(jwk_manager_close_tx);

            tokio::spawn(jwk_consensus_manager.run(
                onchain_oidc_provider_set,
                onchain_observed_jwks,
                jwk_event_rx,
                jwk_rpc_msg_rx,
                jwk_manager_close_rx,
            ));
            info!(
                "jwk consensus manager spawned for epoch {}",
                epoch_state.epoch
            );
        }
    }

    async fn on_new_epoch(&mut self, reconfig_notification: ReconfigNotification<P>) -> Result<()> {
        self.shutdown_current_processor().await;
        self.start_new_epoch(reconfig_notification.on_chain_configs)
            .await;
        Ok(())
    }

    async fn shutdown_current_processor(&mut self) {
        if let Some(tx) = self.jwk_manager_close_tx.take() {
            let (ack_tx, ack_rx) = oneshot::channel();
            let _ = tx.send(ack_tx);
            let _ = ack_rx.await;
        }

        self.jwk_updated_event_txs = None;
    }
}
