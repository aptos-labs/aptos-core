// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    smoke_test_environment::SwarmBuilder,
    state_sync_utils,
    state_sync_utils::enable_consensus_observer,
    utils::{create_test_accounts, execute_transactions, wait_for_all_nodes},
};
use aptos_config::config::NodeConfig;
use aptos_forge::NodeExt;
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
async fn test_consensus_observer_fullnode_sync() {
    // Create a validator swarm of 1 validator with consensus observer enabled
    let mut swarm = SwarmBuilder::new_local(1)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| {
            enable_consensus_observer(true, config);
        }))
        .build()
        .await;

    // Create a fullnode config that uses consensus observer
    let mut vfn_config = NodeConfig::get_default_vfn_config();
    enable_consensus_observer(true, &mut vfn_config);

    // Create the fullnode
    let _ = state_sync_utils::create_fullnode(vfn_config, &mut swarm).await;

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

    // Verify the fullnode is up-to-date
    wait_for_all_nodes(&mut swarm).await;
}

#[tokio::test]
async fn test_consensus_observer_fullnode_restart() {
    // Create a validator swarm of 1 validator with consensus observer enabled
    let mut swarm = SwarmBuilder::new_local(1)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| {
            enable_consensus_observer(true, config);
        }))
        .build()
        .await;

    // Create a fullnode config that uses consensus observer
    let mut vfn_config = NodeConfig::get_default_vfn_config();
    enable_consensus_observer(true, &mut vfn_config);

    // Create the fullnode and test its ability to stay up-to-date
    let vfn_peer_id = state_sync_utils::create_fullnode(vfn_config, &mut swarm).await;
    state_sync_utils::test_fullnode_sync(vfn_peer_id, &mut swarm, true, false).await;
}

#[tokio::test]
async fn test_consensus_observer_fullnode_restart_wipe() {
    // Create a validator swarm of 1 validator with consensus observer enabled
    let mut swarm = SwarmBuilder::new_local(1)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| {
            enable_consensus_observer(true, config);
        }))
        .build()
        .await;

    // Create a fullnode config that uses consensus observer
    let mut vfn_config = NodeConfig::get_default_vfn_config();
    enable_consensus_observer(true, &mut vfn_config);

    // Create the fullnode and test its ability to stay up-to-date (after a data wipe)
    let vfn_peer_id = state_sync_utils::create_fullnode(vfn_config, &mut swarm).await;
    state_sync_utils::test_fullnode_sync(vfn_peer_id, &mut swarm, true, true).await;
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
    // Create a validator swarm of 4 validators with consensus observer enabled
    let mut swarm = SwarmBuilder::new_local(4)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| {
            enable_consensus_observer(true, config);
        }))
        .build()
        .await;

    // Create a fullnode config that uses consensus observer
    let mut vfn_config = NodeConfig::get_default_vfn_config();
    enable_consensus_observer(true, &mut vfn_config);

    // Create the fullnode (i.e., a VFN for the first validator)
    let _ = state_sync_utils::create_fullnode(vfn_config, &mut swarm).await;

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
