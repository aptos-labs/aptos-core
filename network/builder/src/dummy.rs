// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Integration tests for validator_network.

use crate::builder::NetworkBuilder;
use aptos_channels::aptos_channel;
use aptos_config::{
    config::{Peer, PeerRole, PeerSet, RoleType, NETWORK_CHANNEL_SIZE},
    network_id::{NetworkContext, NetworkId, PeerNetworkId},
};
use aptos_crypto::{test_utils::TEST_SEED, x25519, Uniform};
use aptos_netcore::transport::ConnectionOrigin;
use aptos_network::{
    application::{interface::NetworkClient, storage::PeersAndMetadata},
    peer_manager::builder::AuthenticationMode,
    protocols::network::{
        NetworkApplicationConfig, NetworkClientConfig, NetworkEvents, NetworkServiceConfig,
    },
    ProtocolId,
};
use aptos_time_service::TimeService;
use aptos_types::{chain_id::ChainId, network_address::NetworkAddress, PeerId};
use futures::executor::block_on;
use maplit::hashmap;
use rand::{rngs::StdRng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::Arc, time::Duration};
use tokio::runtime::Runtime;

const TEST_RPC_PROTOCOL: ProtocolId = ProtocolId::ConsensusRpcBcs;
const TEST_DIRECT_SEND_PROTOCOL: ProtocolId = ProtocolId::ConsensusDirectSendBcs;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct DummyMsg(pub Vec<u8>);

pub fn dummy_network_config() -> NetworkApplicationConfig {
    let direct_send_protocols = vec![TEST_DIRECT_SEND_PROTOCOL];
    let rpc_protocls = vec![TEST_RPC_PROTOCOL];

    let network_client_config =
        NetworkClientConfig::new(direct_send_protocols.clone(), rpc_protocls.clone());
    let network_service_config = NetworkServiceConfig::new(
        direct_send_protocols,
        rpc_protocls,
        aptos_channel::Config::new(NETWORK_CHANNEL_SIZE),
    );
    NetworkApplicationConfig::new(network_client_config, network_service_config)
}

/// TODO(davidiw): In DummyNetwork, replace DummyMsg with a Serde compatible type once migration
/// is complete
pub type DummyNetworkEvents = NetworkEvents<DummyMsg>;

pub struct DummyNetwork {
    pub runtime: Runtime,
    pub dialer_peer: PeerNetworkId,
    pub dialer_events: DummyNetworkEvents,
    pub dialer_network_client: NetworkClient<DummyMsg>,
    pub listener_peer: PeerNetworkId,
    pub listener_events: DummyNetworkEvents,
    pub listener_network_client: NetworkClient<DummyMsg>,
}

/// The following sets up a 2 peer network and verifies connectivity.
pub fn setup_network() -> DummyNetwork {
    // Create and enter a runtime
    let runtime = Runtime::new().unwrap();
    let _entered_runtime = runtime.enter();

    // Create a new set of peers
    let role = RoleType::Validator;
    let network_id = NetworkId::Validator;
    let chain_id = ChainId::default();
    let dialer_peer = PeerNetworkId::new(network_id, PeerId::random());
    let listener_peer = PeerNetworkId::new(network_id, PeerId::random());

    // Setup keys for dialer.
    let mut rng = StdRng::from_seed(TEST_SEED);
    let dialer_identity_private_key = x25519::PrivateKey::generate(&mut rng);
    let dialer_identity_public_key = dialer_identity_private_key.public_key();
    let dialer_pubkeys: HashSet<_> = vec![dialer_identity_public_key].into_iter().collect();

    // Setup keys for listener.
    let listener_identity_private_key = x25519::PrivateKey::generate(&mut rng);

    // Setup listen addresses
    let dialer_addr: NetworkAddress = "/ip4/127.0.0.1/tcp/0".parse().unwrap();
    let listener_addr: NetworkAddress = "/ip4/127.0.0.1/tcp/0".parse().unwrap();

    // Setup seed peers
    let mut seeds = PeerSet::new();
    seeds.insert(
        dialer_peer.peer_id(),
        Peer::new(vec![], dialer_pubkeys, PeerRole::Validator),
    );

    let authentication_mode = AuthenticationMode::Mutual(listener_identity_private_key);
    let listener_peers_and_metadata = PeersAndMetadata::new(&[network_id]);
    // Set up the listener network
    let network_context = NetworkContext::new(role, network_id, listener_peer.peer_id());
    let mut network_builder = NetworkBuilder::new_for_test(
        chain_id,
        seeds.clone(),
        network_context,
        TimeService::real(),
        listener_addr,
        authentication_mode,
        listener_peers_and_metadata.clone(),
    );

    let (listener_sender, listener_events) = network_builder
        .add_client_and_service::<_, DummyNetworkEvents>(&dummy_network_config(), None, true);
    network_builder.build(runtime.handle().clone()).start();
    let listener_network_client = NetworkClient::new(
        vec![TEST_DIRECT_SEND_PROTOCOL],
        vec![TEST_RPC_PROTOCOL],
        hashmap! {network_id => listener_sender},
        listener_peers_and_metadata.clone(),
    );

    // Add the listener address with port
    let listener_addr = network_builder.listen_address();
    seeds.insert(
        listener_peer.peer_id(),
        Peer::from_addrs(PeerRole::Validator, vec![listener_addr]),
    );

    let authentication_mode = AuthenticationMode::Mutual(dialer_identity_private_key);

    let peers_and_metadata = PeersAndMetadata::new(&[network_id]);
    // Set up the dialer network
    let network_context = NetworkContext::new(role, network_id, dialer_peer.peer_id());

    let mut network_builder = NetworkBuilder::new_for_test(
        chain_id,
        seeds,
        network_context,
        TimeService::real(),
        dialer_addr,
        authentication_mode,
        peers_and_metadata.clone(),
    );

    let (dialer_sender, dialer_events) = network_builder
        .add_client_and_service::<_, DummyNetworkEvents>(&dummy_network_config(), None, true);
    network_builder.build(runtime.handle().clone()).start();
    let dialer_network_client = NetworkClient::new(
        vec![TEST_DIRECT_SEND_PROTOCOL],
        vec![TEST_RPC_PROTOCOL],
        hashmap! {network_id => dialer_sender},
        peers_and_metadata.clone(),
    );

    // Wait for establishing connection
    block_on(wait_for_connection_established(
        peers_and_metadata.clone(),
        network_id,
        listener_peer.peer_id(),
        ConnectionOrigin::Outbound,
        PeerRole::Validator,
    ));

    // Wait for connection to be established on listener side
    block_on(wait_for_connection_established(
        listener_peers_and_metadata.clone(),
        network_id,
        dialer_peer.peer_id(),
        ConnectionOrigin::Inbound,
        PeerRole::Validator,
    ));

    DummyNetwork {
        runtime,
        dialer_peer,
        dialer_events,
        dialer_network_client,
        listener_peer,
        listener_events,
        listener_network_client,
    }
}

/// Helper function to wait for a connection to be established
async fn wait_for_connection_established(
    peers_and_metadata: Arc<PeersAndMetadata>,
    network_id: NetworkId,
    expected_peer_id: PeerId,
    expected_origin: ConnectionOrigin,
    expected_role: PeerRole,
) {
    let timeout_duration = Duration::from_secs(10);
    let peer_network_id = PeerNetworkId::new(network_id, expected_peer_id);

    tokio::time::timeout(timeout_duration, async {
        loop {
            let connected = peers_and_metadata
                .get_connected_peers_and_metadata()
                .unwrap();

            if let Some(metadata) = connected.get(&peer_network_id) {
                assert_eq!(
                    metadata.get_connection_metadata().remote_peer_id,
                    expected_peer_id
                );
                assert_eq!(metadata.get_connection_metadata().origin, expected_origin);
                assert_eq!(metadata.get_connection_metadata().role, expected_role);
                return;
            }

            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("Timed out waiting for connection to be established");
}
