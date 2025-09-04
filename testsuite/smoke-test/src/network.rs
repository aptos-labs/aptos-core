// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    smoke_test_environment::{new_local_swarm_with_velor, SwarmBuilder},
    state_sync::test_all_validator_failures,
    utils::{MAX_CONNECTIVITY_WAIT_SECS, MAX_HEALTHY_WAIT_SECS},
};
use velor::test::CliTestFramework;
use velor_config::{
    config::{
        DiscoveryMethod, FileDiscovery, Identity, NetworkConfig, NodeConfig, OverrideNodeConfig,
        Peer, PeerSet, RestDiscovery,
    },
    network_id::NetworkId,
};
use velor_crypto::{encoding_type::EncodingType, x25519, x25519::PrivateKey};
use velor_forge::{FullNode, Node, NodeExt, Swarm};
use velor_genesis::config::HostAndPort;
use velor_sdk::move_types::account_address::AccountAddress;
use velor_temppath::TempPath;
use std::{
    collections::HashMap,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

#[tokio::test]
async fn test_connection_limiting() {
    let mut swarm = new_local_swarm_with_velor(1).await;
    let version = swarm.versions().max().unwrap();
    let validator_peer_id = swarm.validators().next().unwrap().peer_id();

    // Only allow file based discovery, disallow other nodes
    let cli = CliTestFramework::local_new(0);
    let host = HostAndPort::local(swarm.validators().next().unwrap().port()).unwrap();
    let (private_key, peer_set) =
        generate_private_key_and_peer(&cli, host.clone(), [1u8; 32]).await;
    let discovery_file = create_discovery_file(peer_set.clone());
    let mut full_node_config = NodeConfig::get_default_vfn_config();
    modify_network_config(&mut full_node_config, &NetworkId::Public, |network| {
        network.discovery_method = DiscoveryMethod::None;
        network.discovery_methods = vec![
            DiscoveryMethod::Onchain,
            DiscoveryMethod::File(FileDiscovery {
                path: discovery_file.path().to_path_buf(),
                interval_secs: 1,
            }),
        ];
        network.max_inbound_connections = 0;
    });

    let vfn_peer_id = swarm
        .add_validator_fullnode(
            &version,
            OverrideNodeConfig::new_with_default_base(full_node_config),
            validator_peer_id,
        )
        .unwrap();

    // Wait till nodes are healthy
    swarm
        .fullnode_mut(vfn_peer_id)
        .unwrap()
        .wait_until_healthy(Instant::now() + Duration::from_secs(MAX_HEALTHY_WAIT_SECS))
        .await
        .unwrap();

    // This node should be able to connect
    let pfn_peer_id = swarm
        .add_full_node(
            &version,
            OverrideNodeConfig::new_with_default_base(add_identity_to_config(
                NodeConfig::get_default_pfn_config(),
                &NetworkId::Public,
                private_key,
                peer_set,
            )),
        )
        .await
        .unwrap();
    swarm
        .fullnode_mut(pfn_peer_id)
        .unwrap()
        .wait_until_healthy(Instant::now() + Duration::from_secs(MAX_HEALTHY_WAIT_SECS))
        .await
        .unwrap();
    // This node should connect
    FullNode::wait_for_connectivity(
        swarm.fullnode(pfn_peer_id).unwrap(),
        Instant::now() + Duration::from_secs(MAX_CONNECTIVITY_WAIT_SECS),
    )
    .await
    .unwrap();
    assert_eq!(
        1,
        swarm
            .fullnode(vfn_peer_id)
            .unwrap()
            .get_connected_peers(NetworkId::Public, Some("inbound"))
            .await
            .unwrap()
            .unwrap_or(0)
    );

    // And not be able to connect with an arbitrary one, limit is 0
    // TODO: Improve network checker to keep connection alive so we can test connection limits without nodes
    let cli = CliTestFramework::local_new(0);
    let (private_key, peer_set) =
        generate_private_key_and_peer(&cli, host.clone(), [2u8; 32]).await;
    let pfn_peer_id_fail = swarm
        .add_full_node(
            &version,
            OverrideNodeConfig::new_with_default_base(add_identity_to_config(
                NodeConfig::get_default_pfn_config(),
                &NetworkId::Public,
                private_key,
                peer_set,
            )),
        )
        .await
        .unwrap();

    // This node should fail to connect
    swarm
        .fullnode_mut(pfn_peer_id_fail)
        .unwrap()
        .wait_until_healthy(Instant::now() + Duration::from_secs(MAX_HEALTHY_WAIT_SECS))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_secs(5)).await;
    assert_eq!(
        1,
        swarm
            .fullnode(vfn_peer_id)
            .unwrap()
            .get_connected_peers(NetworkId::Public, Some("inbound"))
            .await
            .unwrap()
            .unwrap_or(0)
    );
}

#[tokio::test]
async fn test_rest_discovery() {
    let mut swarm = SwarmBuilder::new_local(1).with_velor().build().await;

    // Point to an already existing node
    let (version, rest_endpoint) = {
        let validator = swarm.validators().next().unwrap();
        (validator.version(), validator.rest_api_endpoint())
    };
    let mut full_node_config = NodeConfig::get_default_pfn_config();
    let network_config = full_node_config.full_node_networks.first_mut().unwrap();
    network_config.discovery_method = DiscoveryMethod::Rest(RestDiscovery {
        url: rest_endpoint,
        interval_secs: 1,
    });

    // Start a new node that should connect to the previous node only via REST
    // The startup wait time should check if it connects successfully
    swarm
        .add_full_node(
            &version,
            OverrideNodeConfig::new_with_default_base(full_node_config),
        )
        .await
        .unwrap();
}

// Currently this test seems flaky: https://github.com/velor-chain/velor-core/issues/670
#[ignore]
#[tokio::test]
async fn test_file_discovery() {
    let cli = CliTestFramework::local_new(0);
    // TODO: This host needs to be set properly
    let host = HostAndPort::local(6180).unwrap();
    let (_, peer_set) = generate_private_key_and_peer(&cli, host, [0u8; 32]).await;
    let discovery_file = Arc::new(create_discovery_file(peer_set));
    let discovery_file_for_closure = discovery_file.clone();
    let swarm = SwarmBuilder::new_local(1)
        .with_velor()
        .with_init_config(Arc::new(move |_, config, _| {
            let discovery_file_for_closure2 = discovery_file_for_closure.clone();
            modify_network_config(config, &NetworkId::Validator, move |network| {
                network.discovery_method = DiscoveryMethod::None;
                network.discovery_methods = vec![
                    DiscoveryMethod::Onchain,
                    DiscoveryMethod::File(FileDiscovery {
                        path: discovery_file_for_closure2.path().to_path_buf(),
                        interval_secs: 1,
                    }),
                ];
            });
        }))
        .build()
        .await;
    let _validator_peer_id = swarm.validators().next().unwrap().peer_id();

    // At first we should be able to connect
    // TODO: Check connection

    // Now when we clear the file, we shouldn't be able to connect
    write_peerset_to_file((*discovery_file).as_ref(), HashMap::new());
    tokio::time::sleep(Duration::from_millis(300)).await;

    // TODO: Check connection
}

// TODO: add more complex tests for the peer monitoring service.
// TODO: move the state sync functions to a utility file (instead of importing directly).

#[tokio::test]
async fn test_peer_monitoring_service_enabled() {
    // Create a swarm of 4 validators with peer monitoring enabled
    let swarm = SwarmBuilder::new_local(4)
        .with_velor()
        .with_init_config(Arc::new(|_, config, _| {
            config.peer_monitoring_service.enable_peer_monitoring_client = true;
        }))
        .build()
        .await;

    // Test the ability of the validators to sync
    test_all_validator_failures(swarm).await;
}

/// Creates a discovery file with the given `PeerSet`
fn create_discovery_file(peer_set: PeerSet) -> TempPath {
    let discovery_file = TempPath::new();
    discovery_file.create_as_file().unwrap();
    write_peerset_to_file(discovery_file.as_ref(), peer_set);
    discovery_file
}

/// Generates `PrivateKey` and `Peer` information for a client / node
async fn generate_private_key_and_peer(
    cli: &CliTestFramework,
    host: HostAndPort,
    seed: [u8; 32],
) -> (x25519::PrivateKey, HashMap<AccountAddress, Peer>) {
    let temp_folder = TempPath::new();
    temp_folder.create_as_dir().unwrap();
    let private_key_path = temp_folder.path().join("private_key.txt");
    let extract_peer_path = temp_folder.path().join("extract_peer.txt");
    cli.generate_x25519_key(private_key_path.clone(), seed)
        .await
        .unwrap();

    let private_key: x25519::PrivateKey = EncodingType::Hex
        .load_key("test-key", private_key_path.as_path())
        .unwrap();
    let peer_set = cli
        .extract_peer(host, private_key_path, extract_peer_path)
        .await
        .unwrap();
    // Check that public key matches peer
    assert_eq!(
        peer_set
            .iter()
            .next()
            .unwrap()
            .1
            .keys
            .iter()
            .next()
            .unwrap(),
        &private_key.public_key()
    );
    // Check that peer id matches public key
    assert_eq!(
        private_key.public_key().as_slice(),
        peer_set.iter().next().unwrap().0.as_slice()
    );
    (private_key, peer_set)
}

fn modify_network_config<F: FnOnce(&mut NetworkConfig)>(
    node_config: &mut NodeConfig,
    network_id: &NetworkId,
    modifier: F,
) {
    let network = match network_id {
        NetworkId::Validator => node_config.validator_network.as_mut().unwrap(),
        _ => node_config
            .full_node_networks
            .iter_mut()
            .find(|network| &network.network_id == network_id)
            .unwrap(),
    };

    modifier(network);
}

fn add_identity_to_config(
    mut config: NodeConfig,
    network_id: &NetworkId,
    private_key: PrivateKey,
    peer_set: PeerSet,
) -> NodeConfig {
    let (peer_id, _) = peer_set.iter().next().unwrap();
    modify_network_config(&mut config, network_id, |network| {
        network.identity = Identity::from_config(private_key, *peer_id);
    });
    config
}

pub fn write_peerset_to_file(path: &Path, peers: PeerSet) {
    let file_contents = serde_yaml::to_vec(&peers).unwrap();
    std::fs::write(path, file_contents).unwrap();
}
