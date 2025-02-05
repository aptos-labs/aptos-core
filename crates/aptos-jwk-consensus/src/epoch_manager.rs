// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    jwk_manager::JWKManager,
    network::{IncomingRpcRequest, NetworkReceivers, NetworkSender},
    network_interface::JWKConsensusNetworkClient,
    types::JWKConsensusMsg,
    update_certifier::UpdateCertifier,
};
use anyhow::Result;
use aptos_bounded_executor::BoundedExecutor;
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_consensus_types::common::Author;
use aptos_crypto::bls12381::PrivateKey;
use aptos_event_notifications::{
    EventNotification, EventNotificationListener, ReconfigNotification,
    ReconfigNotificationListener,
};
use aptos_logger::{error, info};
use aptos_network::{application::interface::NetworkClient, protocols::network::Event};
use aptos_reliable_broadcast::ReliableBroadcast;
use aptos_types::{
    account_address::AccountAddress,
    epoch_state::EpochState,
    jwks,
    jwks::{ObservedJWKs, ObservedJWKsUpdated, SupportedOIDCProviders},
    on_chain_config::{
        FeatureFlag, Features, OnChainConfigPayload, OnChainConfigProvider, OnChainConsensusConfig,
        OnChainJWKConsensusConfig, ValidatorSet,
    },
};
use aptos_validator_transaction_pool::VTxnPoolState;
use futures::StreamExt;
use futures_channel::oneshot;
use std::{sync::Arc, time::Duration};
use tokio_retry::strategy::ExponentialBackoff;

pub struct EpochManager<P: OnChainConfigProvider> {
    // some useful metadata
    my_addr: AccountAddress,
    epoch_state: Option<Arc<EpochState>>,

    // credential
    consensus_key: Arc<PrivateKey>,

    // events we subscribe
    reconfig_events: ReconfigNotificationListener<P>,
    jwk_updated_events: EventNotificationListener,

    // message channels to JWK manager
    jwk_updated_event_txs: Option<aptos_channel::Sender<(), ObservedJWKsUpdated>>,
    jwk_rpc_msg_tx:
        Option<aptos_channel::Sender<AccountAddress, (AccountAddress, IncomingRpcRequest)>>,
    jwk_manager_close_tx: Option<oneshot::Sender<oneshot::Sender<()>>>,

    // network utils
    self_sender: aptos_channels::Sender<Event<JWKConsensusMsg>>,
    network_sender: JWKConsensusNetworkClient<NetworkClient<JWKConsensusMsg>>,

    // vtxn pool handle
    vtxn_pool: VTxnPoolState,
}

impl<P: OnChainConfigProvider> EpochManager<P> {
    pub fn new(
        my_addr: AccountAddress,
        consensus_key: PrivateKey,
        reconfig_events: ReconfigNotificationListener<P>,
        jwk_updated_events: EventNotificationListener,
        self_sender: aptos_channels::Sender<Event<JWKConsensusMsg>>,
        network_sender: JWKConsensusNetworkClient<NetworkClient<JWKConsensusMsg>>,
        vtxn_pool: VTxnPoolState,
    ) -> Self {
        Self {
            my_addr,
            consensus_key: Arc::new(consensus_key),
            epoch_state: None,
            reconfig_events,
            jwk_updated_events,
            self_sender,
            network_sender,
            vtxn_pool,
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
    ) -> Result<()> {
        if Some(rpc_request.msg.epoch()) == self.epoch_state.as_ref().map(|s| s.epoch) {
            if let Some(tx) = &self.jwk_rpc_msg_tx {
                let _ = tx.push(peer_id, (peer_id, rpc_request));
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
            if let Ok(jwk_event) = ObservedJWKsUpdated::try_from(&event) {
                if let Some(tx) = self.jwk_updated_event_txs.as_ref() {
                    let _ = tx.push((), jwk_event);
                }
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
        println!("Starting new epoch.");
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

        info!(
            epoch = epoch_state.epoch,
            "EpochManager starting new epoch."
        );

        let features = payload.get::<Features>().unwrap_or_default();
        let jwk_consensus_config = payload.get::<OnChainJWKConsensusConfig>();
        let onchain_observed_jwks = payload.get::<ObservedJWKs>().ok();
        let onchain_consensus_config = payload.get::<OnChainConsensusConfig>().unwrap_or_default();

        let (jwk_manager_should_run, oidc_providers) = match jwk_consensus_config {
            Ok(config) => {
                let should_run =
                    config.jwk_consensus_enabled() && onchain_consensus_config.is_vtxn_enabled();
                let providers = config
                    .oidc_providers_cloned()
                    .into_iter()
                    .map(jwks::OIDCProvider::from)
                    .collect();
                (should_run, Some(SupportedOIDCProviders { providers }))
            },
            Err(_) => {
                //TODO: remove this case once the framework change of this commit is published.
                let should_run = features.is_enabled(FeatureFlag::JWK_CONSENSUS)
                    && onchain_consensus_config.is_vtxn_enabled();
                let providers = payload.get::<SupportedOIDCProviders>().ok();
                (should_run, providers)
            },
        };

        if jwk_manager_should_run && my_index.is_some() {
            info!(epoch = epoch_state.epoch, "JWKManager starting.");
            let network_sender = NetworkSender::new(
                self.my_addr,
                self.network_sender.clone(),
                self.self_sender.clone(),
            );
            let rb = ReliableBroadcast::new(
                self.my_addr,
                epoch_state.verifier.get_ordered_account_addresses(),
                Arc::new(network_sender),
                ExponentialBackoff::from_millis(5),
                aptos_time_service::TimeService::real(),
                Duration::from_millis(1000),
                BoundedExecutor::new(8, tokio::runtime::Handle::current()),
            );
            let update_certifier = UpdateCertifier::new(rb);

            let jwk_consensus_manager = JWKManager::new(
                self.consensus_key.clone(),
                self.my_addr,
                epoch_state.clone(),
                Arc::new(update_certifier),
                self.vtxn_pool.clone(),
            );

            let (jwk_event_tx, jwk_event_rx) = aptos_channel::new(QueueStyle::KLAST, 1, None);
            self.jwk_updated_event_txs = Some(jwk_event_tx);
            let (jwk_rpc_msg_tx, jwk_rpc_msg_rx) = aptos_channel::new(QueueStyle::FIFO, 100, None);

            let (jwk_manager_close_tx, jwk_manager_close_rx) = oneshot::channel();
            self.jwk_rpc_msg_tx = Some(jwk_rpc_msg_tx);
            self.jwk_manager_close_tx = Some(jwk_manager_close_tx);

            tokio::spawn(jwk_consensus_manager.run(
                oidc_providers,
                onchain_observed_jwks,
                jwk_event_rx,
                jwk_rpc_msg_rx,
                jwk_manager_close_rx,
            ));
            info!(epoch = epoch_state.epoch, "JWKManager spawned.",);
        }
    }

    async fn on_new_epoch(&mut self, reconfig_notification: ReconfigNotification<P>) -> Result<()> {
        println!("EpochManager received new epoch notification.");
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
