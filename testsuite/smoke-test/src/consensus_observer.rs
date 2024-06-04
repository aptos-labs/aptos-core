// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    smoke_test_environment::SwarmBuilder,
    state_sync_utils,
    utils::{create_test_accounts, execute_transactions, wait_for_all_nodes},
};
use aptos_config::config::NodeConfig;
use aptos_forge::NodeExt;
use std::sync::Arc;

#[tokio::test]
async fn test_consensus_observer_fullnode_simple_sync() {
    // Create a validator swarm of 1 validator with consensus publisher enabled
    let mut swarm = SwarmBuilder::new_local(1)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| {
            config.consensus_observer.publisher_enabled = true;
        }))
        .build()
        .await;

    // Create a fullnode config that uses consensus observer
    let mut vfn_config = NodeConfig::get_default_vfn_config();
    vfn_config.consensus_observer.observer_enabled = true;

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
    // Create a validator swarm of 1 validator with consensus publisher enabled
    let mut swarm = SwarmBuilder::new_local(1)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| {
            config.consensus_observer.publisher_enabled = true;
        }))
        .build()
        .await;

    // Create a fullnode config that uses consensus observer
    let mut vfn_config = NodeConfig::get_default_vfn_config();
    vfn_config.consensus_observer.observer_enabled = true;

    // Create the fullnode and test its ability to stay up-to-date
    let vfn_peer_id = state_sync_utils::create_fullnode(vfn_config, &mut swarm).await;
    state_sync_utils::test_fullnode_sync(vfn_peer_id, &mut swarm, true, false).await;
}

#[tokio::test]
async fn test_consensus_observer_fullnode_restart_wipe() {
    // Create a validator swarm of 1 validator with consensus publisher enabled
    let mut swarm = SwarmBuilder::new_local(1)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| {
            config.consensus_observer.publisher_enabled = true;
        }))
        .build()
        .await;

    // Create a fullnode config that uses consensus observer
    let mut vfn_config = NodeConfig::get_default_vfn_config();
    vfn_config.consensus_observer.observer_enabled = true;

    // Create the fullnode and test its ability to stay up-to-date (after a data wipe)
    let vfn_peer_id = state_sync_utils::create_fullnode(vfn_config, &mut swarm).await;
    state_sync_utils::test_fullnode_sync(vfn_peer_id, &mut swarm, true, true).await;
}
