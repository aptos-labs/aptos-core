// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    smoke_test_environment::new_local_swarm_with_aptos,
    test_utils::{create_and_fund_account, transfer_and_reconfig, transfer_coins},
};
use aptos_config::config::{BootstrappingMode, ContinuousSyncingMode, NodeConfig};
use aptos_logger::info;
use aptos_rest_client::Client as RestClient;
use aptos_sdk::types::LocalAccount;
use aptos_types::{account_address::AccountAddress, PeerId};
use forge::{LocalSwarm, NodeExt, Swarm, SwarmExt};
use std::{
    fs,
    time::{Duration, Instant},
};

const MAX_CATCH_UP_SECS: u64 = 60; // The max time we'll wait for nodes to catch up

#[tokio::test]
async fn test_full_node_bootstrap_accounts() {
    // Create a validator swarm of 1 validator node
    let mut swarm = new_local_swarm_with_aptos(1).await;

    // Create a fullnode config that uses account state syncing
    let mut vfn_config = NodeConfig::default_for_validator_full_node();
    vfn_config.state_sync.state_sync_driver.enable_state_sync_v2 = true;
    vfn_config.state_sync.state_sync_driver.bootstrapping_mode =
        BootstrappingMode::DownloadLatestAccountStates;

    // Create (and stop) the fullnode
    let vfn_peer_id = create_full_node(vfn_config, &mut swarm).await;
    swarm.fullnode_mut(vfn_peer_id).unwrap().stop();

    // Set at most 2 accounts per storage request for the validator
    let validator = swarm.validators_mut().next().unwrap();
    let mut config = validator.config().clone();
    config
        .state_sync
        .storage_service
        .max_account_states_chunk_sizes = 2;
    config.save(validator.config_path()).unwrap();
    validator.restart().await.unwrap();
    validator
        .wait_until_healthy(Instant::now() + Duration::from_secs(MAX_CATCH_UP_SECS))
        .await
        .unwrap();

    // Test the ability of the fullnode to sync
    test_full_node_sync(vfn_peer_id, swarm, true).await;
}

#[tokio::test]
async fn test_full_node_bootstrap_outputs() {
    // Create a validator swarm of 1 validator node
    let mut swarm = new_local_swarm_with_aptos(1).await;

    // Create a fullnode config that uses transaction outputs to sync
    let mut vfn_config = NodeConfig::default_for_validator_full_node();
    vfn_config.state_sync.state_sync_driver.enable_state_sync_v2 = true;
    vfn_config.state_sync.state_sync_driver.bootstrapping_mode =
        BootstrappingMode::ApplyTransactionOutputsFromGenesis;
    vfn_config
        .state_sync
        .state_sync_driver
        .continuous_syncing_mode = ContinuousSyncingMode::ApplyTransactionOutputs;

    // Create the fullnode
    let vfn_peer_id = create_full_node(vfn_config, &mut swarm).await;

    // Test the ability of the fullnode to sync
    test_full_node_sync(vfn_peer_id, swarm, true).await;
}

#[tokio::test]
async fn test_full_node_bootstrap_transactions() {
    // Create a validator swarm of 1 validator node
    let mut swarm = new_local_swarm_with_aptos(1).await;

    // Create a fullnode config that uses transactions to sync
    let mut vfn_config = NodeConfig::default_for_validator_full_node();
    vfn_config.state_sync.state_sync_driver.enable_state_sync_v2 = true;
    vfn_config.state_sync.state_sync_driver.bootstrapping_mode =
        BootstrappingMode::ExecuteTransactionsFromGenesis;
    vfn_config
        .state_sync
        .state_sync_driver
        .continuous_syncing_mode = ContinuousSyncingMode::ExecuteTransactions;

    // Create the fullnode
    let vfn_peer_id = create_full_node(vfn_config, &mut swarm).await;

    // Test the ability of the fullnode to sync
    test_full_node_sync(vfn_peer_id, swarm, true).await;
}

#[tokio::test]
async fn test_full_node_continuous_sync_outputs() {
    // Create a validator swarm of 1 validator node
    let mut swarm = new_local_swarm_with_aptos(1).await;

    // Create a fullnode config that uses transaction outputs to sync
    let mut vfn_config = NodeConfig::default_for_validator_full_node();
    vfn_config.state_sync.state_sync_driver.enable_state_sync_v2 = true;
    vfn_config
        .state_sync
        .state_sync_driver
        .continuous_syncing_mode = ContinuousSyncingMode::ApplyTransactionOutputs;

    // Create the fullnode
    let vfn_peer_id = create_full_node(vfn_config, &mut swarm).await;

    // Test the ability of the fullnode to sync
    test_full_node_sync(vfn_peer_id, swarm, false).await;
}

#[tokio::test]
async fn test_full_node_continuous_sync_transactions() {
    // Create a validator swarm of 1 validator node
    let mut swarm = new_local_swarm_with_aptos(1).await;

    // Create a fullnode config that uses transactions to sync
    let mut vfn_config = NodeConfig::default_for_validator_full_node();
    vfn_config.state_sync.state_sync_driver.enable_state_sync_v2 = true;
    vfn_config
        .state_sync
        .state_sync_driver
        .continuous_syncing_mode = ContinuousSyncingMode::ExecuteTransactions;

    // Create the fullnode
    let vfn_peer_id = create_full_node(vfn_config, &mut swarm).await;

    // Test the ability of the fullnode to sync
    test_full_node_sync(vfn_peer_id, swarm, false).await;
}

/// Creates a new full node using the given config and swarm
async fn create_full_node(full_node_config: NodeConfig, swarm: &mut LocalSwarm) -> PeerId {
    let validator_peer_id = swarm.validators().next().unwrap().peer_id();
    let vfn_peer_id = swarm
        .add_validator_fullnode(
            &swarm.versions().max().unwrap(),
            full_node_config,
            validator_peer_id,
        )
        .await
        .unwrap();
    for fullnode in swarm.full_nodes_mut() {
        fullnode
            .wait_until_healthy(Instant::now() + Duration::from_secs(MAX_CATCH_UP_SECS))
            .await
            .unwrap();
    }
    vfn_peer_id
}

/// A helper method that tests that a full node can sync from a validator after
/// a failure and continue to stay up-to-date.
async fn test_full_node_sync(vfn_peer_id: PeerId, mut swarm: LocalSwarm, epoch_changes: bool) {
    // Stop the fullnode
    swarm.fullnode_mut(vfn_peer_id).unwrap().stop();

    // Execute a number of transactions on the validator
    let validator_peer_id = swarm.validators().next().unwrap().peer_id();
    let validator_client = swarm.validator(validator_peer_id).unwrap().rest_client();
    let (mut account_0, mut account_1) = create_test_accounts(&mut swarm).await;
    execute_transactions(
        &mut swarm,
        &validator_client,
        &mut account_0,
        &account_1,
        epoch_changes,
    )
    .await;

    // Restart the fullnode and verify it can sync
    swarm
        .fullnode_mut(vfn_peer_id)
        .unwrap()
        .restart()
        .await
        .unwrap();
    wait_for_all_nodes(&mut swarm).await;

    // Execute more transactions on the validator and verify the fullnode catches up
    execute_transactions_and_wait(
        &mut swarm,
        &validator_client,
        &mut account_1,
        &account_0,
        epoch_changes,
    )
    .await;
}

#[tokio::test]
async fn test_validator_bootstrap_outputs() {
    // Create a swarm of 4 validators with state sync v2 enabled (output syncing)
    let mut swarm = new_local_swarm_with_aptos(4).await;
    for validator in swarm.validators_mut() {
        let mut config = validator.config().clone();
        config.state_sync.state_sync_driver.enable_state_sync_v2 = true;
        config.state_sync.state_sync_driver.bootstrapping_mode =
            BootstrappingMode::ApplyTransactionOutputsFromGenesis;
        config.state_sync.state_sync_driver.continuous_syncing_mode =
            ContinuousSyncingMode::ApplyTransactionOutputs;
        config.save(validator.config_path()).unwrap();
        validator.restart().await.unwrap();
    }

    // Test the ability of the validators to sync
    test_validator_sync(swarm).await;
}

#[tokio::test]
async fn test_validator_bootstrap_transactions() {
    // Create a swarm of 4 validators with state sync v2 enabled (transaction syncing)
    let mut swarm = new_local_swarm_with_aptos(4).await;
    for validator in swarm.validators_mut() {
        let mut config = validator.config().clone();
        config.state_sync.state_sync_driver.enable_state_sync_v2 = true;
        config.state_sync.state_sync_driver.bootstrapping_mode =
            BootstrappingMode::ExecuteTransactionsFromGenesis;
        config.state_sync.state_sync_driver.continuous_syncing_mode =
            ContinuousSyncingMode::ExecuteTransactions;
        config.save(validator.config_path()).unwrap();
        validator.restart().await.unwrap();
    }

    // Test the ability of the validators to sync
    test_validator_sync(swarm).await;
}

/// A helper method that tests that a validator can sync after a failure and
/// continue to stay up-to-date.
async fn test_validator_sync(mut swarm: LocalSwarm) {
    // Launch the swarm and wait for it to be ready
    swarm.launch().await.unwrap();

    // Execute multiple transactions through validator 0
    let validator_peer_ids = swarm.validators().map(|v| v.peer_id()).collect::<Vec<_>>();
    let validator_client_0 = swarm
        .validator(validator_peer_ids[0])
        .unwrap()
        .rest_client();
    let (mut account_0, mut account_1) = create_test_accounts(&mut swarm).await;
    execute_transactions_and_wait(
        &mut swarm,
        &validator_client_0,
        &mut account_0,
        &account_1,
        true,
    )
    .await;

    // Stop validator 1 and delete the storage
    let validator_1 = validator_peer_ids[1];
    stop_validator_and_delete_storage(&mut swarm, validator_1);

    // Execute more transactions
    execute_transactions(
        &mut swarm,
        &validator_client_0,
        &mut account_1,
        &account_0,
        true,
    )
    .await;

    // Restart validator 1 and wait for all nodes to catchup
    swarm.validator_mut(validator_1).unwrap().start().unwrap();
    wait_for_all_nodes(&mut swarm).await;

    // Execute multiple transactions and verify validator 1 can sync
    execute_transactions_and_wait(
        &mut swarm,
        &validator_client_0,
        &mut account_0,
        &account_1,
        true,
    )
    .await;
}

#[tokio::test]
async fn test_validator_failure_bootstrap_outputs() {
    // Create a swarm of 4 validators with state sync v2 enabled (account
    // bootstrapping and transaction output application).
    let mut swarm = new_local_swarm_with_aptos(4).await;
    for validator in swarm.validators_mut() {
        let mut config = validator.config().clone();
        config.state_sync.state_sync_driver.enable_state_sync_v2 = true;
        config.state_sync.state_sync_driver.bootstrapping_mode =
            BootstrappingMode::DownloadLatestAccountStates;
        config.state_sync.state_sync_driver.continuous_syncing_mode =
            ContinuousSyncingMode::ApplyTransactionOutputs;
        config.save(validator.config_path()).unwrap();
        validator.restart().await.unwrap();
    }

    // Test the ability of the validators to sync
    test_all_validator_failures(swarm).await;
}

#[tokio::test]
async fn test_validator_failure_bootstrap_execution() {
    // Create a swarm of 4 validators with state sync v2 enabled (account
    // bootstrapping and transaction execution).
    let mut swarm = new_local_swarm_with_aptos(4).await;
    for validator in swarm.validators_mut() {
        let mut config = validator.config().clone();
        config.state_sync.state_sync_driver.enable_state_sync_v2 = true;
        config.state_sync.state_sync_driver.bootstrapping_mode =
            BootstrappingMode::DownloadLatestAccountStates;
        config.state_sync.state_sync_driver.continuous_syncing_mode =
            ContinuousSyncingMode::ExecuteTransactions;
        config.save(validator.config_path()).unwrap();
        validator.restart().await.unwrap();
    }

    // Test the ability of the validators to sync
    test_all_validator_failures(swarm).await;
}

/// A helper method that tests that all validators can sync after a failure and
/// continue to stay up-to-date.
async fn test_all_validator_failures(mut swarm: LocalSwarm) {
    // Launch the swarm and wait for it to be ready
    swarm.launch().await.unwrap();

    // Execute multiple transactions through validator 0
    let validator_peer_ids = swarm.validators().map(|v| v.peer_id()).collect::<Vec<_>>();
    let validator_0 = validator_peer_ids[0];
    let validator_client_0 = swarm.validator(validator_0).unwrap().rest_client();
    let (mut account_0, mut account_1) = create_test_accounts(&mut swarm).await;
    execute_transactions_and_wait(
        &mut swarm,
        &validator_client_0,
        &mut account_0,
        &account_1,
        true,
    )
    .await;

    // Go through each validator, stop the node, delete the storage and wait for it to come back
    for validator in validator_peer_ids.clone() {
        stop_validator_and_delete_storage(&mut swarm, validator);
        swarm.validator_mut(validator).unwrap().start().unwrap();
        wait_for_all_nodes(&mut swarm).await;
    }

    // Execute multiple transactions (no epoch changes) and verify validator 0 can sync
    execute_transactions_and_wait(
        &mut swarm,
        &validator_client_0,
        &mut account_1,
        &account_0,
        false,
    )
    .await;

    // Go through each validator, stop the node, delete the storage and wait for it to come back
    for validator in validator_peer_ids.clone() {
        stop_validator_and_delete_storage(&mut swarm, validator);
        swarm.validator_mut(validator).unwrap().start().unwrap();
        wait_for_all_nodes(&mut swarm).await;
    }

    // Execute multiple transactions (with epoch changes) and verify validator 0 can sync
    let validator_client_1 = swarm
        .validator(validator_peer_ids[0])
        .unwrap()
        .rest_client();
    execute_transactions_and_wait(
        &mut swarm,
        &validator_client_1,
        &mut account_1,
        &account_0,
        true,
    )
    .await;
}

#[tokio::test]
async fn test_single_validator_failure() {
    // Create a swarm of 1 validator
    let mut swarm = new_local_swarm_with_aptos(1).await;
    swarm.launch().await.unwrap();

    // Enable state sync v2 and reboot the node
    let validator = swarm.validators_mut().next().unwrap();
    let mut config = validator.config().clone();
    config.state_sync.state_sync_driver.enable_state_sync_v2 = true;
    config.save(validator.config_path()).unwrap();
    validator.stop();
    swarm.launch().await.unwrap();

    // Execute multiple transactions
    let validator = swarm.validators_mut().next().unwrap();
    let validator_client = validator.rest_client();
    let (mut account_0, mut account_1) = create_test_accounts(&mut swarm).await;
    execute_transactions(
        &mut swarm,
        &validator_client,
        &mut account_0,
        &account_1,
        true,
    )
    .await;

    // Restart the validator
    let validator = swarm.validators_mut().next().unwrap();
    validator.stop();
    swarm.launch().await.unwrap();

    // Execute more transactions
    execute_transactions(
        &mut swarm,
        &validator_client,
        &mut account_1,
        &account_0,
        true,
    )
    .await;
}

/// Executes transactions using the given transaction factory, client and
/// accounts. If `force_epoch_changes` is true, also execute transactions to
/// force reconfigurations.
async fn execute_transactions(
    swarm: &mut LocalSwarm,
    client: &RestClient,
    sender: &mut LocalAccount,
    receiver: &LocalAccount,
    execute_epoch_changes: bool,
) {
    let num_transfers = 10;

    let transaction_factory = swarm.chain_info().transaction_factory();
    if execute_epoch_changes {
        transfer_and_reconfig(
            client,
            &transaction_factory,
            swarm.chain_info().root_account,
            sender,
            receiver,
            num_transfers,
        )
        .await;
    } else {
        for _ in 0..num_transfers {
            // Execute simple transfer transactions
            transfer_coins(client, &transaction_factory, sender, receiver, 1).await;
        }
    }
}

/// Executes transactions and waits for all nodes to catch up
async fn execute_transactions_and_wait(
    swarm: &mut LocalSwarm,
    client: &RestClient,
    sender: &mut LocalAccount,
    receiver: &LocalAccount,
    epoch_changes: bool,
) {
    execute_transactions(swarm, client, sender, receiver, epoch_changes).await;
    wait_for_all_nodes(swarm).await;
}

/// Waits for all nodes to catch up
async fn wait_for_all_nodes(swarm: &mut LocalSwarm) {
    swarm
        .wait_for_all_nodes_to_catchup(Instant::now() + Duration::from_secs(MAX_CATCH_UP_SECS))
        .await
        .unwrap();
}

/// Creates and funds two test accounts
async fn create_test_accounts(swarm: &mut LocalSwarm) -> (LocalAccount, LocalAccount) {
    let token_amount = 1000;
    let account_0 = create_and_fund_account(swarm, token_amount).await;
    let account_1 = create_and_fund_account(swarm, token_amount).await;
    (account_0, account_1)
}

/// Stops the specified validator and deletes storage
fn stop_validator_and_delete_storage(swarm: &mut LocalSwarm, validator: AccountAddress) {
    // Stop the validator
    swarm.validator_mut(validator).unwrap().stop();

    // Delete the validator storage
    let node_config = swarm.validator_mut(validator).unwrap().config().clone();
    let state_db_path = node_config.storage.dir().join("aptosdb");
    info!(
        "Deleting state db path {:?} for validator {:?}",
        state_db_path.as_path(),
        validator
    );
    assert!(state_db_path.as_path().exists());
    fs::remove_dir_all(state_db_path).unwrap();
}
