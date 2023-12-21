// Copyright © Aptos Foundation

use aptos_config::network_id::{NetworkId, PeerNetworkId};
use aptos_event_notifications::{
    DbBackedOnChainConfig, EventNotificationListener, ReconfigNotificationListener,
};
use aptos_network::application::{
    error::Error,
    interface::{NetworkClient, NetworkClientInterface, NetworkServiceEvents},
};
use aptos_types::PeerId;
use aptos_validator_transaction_pool as vtxn_pool;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::runtime::Runtime;

#[allow(clippy::let_and_return)]
pub fn start_jwk_consensus_runtime(
    _network_client: NetworkClient<JWKConsensusMsg>,
    _network_service_events: NetworkServiceEvents<JWKConsensusMsg>,
    _vtxn_pool_writer: vtxn_pool::SingleTopicWriteClient,
    mut reconfig_events: ReconfigNotificationListener<DbBackedOnChainConfig>,
    mut onchain_jwk_updated_events: EventNotificationListener,
) -> Runtime {
    let runtime = aptos_runtimes::spawn_named_runtime("jwk".into(), Some(4));
    runtime.spawn(async move {
        loop {
            tokio::select! {
                _ = reconfig_events.select_next_some() => {},
                _ = onchain_jwk_updated_events.select_next_some() => {},
            }
        }
    });
    runtime
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct JWKConsensusMsg {}

#[derive(Clone)]
pub struct JWKNetworkClient<NetworkClient> {
    network_client: NetworkClient,
}

impl<NetworkClient: NetworkClientInterface<JWKConsensusMsg>> JWKNetworkClient<NetworkClient> {
    pub fn new(network_client: NetworkClient) -> Self {
        Self { network_client }
    }

    pub async fn send_rpc(
        &self,
        peer: PeerId,
        message: JWKConsensusMsg,
        rpc_timeout: Duration,
    ) -> Result<JWKConsensusMsg, Error> {
        let peer_network_id = PeerNetworkId::new(NetworkId::Validator, peer);
        self.network_client
            .send_to_peer_rpc(message, rpc_timeout, peer_network_id)
            .await
    }
}

pub mod network_interface;
