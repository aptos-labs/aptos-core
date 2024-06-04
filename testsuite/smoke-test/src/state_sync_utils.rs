// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::utils::{
    create_test_accounts, execute_transactions, execute_transactions_and_wait, wait_for_all_nodes,
    MAX_HEALTHY_WAIT_SECS,
};
use aptos_config::config::{NodeConfig, OverrideNodeConfig};
use aptos_forge::{LocalSwarm, NodeExt, Swarm};
use aptos_sdk::types::PeerId;
use move_core_types::account_address::AccountAddress;
use std::time::{Duration, Instant};

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
    for fullnode in swarm.full_nodes_mut() {
        fullnode
            .wait_until_healthy(Instant::now() + Duration::from_secs(MAX_HEALTHY_WAIT_SECS))
            .await
            .unwrap();
    }
    vfn_peer_id
}

/// Stops the specified fullnode and deletes storage
pub async fn stop_fullnode_and_delete_storage(swarm: &mut LocalSwarm, fullnode: AccountAddress) {
    let fullnode = swarm.full_node_mut(fullnode).unwrap();

    // The fullnode is stopped during the clear_storage() call
    fullnode.clear_storage().await.unwrap();
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
