// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    smoke_test_environment::SwarmBuilder,
    utils::{
        create_test_accounts, execute_transactions, execute_transactions_and_wait,
        wait_for_all_nodes, MAX_HEALTHY_WAIT_SECS,
    },
};
use velor_config::config::{BootstrappingMode, NodeConfig, OverrideNodeConfig};
use velor_db::VelorDB;
use velor_forge::{LocalNode, LocalSwarm, Node, NodeExt, Swarm};
use velor_inspection_service::inspection_client::InspectionClient;
use velor_rest_client::Client as RestClient;
use velor_sdk::types::PeerId;
use velor_storage_interface::DbReader;
use move_core_types::account_address::AccountAddress;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

/// Creates a new full node using the given config and swarm
pub async fn create_fullnode(full_node_config: NodeConfig, swarm: &mut LocalSwarm) -> PeerId {
    let validator_peer_id = swarm.validators().next().unwrap().peer_id();
    let vfn_peer_id = swarm
        .add_validator_fullnode(
            &swarm.versions().max().unwrap(),
            OverrideNodeConfig::new_with_default_base(full_node_config),
            validator_peer_id,
        )
        .unwrap();
    for fullnode in swarm.full_nodes() {
        fullnode
            .wait_until_healthy(Instant::now() + Duration::from_secs(MAX_HEALTHY_WAIT_SECS))
            .await
            .unwrap();
    }
    vfn_peer_id
}

/// Enables consensus observer for the given node config if `use_consensus_observer`
/// is true. This currently assumes that validators will only use the publisher,
/// and VFNs will only use the observer.
pub fn enable_consensus_observer(use_consensus_observer: bool, node_config: &mut NodeConfig) {
    if use_consensus_observer {
        match node_config.base.role {
            velor_config::config::RoleType::Validator => {
                node_config.consensus_observer.publisher_enabled = true;
            },
            velor_config::config::RoleType::FullNode => {
                node_config.consensus_observer.observer_enabled = true;
                node_config.consensus_observer.publisher_enabled = true;
            },
        }
    }
}

/// Stops the specified fullnode and only deletes storage if `clear_storage` is true
pub async fn stop_fullnode_and_delete_storage(
    swarm: &mut LocalSwarm,
    fullnode: AccountAddress,
    clear_storage: bool,
) {
    let fullnode = swarm.full_node(fullnode).unwrap();
    if clear_storage {
        // The fullnode is implicitly stopped during the clear_storage() call
        fullnode.clear_storage().await.unwrap();
    } else {
        fullnode.stop().await.unwrap();
    }
}

/// Stops the specified validator and deletes storage if `clear_storage` is true
pub async fn stop_validator_and_delete_storage(
    swarm: &mut LocalSwarm,
    validator: AccountAddress,
    clear_storage: bool,
) {
    let validator = swarm.validator_mut(validator).unwrap();
    if clear_storage {
        // The validator is implicitly stopped during the clear_storage() call
        validator.clear_storage().await.unwrap();
    } else {
        validator.stop();
    }
}

/// A test method that verifies that a fullnode can fast sync from
/// a validator after a data wipe.
/// - If `epoch_changes` are enabled then epoch changes can occur
///   during test execution and fullnode syncing.
/// - If `enable_consensus_observer` is enabled then the fullnode
///   will use the consensus observer.
pub async fn test_fullnode_fast_sync(epoch_changes: bool, use_consensus_observer: bool) {
    // Create a swarm with 2 validators
    let mut swarm = SwarmBuilder::new_local(2)
        .with_velor()
        .with_init_config(Arc::new(move |_, config, _| {
            config.state_sync.state_sync_driver.bootstrapping_mode =
                BootstrappingMode::DownloadLatestStates;
            enable_consensus_observer(use_consensus_observer, config);
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

    // Create a fullnode config with appropriate settings
    let mut vfn_config = NodeConfig::get_default_vfn_config();
    vfn_config.state_sync.state_sync_driver.bootstrapping_mode =
        BootstrappingMode::DownloadLatestStates;
    enable_consensus_observer(use_consensus_observer, &mut vfn_config);

    // Test the ability of a fullnode to sync
    let vfn_peer_id = create_fullnode(vfn_config, &mut swarm).await;
    test_fullnode_sync(vfn_peer_id, &mut swarm, epoch_changes, true).await;

    // Verify the oldest ledger info and pruning metrics for the fullnode
    if epoch_changes {
        let fullnode = swarm.fullnode_mut(vfn_peer_id).unwrap();
        verify_fast_sync_version_and_metrics(fullnode, false).await;
    }
}

/// A helper method that tests that a full node can sync from a validator after
/// a failure and continue to stay up-to-date.
pub async fn test_fullnode_sync(
    vfn_peer_id: PeerId,
    swarm: &mut LocalSwarm,
    epoch_changes: bool,
    clear_storage: bool,
) {
    // Stop the fullnode and potentially clear storage
    stop_fullnode_and_delete_storage(swarm, vfn_peer_id, clear_storage).await;

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

/// Verifies that the oldest ledger info, pruning metrics and first
/// ledger info are all correctly aligned after a fast sync.
pub async fn verify_fast_sync_version_and_metrics(node: &mut LocalNode, sync_to_genesis: bool) {
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
    let velor_db = VelorDB::new_for_test_with_sharding(db_path_buf.as_path(), 1 << 13);
    velor_db.get_epoch_ending_ledger_info(0).unwrap();

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
            "velor_pruner_versions{pruner_name=state_merkle_pruner,tag=min_readable}",
        )
        .await
        .unwrap()
        .unwrap();
    let epoch_snapshot_pruner_version = node_inspection_client
        .get_node_metric_i64(
            "velor_pruner_versions{pruner_name=epoch_snapshot_pruner,tag=min_readable}",
        )
        .await
        .unwrap()
        .unwrap();
    let ledger_pruner_version = node_inspection_client
        .get_node_metric_i64("velor_pruner_versions{pruner_name=ledger_pruner,tag=min_readable}")
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
