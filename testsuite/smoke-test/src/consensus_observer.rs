// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    smoke_test_environment::SwarmBuilder,
    state_sync_utils,
    state_sync_utils::enable_consensus_observer,
    utils::{add_node_to_seeds, create_test_accounts, execute_transactions, wait_for_all_nodes},
};
use velor_config::{
    config::{NodeConfig, OverrideNodeConfig, PeerRole},
    network_id::NetworkId,
};
use velor_forge::{LocalNode, NodeExt, Swarm};
use velor_types::on_chain_config::{
    ConsensusAlgorithmConfig, OnChainConsensusConfig, ValidatorTxnConfig, DEFAULT_WINDOW_SIZE,
};
use std::sync::Arc;

#[tokio::test]
async fn test_consensus_observer_fast_sync_epoch_changes() {
    // Test fast syncing with consensus observer and epoch changes
    state_sync_utils::test_fullnode_fast_sync(true, true).await;
}

#[tokio::test]
async fn test_consensus_observer_fast_sync_no_epoch_changes() {
    // Test fast syncing with consensus observer and without epoch changes
    state_sync_utils::test_fullnode_fast_sync(false, true).await;
}

#[tokio::test]
async fn test_consensus_observer_fullnode_restart() {
    // Create a swarm of 1 validator with consensus observer enabled
    let mut swarm = SwarmBuilder::new_local(1)
        .with_velor()
        .with_init_config(Arc::new(|_, config, _| {
            enable_consensus_observer(true, config);
        }))
        .build()
        .await;

    // Create a VFN config that uses consensus observer
    let mut vfn_config = NodeConfig::get_default_vfn_config();
    enable_consensus_observer(true, &mut vfn_config);

    // Create the VFN and test its ability to stay up-to-date
    let vfn_peer_id = state_sync_utils::create_fullnode(vfn_config, &mut swarm).await;
    state_sync_utils::test_fullnode_sync(vfn_peer_id, &mut swarm, true, false).await;
}

#[tokio::test]
async fn test_consensus_observer_fullnode_restart_wipe() {
    // Create a swarm of 1 validator with consensus observer enabled
    let mut swarm = SwarmBuilder::new_local(1)
        .with_velor()
        .with_init_config(Arc::new(|_, config, _| {
            enable_consensus_observer(true, config);
        }))
        .build()
        .await;

    // Create a VFN config that uses consensus observer
    let mut vfn_config = NodeConfig::get_default_vfn_config();
    enable_consensus_observer(true, &mut vfn_config);

    // Create the VFN and test its ability to stay up-to-date (after a data wipe)
    let vfn_peer_id = state_sync_utils::create_fullnode(vfn_config, &mut swarm).await;
    state_sync_utils::test_fullnode_sync(vfn_peer_id, &mut swarm, true, true).await;
}

#[tokio::test]
async fn test_consensus_observer_fullnode_sync() {
    // Create a VFN config with consensus observer enabled
    let mut vfn_config = NodeConfig::get_default_vfn_config();
    enable_consensus_observer(true, &mut vfn_config);

    // Create a swarm of 1 validator and VFN with consensus observer enabled
    let mut swarm = SwarmBuilder::new_local(1)
        .with_num_fullnodes(1)
        .with_velor()
        .with_init_config(Arc::new(|_, config, _| {
            enable_consensus_observer(true, config);
        }))
        .with_vfn_config(vfn_config)
        .build()
        .await;

    // Execute a number of transactions on the validator
    let validator_peer_id = swarm.validators().next().unwrap().peer_id();
    let validator_client = swarm.validator(validator_peer_id).unwrap().rest_client();
    let (mut account_0, account_1) = create_test_accounts(&mut swarm).await;
    execute_transactions(
        &mut swarm,
        &validator_client,
        &mut account_0,
        &account_1,
        false,
    )
    .await;

    // Verify the VFN is up-to-date
    wait_for_all_nodes(&mut swarm).await;
}

#[tokio::test]
async fn test_consensus_observer_fullnode_sync_disable_quorum_store() {
    // Create a VFN config with consensus observer enabled
    let mut vfn_config = NodeConfig::get_default_vfn_config();
    enable_consensus_observer(true, &mut vfn_config);

    // Create a swarm of 1 validator and VFN with quorum store disabled
    let mut swarm = SwarmBuilder::new_local(1)
        .with_num_fullnodes(1)
        .with_velor()
        .with_init_config(Arc::new(|_, config, _| {
            enable_consensus_observer(true, config);
        }))
        .with_init_genesis_config(Arc::new(|genesis_config| {
            genesis_config.consensus_config = OnChainConsensusConfig::V4 {
                alg: ConsensusAlgorithmConfig::default_with_quorum_store_disabled(),
                vtxn: ValidatorTxnConfig::default_for_genesis(),
                window_size: DEFAULT_WINDOW_SIZE,
            };
        }))
        .with_vfn_config(vfn_config)
        .build()
        .await;

    // Execute a number of transactions on the validator
    let validator_peer_id = swarm.validators().next().unwrap().peer_id();
    let validator_client = swarm.validator(validator_peer_id).unwrap().rest_client();
    let (mut account_0, account_1) = create_test_accounts(&mut swarm).await;
    execute_transactions(
        &mut swarm,
        &validator_client,
        &mut account_0,
        &account_1,
        false,
    )
    .await;

    // Verify the VFN is up-to-date
    wait_for_all_nodes(&mut swarm).await;
}

#[tokio::test]
async fn test_consensus_observer_fullnode_sync_disconnected() {
    // Create a VFN config with consensus observer enabled
    let mut vfn_config = NodeConfig::get_default_vfn_config();
    enable_consensus_observer(true, &mut vfn_config);

    // Create a swarm of 3 validators and VFNs with consensus observer enabled
    let mut swarm = SwarmBuilder::new_local(3)
        .with_num_fullnodes(3)
        .with_velor()
        .with_init_config(Arc::new(|_, config, _| {
            enable_consensus_observer(true, config);
        }))
        .with_vfn_config(vfn_config)
        .build()
        .await;

    // Execute a number of transactions on the first validator
    let validator_peer_id = swarm.validators().next().unwrap().peer_id();
    let validator_client = swarm.validator(validator_peer_id).unwrap().rest_client();
    let (mut account_0, account_1) = create_test_accounts(&mut swarm).await;
    execute_transactions(
        &mut swarm,
        &validator_client,
        &mut account_0,
        &account_1,
        false,
    )
    .await;

    // Remove the VFN network from the first VFN (so that it relies on the public network)
    let vfn_peer_id = swarm.fullnodes().next().unwrap().peer_id();
    let vfn = swarm.fullnode_mut(vfn_peer_id).unwrap();
    let mut vfn_config = vfn.config().clone();
    let mut full_node_networks = vec![];
    for network in vfn_config.full_node_networks.iter_mut() {
        if network.network_id.is_public_network() {
            full_node_networks.push(network.clone());
        }
    }
    vfn_config.full_node_networks = full_node_networks;

    // Update and restart the VFN
    update_node_config_and_restart(vfn, vfn_config);

    // Execute a number of transactions on the first validator (again)
    execute_transactions(
        &mut swarm,
        &validator_client,
        &mut account_0,
        &account_1,
        false,
    )
    .await;

    // Verify the VFNs are up-to-date
    wait_for_all_nodes(&mut swarm).await;
}

#[tokio::test]
async fn test_consensus_observer_fullnode_sync_multiple_nodes() {
    // Create a VFN config with consensus observer enabled
    let mut vfn_config = NodeConfig::get_default_vfn_config();
    enable_consensus_observer(true, &mut vfn_config);

    // Create a swarm of 3 validators and VFNs with consensus observer enabled
    let mut swarm = SwarmBuilder::new_local(3)
        .with_num_fullnodes(3)
        .with_velor()
        .with_init_config(Arc::new(|_, config, _| {
            enable_consensus_observer(true, config);
        }))
        .with_vfn_config(vfn_config)
        .build()
        .await;

    // Execute a number of transactions on the first validator
    let validator_peer_id = swarm.validators().next().unwrap().peer_id();
    let validator_client = swarm.validator(validator_peer_id).unwrap().rest_client();
    let (mut account_0, account_1) = create_test_accounts(&mut swarm).await;
    execute_transactions(
        &mut swarm,
        &validator_client,
        &mut account_0,
        &account_1,
        false,
    )
    .await;

    // Verify the VFNs are all able to sync
    wait_for_all_nodes(&mut swarm).await;
}

#[tokio::test]
async fn test_consensus_observer_public_fullnode_sync() {
    // Create a VFN config with consensus observer enabled
    let mut vfn_config = NodeConfig::get_default_vfn_config();
    enable_consensus_observer(true, &mut vfn_config);

    // Create a swarm of 2 validators and 1 VFN with consensus observer enabled
    let mut swarm = SwarmBuilder::new_local(2)
        .with_num_fullnodes(1)
        .with_velor()
        .with_init_config(Arc::new(|_, config, _| {
            enable_consensus_observer(true, config);
        }))
        .with_vfn_config(vfn_config)
        .build()
        .await;

    // Create a PFN config with consensus observer enabled
    let mut pfn_config = NodeConfig::get_default_pfn_config();
    enable_consensus_observer(true, &mut pfn_config);

    // Create the PFN and connect it to the VFN
    let vfn_peer_id = swarm.full_nodes().next().unwrap().peer_id();
    let vfn_config = swarm.fullnode(vfn_peer_id).unwrap().config();
    add_node_to_seeds(
        &mut pfn_config,
        vfn_config,
        NetworkId::Public,
        PeerRole::PreferredUpstream,
    );
    swarm
        .add_full_node(
            &swarm.versions().max().unwrap(),
            OverrideNodeConfig::new_with_default_base(pfn_config),
        )
        .await
        .unwrap();

    // Wait for the PFN to come up
    wait_for_all_nodes(&mut swarm).await;

    // Execute a number of transactions on the first validator
    let validator_peer_id = swarm.validators().next().unwrap().peer_id();
    let validator_client = swarm.validator(validator_peer_id).unwrap().rest_client();
    let (mut account_0, account_1) = create_test_accounts(&mut swarm).await;
    execute_transactions(
        &mut swarm,
        &validator_client,
        &mut account_0,
        &account_1,
        false,
    )
    .await;

    // Verify all nodes are able to sync
    wait_for_all_nodes(&mut swarm).await;
}

#[tokio::test]
async fn test_consensus_observer_validator_restart() {
    // Test the ability of a validator to catch up after a restart
    test_validator_restart(false).await;
}

#[tokio::test]
async fn test_consensus_observer_validator_restart_wipe() {
    // Test the ability of a validator to catch up after a data wipe
    test_validator_restart(true).await;
}

/// A simple helper function that tests the ability of a validator (and it's
/// corresponding VFN) to catch up after a restart or data wipe.
async fn test_validator_restart(clear_storage: bool) {
    // Create a VFN config with consensus observer enabled
    let mut vfn_config = NodeConfig::get_default_vfn_config();
    enable_consensus_observer(true, &mut vfn_config);

    // Create a swarm of 4 validators and 1 VFN with consensus observer enabled
    let mut swarm = SwarmBuilder::new_local(4)
        .with_num_fullnodes(1)
        .with_velor()
        .with_init_config(Arc::new(|_, config, _| {
            enable_consensus_observer(true, config);
        }))
        .with_vfn_config(vfn_config)
        .build()
        .await;

    // Execute transactions on the first validator
    let validator_peer_id = swarm.validators().next().unwrap().peer_id();
    let validator_client = swarm.validator(validator_peer_id).unwrap().rest_client();
    let (mut account_0, account_1) = create_test_accounts(&mut swarm).await;
    execute_transactions(
        &mut swarm,
        &validator_client,
        &mut account_0,
        &account_1,
        true,
    )
    .await;

    // Stop the first validator (the VFN will also stop making progress)
    let validator_peer_id = swarm.validators().next().unwrap().peer_id();
    state_sync_utils::stop_validator_and_delete_storage(
        &mut swarm,
        validator_peer_id,
        clear_storage,
    )
    .await;

    // Execute a number of transactions on another validator
    let validator_peer_id = swarm.validators().last().unwrap().peer_id();
    let validator_client = swarm.validator(validator_peer_id).unwrap().rest_client();
    execute_transactions(
        &mut swarm,
        &validator_client,
        &mut account_0,
        &account_1,
        true,
    )
    .await;

    // Restart the first validator
    let validator_peer_id = swarm.validators().next().unwrap().peer_id();
    swarm
        .validator_mut(validator_peer_id)
        .unwrap()
        .start()
        .unwrap();

    // Verify that all nodes can catch up
    wait_for_all_nodes(&mut swarm).await;
}

/// Update the specified node's config and restart the node
fn update_node_config_and_restart(node: &mut LocalNode, mut config: NodeConfig) {
    // Stop the node
    node.stop();

    // Update the node's config
    let node_path = node.config_path();
    config.save_to_path(node_path).unwrap();

    // Restart the node
    node.start().unwrap();
}
