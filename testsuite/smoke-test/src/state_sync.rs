// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    smoke_test_environment::{new_local_swarm_with_aptos, SwarmBuilder},
    test_utils::{
        create_test_accounts, execute_transactions, execute_transactions_and_wait,
        wait_for_all_nodes, MAX_CATCH_UP_WAIT_SECS, MAX_HEALTHY_WAIT_SECS,
    },
};
use aptos_config::config::{
    BootstrappingMode, ContinuousSyncingMode, NodeConfig, OverrideNodeConfig,
};
use aptos_db::AptosDB;
use aptos_forge::{LocalNode, LocalSwarm, Node, NodeExt, Swarm};
use aptos_inspection_service::inspection_client::InspectionClient;
use aptos_rest_client::Client as RestClient;
use aptos_storage_interface::DbReader;
use aptos_types::{
    account_address::AccountAddress,
    on_chain_config::{
        ConsensusConfigV1, ConsensusConfigV1Ext, LeaderReputationType, OnChainConsensusConfig,
        ProposerAndVoterConfig, ProposerElectionType,
    },
    PeerId,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

// TODO: Go through the existing tests, identify any gaps, and clean up the rest.

#[tokio::test]
async fn test_fullnode_fast_sync_epoch_changes() {
    // Test fast syncing in the presence of epoch changes
    test_fullnode_fast_sync(true).await;
}

#[tokio::test]
async fn test_fullnode_fast_sync_no_epoch_changes() {
    // Test fast syncing without epoch changes
    test_fullnode_fast_sync(false).await;
}

#[tokio::test]
async fn test_fullnode_output_sync_epoch_changes() {
    // Create a validator swarm of 1 validator node
    let mut swarm = new_local_swarm_with_aptos(1).await;

    // Create a fullnode config that uses transaction outputs to sync
    let mut vfn_config = NodeConfig::get_default_vfn_config();
    vfn_config.state_sync.state_sync_driver.bootstrapping_mode =
        BootstrappingMode::ApplyTransactionOutputsFromGenesis;
    vfn_config
        .state_sync
        .state_sync_driver
        .continuous_syncing_mode = ContinuousSyncingMode::ApplyTransactionOutputs;
    vfn_config.state_sync.aptos_data_client.use_compression = true;

    // Create the fullnode and test its ability to sync
    let vfn_peer_id = create_fullnode(vfn_config, &mut swarm).await;
    test_fullnode_sync(vfn_peer_id, &mut swarm, true, false).await;
}

#[tokio::test]
async fn test_fullnode_output_sync_no_compression() {
    // Create a validator swarm of 1 validator node
    let mut swarm = new_local_swarm_with_aptos(1).await;

    // Create a fullnode config that uses transaction outputs to sync (without compression)
    let mut vfn_config = NodeConfig::get_default_vfn_config();
    vfn_config.state_sync.state_sync_driver.bootstrapping_mode =
        BootstrappingMode::ApplyTransactionOutputsFromGenesis;
    vfn_config
        .state_sync
        .state_sync_driver
        .continuous_syncing_mode = ContinuousSyncingMode::ApplyTransactionOutputs;
    vfn_config.state_sync.aptos_data_client.use_compression = false;

    // Create the fullnode and test its ability to sync
    let vfn_peer_id = create_fullnode(vfn_config, &mut swarm).await;
    test_fullnode_sync(vfn_peer_id, &mut swarm, true, false).await;
}

#[tokio::test]
async fn test_fullnode_output_sync_exponential_backoff() {
    // Create a validator swarm of 1 validator node
    let mut swarm = new_local_swarm_with_aptos(1).await;

    // Create a fullnode config that uses transaction outputs to sync with a small timeout
    let mut vfn_config = NodeConfig::get_default_vfn_config();
    vfn_config.state_sync.state_sync_driver.bootstrapping_mode =
        BootstrappingMode::ApplyTransactionOutputsFromGenesis;
    vfn_config
        .state_sync
        .state_sync_driver
        .continuous_syncing_mode = ContinuousSyncingMode::ApplyTransactionOutputs;
    vfn_config.state_sync.aptos_data_client.response_timeout_ms = 1;

    // Create the fullnode and test its ability to sync
    let vfn_peer_id = create_fullnode(vfn_config, &mut swarm).await;
    test_fullnode_sync(vfn_peer_id, &mut swarm, true, false).await;
}

#[tokio::test]
async fn test_fullnode_intelligent_sync_epoch_changes() {
    // Create a validator swarm of 1 validator node with a small network limit
    let mut swarm = SwarmBuilder::new_local(1)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| {
            config.state_sync.storage_service.max_network_chunk_bytes = 5 * 1024;
        }))
        .build()
        .await;

    // Create a fullnode config that uses transactions or outputs to sync
    let mut vfn_config = NodeConfig::get_default_vfn_config();
    vfn_config.state_sync.state_sync_driver.bootstrapping_mode =
        BootstrappingMode::ExecuteOrApplyFromGenesis;
    vfn_config
        .state_sync
        .state_sync_driver
        .continuous_syncing_mode = ContinuousSyncingMode::ExecuteTransactionsOrApplyOutputs;
    vfn_config
        .state_sync
        .aptos_data_client
        .max_num_output_reductions = 1;
    vfn_config.state_sync.aptos_data_client.response_timeout_ms = 1;

    // Create the fullnode and test its ability to sync
    let vfn_peer_id = create_fullnode(vfn_config, &mut swarm).await;
    test_fullnode_sync(vfn_peer_id, &mut swarm, true, false).await;
}

#[tokio::test]
async fn test_fullnode_fast_and_intelligent_sync_epoch_changes() {
    // Create a validator swarm of 1 validator node with a small network limit
    let mut swarm = SwarmBuilder::new_local(1)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| {
            config.state_sync.storage_service.max_network_chunk_bytes = 500 * 1024;
        }))
        .build()
        .await;

    // Create a fullnode config that uses fast and intelligent syncing
    let mut vfn_config = NodeConfig::get_default_vfn_config();
    vfn_config.state_sync.state_sync_driver.bootstrapping_mode =
        BootstrappingMode::DownloadLatestStates;
    vfn_config
        .state_sync
        .state_sync_driver
        .continuous_syncing_mode = ContinuousSyncingMode::ExecuteTransactionsOrApplyOutputs;
    vfn_config
        .state_sync
        .aptos_data_client
        .max_num_output_reductions = 2;
    vfn_config.state_sync.aptos_data_client.response_timeout_ms = 1;

    // Create the fullnode and test its ability to sync
    let vfn_peer_id = create_fullnode(vfn_config, &mut swarm).await;
    test_fullnode_sync(vfn_peer_id, &mut swarm, true, false).await;
}

#[tokio::test]
async fn test_fullnode_execution_sync_epoch_changes() {
    // Create a validator swarm of 1 validator node
    let mut swarm = new_local_swarm_with_aptos(1).await;

    // Create a fullnode config that uses transactions to sync
    let mut vfn_config = NodeConfig::get_default_vfn_config();
    vfn_config.state_sync.state_sync_driver.bootstrapping_mode =
        BootstrappingMode::ExecuteTransactionsFromGenesis;
    vfn_config
        .state_sync
        .state_sync_driver
        .continuous_syncing_mode = ContinuousSyncingMode::ExecuteTransactions;
    vfn_config.state_sync.aptos_data_client.use_compression = true;

    // Create the fullnode and test its ability to sync
    let vfn_peer_id = create_fullnode(vfn_config, &mut swarm).await;
    test_fullnode_sync(vfn_peer_id, &mut swarm, true, false).await;
}

#[tokio::test]
async fn test_fullnode_output_sync_no_epoch_changes() {
    // Create a validator swarm of 1 validator node
    let mut swarm = new_local_swarm_with_aptos(1).await;

    // Create a fullnode config that uses transaction outputs to sync
    let mut vfn_config = NodeConfig::get_default_vfn_config();
    vfn_config
        .state_sync
        .state_sync_driver
        .continuous_syncing_mode = ContinuousSyncingMode::ApplyTransactionOutputs;

    // Create the fullnode and test its ability to sync
    let vfn_peer_id = create_fullnode(vfn_config, &mut swarm).await;
    test_fullnode_sync(vfn_peer_id, &mut swarm, false, false).await;
}

#[tokio::test]
async fn test_fullnode_execution_sync_no_epoch_changes() {
    // Create a validator swarm of 1 validator node
    let mut swarm = new_local_swarm_with_aptos(1).await;

    // Create a fullnode config that uses transactions to sync
    let mut vfn_config = NodeConfig::get_default_vfn_config();
    vfn_config
        .state_sync
        .state_sync_driver
        .continuous_syncing_mode = ContinuousSyncingMode::ExecuteTransactions;

    // Create the fullnode and test its ability to sync
    let vfn_peer_id = create_fullnode(vfn_config, &mut swarm).await;
    test_fullnode_sync(vfn_peer_id, &mut swarm, false, false).await;
}

#[tokio::test]
async fn test_single_validator_reboot() {
    // Create a swarm of 1 validator
    let mut swarm = new_local_swarm_with_aptos(1).await;

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
    validator.start().unwrap();
    swarm
        .wait_all_alive(Duration::from_secs(MAX_CATCH_UP_WAIT_SECS))
        .await
        .unwrap();

    // Execute more transactions
    execute_transactions_and_wait(
        &mut swarm,
        &validator_client,
        &mut account_1,
        &account_0,
        true,
    )
    .await;
}

#[tokio::test]
async fn test_validator_output_sync_epoch_changes() {
    // Create a swarm of 4 validators using output syncing
    let mut swarm = SwarmBuilder::new_local(4)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| {
            config.state_sync.state_sync_driver.bootstrapping_mode =
                BootstrappingMode::ApplyTransactionOutputsFromGenesis;
            config.state_sync.state_sync_driver.continuous_syncing_mode =
                ContinuousSyncingMode::ApplyTransactionOutputs;
        }))
        .build()
        .await;

    // Test the ability of the validator to sync
    test_validator_sync(&mut swarm, 1, true).await;
}

#[tokio::test]
async fn test_validator_sync_and_participate_epoch_changes() {
    // Test the default syncing method with epoch changes
    test_validator_sync_and_participate(false, true).await;
}

#[tokio::test]
async fn test_validator_sync_and_participate_no_epoch_changes() {
    // Test the default syncing method without epoch changes
    test_validator_sync_and_participate(false, false).await;
}

#[tokio::test]
async fn test_validator_fast_sync_and_participate_epoch_changes() {
    // Test fast syncing with epoch changes
    test_validator_sync_and_participate(true, true).await;
}

#[tokio::test]
async fn test_validator_fast_sync_and_participate_no_epoch_changes() {
    // Test fast syncing without epoch changes
    test_validator_sync_and_participate(true, false).await;
}

#[ignore] // Ignore this test because it takes a long time. But, it works so it shouldn't be removed.
#[tokio::test]
async fn test_validator_output_sync_small_network_limit() {
    // Create a swarm of 4 validators using output syncing and an aggressive network limit
    let mut swarm = SwarmBuilder::new_local(4)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| {
            config.state_sync.state_sync_driver.bootstrapping_mode =
                BootstrappingMode::ApplyTransactionOutputsFromGenesis;
            config.state_sync.state_sync_driver.continuous_syncing_mode =
                ContinuousSyncingMode::ApplyTransactionOutputs;
            config.state_sync.storage_service.max_network_chunk_bytes = 100 * 1024;
        }))
        .build()
        .await;

    // Test the ability of the validator to sync
    test_validator_sync(&mut swarm, 1, true).await;
}

#[ignore] // Ignore this test because it takes a long time. But, it works so it shouldn't be removed.
#[tokio::test]
async fn test_validator_output_sync_unrealistic_network_limit() {
    // Create a swarm of 4 validators using output syncing and an unrealistic network limit.
    // This forces all chunks to be of size 1.
    let mut swarm = SwarmBuilder::new_local(4)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| {
            config.state_sync.state_sync_driver.bootstrapping_mode =
                BootstrappingMode::ApplyTransactionOutputsFromGenesis;
            config.state_sync.state_sync_driver.continuous_syncing_mode =
                ContinuousSyncingMode::ApplyTransactionOutputs;
            config.state_sync.storage_service.max_network_chunk_bytes = 1;
        }))
        .build()
        .await;

    // Test the ability of the validator to sync
    test_validator_sync(&mut swarm, 1, true).await;
}

#[tokio::test]
async fn test_validator_fast_sync_no_compression() {
    // Create a swarm of 4 validators using fast syncing and no compression
    let mut swarm = SwarmBuilder::new_local(4)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| {
            config.state_sync.state_sync_driver.bootstrapping_mode =
                BootstrappingMode::DownloadLatestStates;
            config.state_sync.state_sync_driver.continuous_syncing_mode =
                ContinuousSyncingMode::ApplyTransactionOutputs;
            config.state_sync.aptos_data_client.use_compression = false;
        }))
        .build()
        .await;

    // Test the ability of the validator to sync
    test_validator_sync(&mut swarm, 1, true).await;
}

#[ignore] // Ignore this test because it takes a long time. But, it works so it shouldn't be removed.
#[tokio::test]
async fn test_validator_fast_sync_small_network_limit() {
    // Create a swarm of 4 validators using fast sync and an aggressive network limit
    let mut swarm = SwarmBuilder::new_local(4)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| {
            config.state_sync.state_sync_driver.bootstrapping_mode =
                BootstrappingMode::DownloadLatestStates;
            config.state_sync.state_sync_driver.continuous_syncing_mode =
                ContinuousSyncingMode::ExecuteTransactions;
            config.state_sync.storage_service.max_network_chunk_bytes = 200 * 1024;
        }))
        .build()
        .await;

    // Test the ability of the validator to sync
    test_validator_sync(&mut swarm, 1, true).await;
}

#[ignore] // Ignore this test because it takes a long time. But, it works so it shouldn't be removed.
#[tokio::test]
async fn test_validator_fast_sync_unrealistic_network_limit() {
    // Create a swarm of 4 validators using fast sync and an unrealistic network limit.
    // This forces all chunks to be of size 1.
    let mut swarm = SwarmBuilder::new_local(4)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| {
            config.state_sync.state_sync_driver.bootstrapping_mode =
                BootstrappingMode::DownloadLatestStates;
            config.state_sync.state_sync_driver.continuous_syncing_mode =
                ContinuousSyncingMode::ExecuteTransactions;
            config.state_sync.storage_service.max_network_chunk_bytes = 1;
        }))
        .build()
        .await;

    // Test the ability of the validator to sync
    test_validator_sync(&mut swarm, 1, true).await;
}

#[tokio::test]
async fn test_validator_fast_sync_exponential_backoff_epoch_changes() {
    // Test fast syncing with exponential backoff and epoch changes
    test_validator_sync_exponential_backoff(true).await;
}

#[tokio::test]
async fn test_validator_fast_sync_exponential_backoff_no_epoch_changes() {
    // Test fast syncing without exponential backoff and no epoch changes
    test_validator_sync_exponential_backoff(false).await;
}

#[tokio::test]
async fn test_validator_execution_sync_epoch_changes() {
    // Create a swarm of 4 validators using transaction syncing
    let mut swarm = SwarmBuilder::new_local(4)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| {
            config.state_sync.state_sync_driver.bootstrapping_mode =
                BootstrappingMode::ExecuteTransactionsFromGenesis;
            config.state_sync.state_sync_driver.continuous_syncing_mode =
                ContinuousSyncingMode::ExecuteTransactions;
        }))
        .build()
        .await;

    // Test the ability of the validator to sync
    test_validator_sync(&mut swarm, 1, true).await;
}

#[tokio::test]
async fn test_validator_intelligent_sync_epoch_changes() {
    // Create a swarm of 4 validators using transaction or output syncing
    let mut swarm = SwarmBuilder::new_local(4)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| {
            config.state_sync.state_sync_driver.bootstrapping_mode =
                BootstrappingMode::ExecuteOrApplyFromGenesis;
            config.state_sync.state_sync_driver.continuous_syncing_mode =
                ContinuousSyncingMode::ExecuteTransactionsOrApplyOutputs;
            config.state_sync.storage_service.max_network_chunk_bytes = 10 * 1024;
            config
                .state_sync
                .aptos_data_client
                .max_num_output_reductions = 1;
            config.state_sync.aptos_data_client.response_timeout_ms = 1;
        }))
        .build()
        .await;

    // Test the ability of the validator to sync
    test_validator_sync(&mut swarm, 1, true).await;
}

#[ignore] // Ignore this test because it takes a long time. But, it works so it shouldn't be removed.
#[tokio::test]
async fn test_validator_execution_sync_small_network_limits() {
    // Create a swarm of 4 validators using transaction syncing and an aggressive network limit
    let mut swarm = SwarmBuilder::new_local(4)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| {
            config.state_sync.state_sync_driver.bootstrapping_mode =
                BootstrappingMode::ExecuteTransactionsFromGenesis;
            config.state_sync.state_sync_driver.continuous_syncing_mode =
                ContinuousSyncingMode::ExecuteTransactions;
            config.state_sync.storage_service.max_network_chunk_bytes = 100 * 1024;
        }))
        .build()
        .await;

    // Test the ability of the validator to sync
    test_validator_sync(&mut swarm, 1, true).await;
}

#[ignore] // Ignore this test because it takes a long time. But, it works so it shouldn't be removed.
#[tokio::test]
async fn test_validator_execution_sync_unrealistic_network_limits() {
    // Create a swarm of 4 validators using transaction syncing and an unrealistic network limit.
    // This forces all chunks to be of size 1.
    let mut swarm = SwarmBuilder::new_local(4)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| {
            config.state_sync.state_sync_driver.bootstrapping_mode =
                BootstrappingMode::ExecuteTransactionsFromGenesis;
            config.state_sync.state_sync_driver.continuous_syncing_mode =
                ContinuousSyncingMode::ExecuteTransactions;
            config.state_sync.storage_service.max_network_chunk_bytes = 1;
        }))
        .build()
        .await;

    // Test the ability of the validator to sync
    test_validator_sync(&mut swarm, 1, true).await;
}

#[tokio::test]
async fn test_validator_output_sync_exponential_backoff() {
    // Create a swarm of 4 validators using output syncing and a small response timeout
    let mut swarm = SwarmBuilder::new_local(4)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| {
            config.state_sync.state_sync_driver.bootstrapping_mode =
                BootstrappingMode::ApplyTransactionOutputsFromGenesis;
            config.state_sync.state_sync_driver.continuous_syncing_mode =
                ContinuousSyncingMode::ApplyTransactionOutputs;
            config.state_sync.aptos_data_client.response_timeout_ms = 1;
        }))
        .build()
        .await;

    // Test the ability of the validator to sync
    test_validator_sync(&mut swarm, 1, true).await;
}

#[tokio::test]
async fn test_validator_execution_sync_no_compression() {
    // Create a swarm of 4 validators using transaction syncing and no compression
    let mut swarm = SwarmBuilder::new_local(4)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| {
            config.state_sync.state_sync_driver.bootstrapping_mode =
                BootstrappingMode::ExecuteTransactionsFromGenesis;
            config.state_sync.state_sync_driver.continuous_syncing_mode =
                ContinuousSyncingMode::ExecuteTransactions;
            config.state_sync.aptos_data_client.use_compression = false;
        }))
        .build()
        .await;

    // Test the ability of the validator to sync
    test_validator_sync(&mut swarm, 1, true).await;
}

#[ignore] // Ignore this test because it takes a long time. But, it works so it shouldn't be removed.
#[tokio::test]
async fn test_all_validators_fast_and_output_sync() {
    // Create a swarm of 4 validators with fast and output syncing
    let swarm = SwarmBuilder::new_local(4)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| {
            config.state_sync.state_sync_driver.bootstrapping_mode =
                BootstrappingMode::DownloadLatestStates;
            config.state_sync.state_sync_driver.continuous_syncing_mode =
                ContinuousSyncingMode::ApplyTransactionOutputs;
        }))
        .build()
        .await;

    // Test the ability of all validators to sync
    test_all_validator_failures(swarm).await;
}

#[ignore] // Ignore this test because it takes a long time. But, it works so it shouldn't be removed.
#[tokio::test]
async fn test_all_validators_fast_and_execution_sync() {
    // Create a swarm of 4 validators with fast and execution syncing
    let swarm = SwarmBuilder::new_local(4)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| {
            config.state_sync.state_sync_driver.bootstrapping_mode =
                BootstrappingMode::DownloadLatestStates;
            config.state_sync.state_sync_driver.continuous_syncing_mode =
                ContinuousSyncingMode::ExecuteTransactions;
        }))
        .build()
        .await;

    // Test the ability of all validators to sync
    test_all_validator_failures(swarm).await;
}

/// Creates a new full node using the given config and swarm
async fn create_fullnode(full_node_config: NodeConfig, swarm: &mut LocalSwarm) -> PeerId {
    let validator_peer_id = swarm.validators().next().unwrap().peer_id();
    let vfn_peer_id = swarm
        .add_validator_fullnode(
            &swarm.versions().max().unwrap(),
            OverrideNodeConfig::new_with_default_base(full_node_config),
            validator_peer_id,
        )
        .unwrap();
    for fullnode in swarm.full_nodes_mut() {
        fullnode
            .wait_until_healthy(Instant::now() + Duration::from_secs(MAX_HEALTHY_WAIT_SECS))
            .await
            .unwrap();
    }
    vfn_peer_id
}

/// A test method that verifies that a fullnode can fast sync from
/// a validator after a data wipe. If `epoch_changes` are enabled
/// then epoch changes can occur during test execution and fullnode syncing.
async fn test_fullnode_fast_sync(epoch_changes: bool) {
    // Create a swarm with 2 validators
    let mut swarm = SwarmBuilder::new_local(2)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| {
            config.state_sync.state_sync_driver.bootstrapping_mode =
                BootstrappingMode::DownloadLatestStates;
        }))
        .with_init_genesis_config(Arc::new(|genesis_config| {
            genesis_config.epoch_duration_secs = 10_000; // Prevent epoch changes from occurring unnecessarily
        }))
        .build()
        .await;

    // Verify the oldest ledger info and pruning metrics for the validators
    for validator in swarm.validators_mut() {
        verify_fast_sync_version_and_metrics(validator, true).await;
    }

    // Create a fullnode config that uses fast syncing
    let mut vfn_config = NodeConfig::get_default_vfn_config();
    vfn_config.state_sync.state_sync_driver.bootstrapping_mode =
        BootstrappingMode::DownloadLatestStates;

    // Test the ability of a fullnode to sync
    let vfn_peer_id = create_fullnode(vfn_config, &mut swarm).await;
    test_fullnode_sync(vfn_peer_id, &mut swarm, epoch_changes, true).await;

    // Verify the oldest ledger info and pruning metrics for the fullnode
    if epoch_changes {
        let fullnode = swarm.fullnode_mut(vfn_peer_id).unwrap();
        verify_fast_sync_version_and_metrics(fullnode, false).await;
    }
}

/// A test method that verifies that a validator can fast sync from other
/// validators after a data wipe and while requiring exponential backoff.
/// If `epoch_changes` are enabled then epoch changes can occur during
/// test execution and validator syncing.
async fn test_validator_sync_exponential_backoff(epoch_changes: bool) {
    // Create a swarm of 4 validators using fast sync and a small response timeout
    let mut swarm = SwarmBuilder::new_local(4)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| {
            config.state_sync.state_sync_driver.bootstrapping_mode =
                BootstrappingMode::DownloadLatestStates;
            config.state_sync.state_sync_driver.continuous_syncing_mode =
                ContinuousSyncingMode::ApplyTransactionOutputs;
            config.state_sync.aptos_data_client.use_compression = false;
            config.state_sync.aptos_data_client.response_timeout_ms = 1;
        }))
        .with_init_genesis_config(Arc::new(|genesis_config| {
            genesis_config.epoch_duration_secs = 10_000; // Prevent epoch changes from occurring unnecessarily
        }))
        .build()
        .await;

    // Test the ability of the validator to sync
    test_validator_sync(&mut swarm, 1, epoch_changes).await;
}

/// A helper method that tests that a full node can sync from a validator after
/// a failure and continue to stay up-to-date.
async fn test_fullnode_sync(
    vfn_peer_id: PeerId,
    swarm: &mut LocalSwarm,
    epoch_changes: bool,
    clear_storage: bool,
) {
    // Stop the fullnode and potentially clear storage
    if clear_storage {
        stop_fullnode_and_delete_storage(swarm, vfn_peer_id).await;
    } else {
        swarm.fullnode_mut(vfn_peer_id).unwrap().stop();
    }

    // Execute a number of transactions on the validator
    let validator_peer_id = swarm.validators().next().unwrap().peer_id();
    let validator_client = swarm.validator(validator_peer_id).unwrap().rest_client();
    let (mut account_0, mut account_1) = create_test_accounts(swarm).await;
    execute_transactions(
        swarm,
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
    wait_for_all_nodes(swarm).await;

    // Execute more transactions on the validator and verify the fullnode catches up
    execute_transactions_and_wait(
        swarm,
        &validator_client,
        &mut account_1,
        &account_0,
        epoch_changes,
    )
    .await;
}

/// A helper method that tests that a validator can sync after a failure and
/// continue to stay up-to-date.
async fn test_validator_sync(
    swarm: &mut LocalSwarm,
    validator_index_to_test: usize,
    epoch_changes: bool,
) {
    // Execute multiple transactions through validator 0
    let validator_peer_ids = swarm.validators().map(|v| v.peer_id()).collect::<Vec<_>>();
    let validator_client_0 = swarm
        .validator(validator_peer_ids[0])
        .unwrap()
        .rest_client();
    let (mut account_0, mut account_1) = create_test_accounts(swarm).await;
    execute_transactions_and_wait(
        swarm,
        &validator_client_0,
        &mut account_0,
        &account_1,
        epoch_changes,
    )
    .await;

    // Stop the specified validator and delete the storage
    let validator = validator_peer_ids[validator_index_to_test];
    stop_validator_and_delete_storage(swarm, validator).await;

    // Execute more transactions
    execute_transactions(
        swarm,
        &validator_client_0,
        &mut account_1,
        &account_0,
        epoch_changes,
    )
    .await;

    // Restart the validator and wait for all nodes to catchup
    swarm.validator_mut(validator).unwrap().start().unwrap();
    wait_for_all_nodes(swarm).await;

    // Execute multiple transactions and verify the validator
    // can sync and that consensus is still running.
    execute_transactions_and_wait(
        swarm,
        &validator_client_0,
        &mut account_0,
        &account_1,
        epoch_changes,
    )
    .await;
}

/// A test method that verifies that a validator can sync after a data wipe
/// and begin to participate in consensus. If `epoch_changes` are enabled
/// then epoch changes can occur during test execution and validator syncing.
/// If `fast_sync` is true, then the validator will use fast (snapshot)
/// syncing. Otherwise, it will use the default syncing method.
async fn test_validator_sync_and_participate(fast_sync: bool, epoch_changes: bool) {
    // Create a swarm of 4 validators
    let num_validators = 4;
    let mut swarm = SwarmBuilder::new_local(num_validators)
        .with_aptos()
        .with_init_config(Arc::new(move |_, config, _| {
            if fast_sync {
                // Set the bootstrapping mode to fast syncing
                config.state_sync.state_sync_driver.bootstrapping_mode =
                    BootstrappingMode::DownloadLatestStates;
                config.state_sync.storage_service.max_state_chunk_size = 30;
            }
        }))
        .with_init_genesis_config(Arc::new(|genesis_config| {
            // Shorten the required proposer history to speed up the test
            let consensus_config = match genesis_config.consensus_config.clone() {
                OnChainConsensusConfig::V1(consensus_config) => consensus_config,
                OnChainConsensusConfig::V2(consensus_config) => consensus_config,
                OnChainConsensusConfig::V3(ConsensusConfigV1Ext { main, .. }) => main,
                config => unimplemented!(
                    "This test requires a V1/V2/V3 consensus config, but got: {:?}",
                    config
                ),
            };
            let leader_reputation_type = match &consensus_config.proposer_election_type {
                ProposerElectionType::LeaderReputation(leader_reputation_type) => {
                    leader_reputation_type
                },
                proposer_election_type => panic!(
                    "This test requires a leader reputation proposer election, but got: {:?}",
                    proposer_election_type
                ),
            };
            let proposer_and_voter_config = match &leader_reputation_type {
                LeaderReputationType::ProposerAndVoterV2(proposer_and_voter_config) => {
                    proposer_and_voter_config
                },
                leader_reputation_type => panic!(
                    "This test requires a proposer and voter V2 leader reputation, but got: {:?}",
                    leader_reputation_type
                ),
            };
            genesis_config.consensus_config = OnChainConsensusConfig::V1(ConsensusConfigV1 {
                proposer_election_type: ProposerElectionType::LeaderReputation(
                    LeaderReputationType::ProposerAndVoter(ProposerAndVoterConfig {
                        proposer_window_num_validators_multiplier: 1,
                        voter_window_num_validators_multiplier: 1,
                        use_history_from_previous_epoch_max_count: 1,
                        ..*proposer_and_voter_config
                    }),
                ),
                ..Default::default()
            });

            // Prevent epoch changes from occurring unnecessarily
            genesis_config.epoch_duration_secs = 10_000;
        }))
        .build()
        .await;

    // Test the ability of the second validator to sync
    let validator_index_to_test = 1;
    test_validator_sync(&mut swarm, validator_index_to_test, epoch_changes).await;

    // Verify the oldest ledger info and pruning metrics for the second validator
    if fast_sync && epoch_changes {
        let validator = swarm.validators_mut().nth(validator_index_to_test).unwrap();
        verify_fast_sync_version_and_metrics(validator, false).await;
    }

    // Execute multiple transactions through the first validator
    let validator_peer_ids = swarm.validators().map(|v| v.peer_id()).collect::<Vec<_>>();
    let validator_client = swarm
        .validator(*validator_peer_ids.first().unwrap())
        .unwrap()
        .rest_client();
    let (mut account_0, account_1) = create_test_accounts(&mut swarm).await;
    execute_transactions_and_wait(
        &mut swarm,
        &validator_client,
        &mut account_0,
        &account_1,
        false,
    )
    .await;

    // Stop the last validator (to prevent it from participating in consensus)
    let last_validator = swarm
        .validator_mut(*validator_peer_ids.last().unwrap())
        .unwrap();
    last_validator.stop();

    // Verify that consensus is progressing (the second validator should participate after syncing)
    execute_transactions(
        &mut swarm,
        &validator_client,
        &mut account_0,
        &account_1,
        false,
    )
    .await;
}

/// A helper method that tests that all validators can sync after a failure and
/// continue to stay up-to-date.
pub async fn test_all_validator_failures(mut swarm: LocalSwarm) {
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
        stop_validator_and_delete_storage(&mut swarm, validator).await;
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
        stop_validator_and_delete_storage(&mut swarm, validator).await;
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

/// Stops the specified fullnode and deletes storage
async fn stop_fullnode_and_delete_storage(swarm: &mut LocalSwarm, fullnode: AccountAddress) {
    let fullnode = swarm.full_node_mut(fullnode).unwrap();

    // The fullnode is stopped during the clear_storage() call
    fullnode.clear_storage().await.unwrap();
}

/// Stops the specified validator and deletes storage
async fn stop_validator_and_delete_storage(swarm: &mut LocalSwarm, validator: AccountAddress) {
    let validator = swarm.validator_mut(validator).unwrap();

    // The validator is stopped during the clear_storage() call
    validator.clear_storage().await.unwrap();
}

/// Verifies that the oldest ledger info, pruning metrics and first
/// ledger info are all correctly aligned after a fast sync.
async fn verify_fast_sync_version_and_metrics(node: &mut LocalNode, sync_to_genesis: bool) {
    // Verify the oldest ledger info for the node
    verify_oldest_version_after_fast_sync(node.rest_client(), sync_to_genesis).await;

    // Verify the node's pruning metrics
    let inspection_client = node.inspection_client();
    verify_pruning_metrics_after_fast_sync(inspection_client, sync_to_genesis).await;

    // Verify that the ledger info exists at version 0
    verify_first_ledger_info(node);
}

/// Verifies that the ledger info at version 0 exists in the given node's DB
fn verify_first_ledger_info(node: &mut LocalNode) {
    // Get the DB path for the node
    let db_path = node.config().base.data_dir.as_path();
    let mut db_path_buf = db_path.to_path_buf();
    db_path_buf.push("db");

    // Stop the node to prevent any DB contention
    node.stop();

    // Verify that the ledger info exists at version 0
    let aptos_db = AptosDB::new_for_test(db_path_buf.as_path());
    aptos_db.get_epoch_ending_ledger_info(0).unwrap();

    // Restart the node
    node.start().unwrap();
}

/// Verifies the oldest ledger version on a node after fast syncing
async fn verify_oldest_version_after_fast_sync(
    node_rest_client: RestClient,
    sync_to_genesis: bool,
) {
    // Fetch the oldest ledger version from the node
    let ledger_information = node_rest_client.get_ledger_information().await.unwrap();
    let oldest_ledger_version = ledger_information.inner().oldest_ledger_version;

    // Verify the oldest ledger version after fast syncing
    if sync_to_genesis {
        // The node should have fast synced to genesis
        assert_eq!(oldest_ledger_version, 0);
    } else {
        // The node should have fast synced to the latest epoch
        assert!(oldest_ledger_version > 0);
    }
}

/// Verifies the pruning metrics on a node after fast syncing
async fn verify_pruning_metrics_after_fast_sync(
    node_inspection_client: InspectionClient,
    sync_to_genesis: bool,
) {
    // Fetch the pruning metrics from the node
    let state_merkle_pruner_version = node_inspection_client
        .get_node_metric_i64(
            "aptos_pruner_versions{pruner_name=state_merkle_pruner,tag=min_readable}",
        )
        .await
        .unwrap()
        .unwrap();
    let epoch_snapshot_pruner_version = node_inspection_client
        .get_node_metric_i64(
            "aptos_pruner_versions{pruner_name=epoch_snapshot_pruner,tag=min_readable}",
        )
        .await
        .unwrap()
        .unwrap();
    let ledger_pruner_version = node_inspection_client
        .get_node_metric_i64("aptos_pruner_versions{pruner_name=ledger_pruner,tag=min_readable}")
        .await
        .unwrap()
        .unwrap();

    // Verify that the pruning metrics are valid
    if sync_to_genesis {
        // The node should have fast synced to genesis
        assert_eq!(state_merkle_pruner_version, 0);
        assert_eq!(epoch_snapshot_pruner_version, 0);
        assert_eq!(ledger_pruner_version, 0);
    } else {
        // The node should have fast synced to the latest epoch
        assert!(state_merkle_pruner_version > 0);
        assert!(epoch_snapshot_pruner_version > 0);
        assert!(ledger_pruner_version > 0);
    }
}
