// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    smoke_test_environment::new_local_swarm,
    test_utils::{create_and_fund_account, transfer_and_reconfig, transfer_coins},
};
use diem_config::config::{BootstrappingMode, ContinuousSyncingMode, NodeConfig};
use diem_rest_client::Client as RestClient;
use diem_sdk::{transaction_builder::TransactionFactory, types::LocalAccount};
use forge::{LocalSwarm, NodeExt, Swarm, SwarmExt};
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;

const MAX_CATCH_UP_SECS: u64 = 60; // The max time we'll wait for nodes to catch up

#[test]
fn test_full_node_bootstrap_outputs() {
    // Create a validator swarm of 1 validator node
    let swarm = new_local_swarm(1);

    // Create a fullnode config that uses transaction outputs to sync
    let mut vfn_config = NodeConfig::default_for_validator_full_node();
    vfn_config.state_sync.state_sync_driver.enable_state_sync_v2 = true;
    vfn_config.state_sync.state_sync_driver.bootstrapping_mode =
        BootstrappingMode::ApplyTransactionOutputsFromGenesis;
    vfn_config
        .state_sync
        .state_sync_driver
        .continuous_syncing_mode = ContinuousSyncingMode::ApplyTransactionOutputs;

    // Test the ability of the fullnode to sync
    test_full_node_sync(vfn_config, swarm, true)
}

#[test]
fn test_full_node_bootstrap_transactions() {
    // Create a validator swarm of 1 validator node
    let swarm = new_local_swarm(1);

    // Create a fullnode config that uses transactions to sync
    let mut vfn_config = NodeConfig::default_for_validator_full_node();
    vfn_config.state_sync.state_sync_driver.enable_state_sync_v2 = true;
    vfn_config.state_sync.state_sync_driver.bootstrapping_mode =
        BootstrappingMode::ExecuteTransactionsFromGenesis;
    vfn_config
        .state_sync
        .state_sync_driver
        .continuous_syncing_mode = ContinuousSyncingMode::ExecuteTransactions;

    // Test the ability of the fullnode to sync
    test_full_node_sync(vfn_config, swarm, true)
}

#[test]
fn test_full_node_continuous_sync_outputs() {
    // Create a validator swarm of 1 validator node
    let swarm = new_local_swarm(1);

    // Create a fullnode config that uses transaction outputs to sync
    let mut vfn_config = NodeConfig::default_for_validator_full_node();
    vfn_config.state_sync.state_sync_driver.enable_state_sync_v2 = true;
    vfn_config
        .state_sync
        .state_sync_driver
        .continuous_syncing_mode = ContinuousSyncingMode::ApplyTransactionOutputs;

    // Test the ability of the fullnode to sync
    test_full_node_sync(vfn_config, swarm, false)
}

#[test]
fn test_full_node_continuous_sync_transactions() {
    // Create a validator swarm of 1 validator node
    let swarm = new_local_swarm(1);

    // Create a fullnode config that uses transactions to sync
    let mut vfn_config = NodeConfig::default_for_validator_full_node();
    vfn_config.state_sync.state_sync_driver.enable_state_sync_v2 = true;
    vfn_config
        .state_sync
        .state_sync_driver
        .continuous_syncing_mode = ContinuousSyncingMode::ExecuteTransactions;

    // Test the ability of the fullnode to sync
    test_full_node_sync(vfn_config, swarm, false)
}

/// A helper method that tests that a full node can sync from a validator after
/// a failure and continue to stay up-to-date.
fn test_full_node_sync(full_node_config: NodeConfig, mut swarm: LocalSwarm, epoch_changes: bool) {
    // Create the fullnode
    let validator_peer_id = swarm.validators().next().unwrap().peer_id();
    let vfn_peer_id = swarm
        .add_validator_fullnode(
            &swarm.versions().max().unwrap(),
            full_node_config,
            validator_peer_id,
        )
        .unwrap();

    // Start the validator and fullnode (make sure they boot up)
    swarm
        .validator_mut(validator_peer_id)
        .unwrap()
        .wait_until_healthy(Instant::now() + Duration::from_secs(MAX_CATCH_UP_SECS))
        .unwrap();
    for fullnode in swarm.full_nodes_mut() {
        fullnode
            .wait_until_healthy(Instant::now() + Duration::from_secs(MAX_CATCH_UP_SECS))
            .unwrap();
    }

    // Stop the fullnode
    swarm.fullnode_mut(vfn_peer_id).unwrap().stop();

    // Execute a number of transactions on the validator
    let validator_client = swarm.validator(validator_peer_id).unwrap().rest_client();
    let transaction_factory = swarm.chain_info().transaction_factory();
    let mut account_0 = create_and_fund_account(&mut swarm, 1000);
    let mut account_1 = create_and_fund_account(&mut swarm, 1000);
    execute_transactions(
        &mut swarm,
        &validator_client,
        &transaction_factory,
        &mut account_0,
        &account_1,
        epoch_changes,
    );

    // Restart the fullnode and verify it can sync
    swarm.fullnode_mut(vfn_peer_id).unwrap().restart().unwrap();
    swarm
        .wait_for_all_nodes_to_catchup(Instant::now() + Duration::from_secs(MAX_CATCH_UP_SECS))
        .unwrap();

    // Execute more transactions on the validator and verify the fullnode catches up
    execute_transactions(
        &mut swarm,
        &validator_client,
        &transaction_factory,
        &mut account_1,
        &account_0,
        epoch_changes,
    );
    swarm
        .wait_for_all_nodes_to_catchup(Instant::now() + Duration::from_secs(MAX_CATCH_UP_SECS))
        .unwrap();
}

/// Executes transactions using the given transaction factory, client and
/// accounts. If `force_epoch_changes` is true, also execute transactions to
/// force reconfigurations.
fn execute_transactions(
    swarm: &mut LocalSwarm,
    client: &RestClient,
    transaction_factory: &TransactionFactory,
    sender: &mut LocalAccount,
    receiver: &LocalAccount,
    execute_epoch_changes: bool,
) {
    let runtime = Runtime::new().unwrap();
    let num_transfers = 10;
    if execute_epoch_changes {
        runtime.block_on(async {
            transfer_and_reconfig(
                client,
                transaction_factory,
                swarm.chain_info().root_account,
                sender,
                receiver,
                num_transfers,
            )
            .await;
        });
    } else {
        runtime.block_on(async {
            for _ in 0..num_transfers {
                // Execute simple transfer transactions
                transfer_coins(client, transaction_factory, sender, receiver, 1).await;
            }
        });
    }
}
