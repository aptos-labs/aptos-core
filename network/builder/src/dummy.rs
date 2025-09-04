// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for validator_network.

use crate::builder::NetworkBuilder;
use velor_channels::velor_channel;
use velor_config::{
    config::{Peer, PeerRole, PeerSet, RoleType, NETWORK_CHANNEL_SIZE},
    network_id::{NetworkContext, NetworkId, PeerNetworkId},
};
use velor_crypto::{test_utils::TEST_SEED, x25519, Uniform};
use velor_netcore::transport::ConnectionOrigin;
use velor_network::{
    application::{interface::NetworkClient, storage::PeersAndMetadata},
    peer_manager::{builder::AuthenticationMode, ConnectionNotification},
    protocols::network::{
        NetworkApplicationConfig, NetworkClientConfig, NetworkEvents, NetworkServiceConfig,
    },
    ProtocolId,
};
use velor_time_service::TimeService;
use velor_types::{chain_id::ChainId, network_address::NetworkAddress, PeerId};
use futures::executor::block_on;
use maplit::hashmap;
use rand::{rngs::StdRng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
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
        velor_channel::Config::new(NETWORK_CHANNEL_SIZE),
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
    let mut listener_connection_events = listener_peers_and_metadata.subscribe();
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

    let mut connection_events = peers_and_metadata.subscribe();

    let (dialer_sender, dialer_events) = network_builder
        .add_client_and_service::<_, DummyNetworkEvents>(&dummy_network_config(), None, true);
    network_builder.build(runtime.handle().clone()).start();
    let dialer_network_client = NetworkClient::new(
        vec![TEST_DIRECT_SEND_PROTOCOL],
        vec![TEST_RPC_PROTOCOL],
        hashmap! {network_id => dialer_sender},
        peers_and_metadata,
    );

    // Wait for establishing connection
    let first_dialer_event = block_on(connection_events.recv()).unwrap();
    if let ConnectionNotification::NewPeer(metadata, _network_id) = first_dialer_event {
        assert_eq!(metadata.remote_peer_id, listener_peer.peer_id());
        assert_eq!(metadata.origin, ConnectionOrigin::Outbound);
        assert_eq!(metadata.role, PeerRole::Validator);
    } else {
        panic!(
            "No NewPeer event on dialer received instead: {:?}",
            first_dialer_event
        );
    }

    let first_listener_event = block_on(listener_connection_events.recv()).unwrap();
    if let ConnectionNotification::NewPeer(metadata, _network_id) = first_listener_event {
        assert_eq!(metadata.remote_peer_id, dialer_peer.peer_id());
        assert_eq!(metadata.origin, ConnectionOrigin::Inbound);
        assert_eq!(metadata.role, PeerRole::Validator);
    } else {
        panic!(
            "No NewPeer event on listener received instead: {:?}",
            first_listener_event
        );
    }

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
