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
use aptos_validator_transaction_pool::VTxnPoolState;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::runtime::Runtime;

#[allow(clippy::let_and_return)]
pub fn start_jwk_consensus_runtime(
    _network_client: NetworkClient<JWKConsensusMsg>,
    _network_service_events: NetworkServiceEvents<JWKConsensusMsg>,
    _vtxn_pool: VTxnPoolState,
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

pub mod network;
pub mod network_interface;
pub mod types;
