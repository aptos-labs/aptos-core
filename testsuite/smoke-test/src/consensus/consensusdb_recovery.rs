// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    smoke_test_environment::new_local_swarm_with_aptos,
    test_utils::{assert_balance, create_and_fund_account, transfer_coins},
};
use consensus::CONSENSUS_DB_NAME;
use forge::{HealthCheckError, NodeExt, Swarm};
use std::{
    fs,
    time::{Duration, Instant},
};

#[tokio::test]
async fn test_consensusdb_recovery() {
    let mut swarm = new_local_swarm_with_aptos(4).await;
    let validator_peer_ids = swarm.validators().map(|v| v.peer_id()).collect::<Vec<_>>();
    let client_1 = swarm
        .validator(validator_peer_ids[1])
        .unwrap()
        .rest_client();
    let transaction_factory = swarm.chain_info().transaction_factory();

    let mut account_0 = create_and_fund_account(&mut swarm, 100).await;
    let account_1 = create_and_fund_account(&mut swarm, 10).await;
    let txn = transfer_coins(
        &client_1,
        &transaction_factory,
        &mut account_0,
        &account_1,
        10,
    )
    .await;
    assert_balance(&client_1, &account_0, 90).await;
    assert_balance(&client_1, &account_1, 20).await;

    // Stop a node
    let node_to_restart = validator_peer_ids[0];
    let node_config = swarm.validator(node_to_restart).unwrap().config().clone();
    let node = swarm.validator_mut(node_to_restart).unwrap();
    node.stop();
    let consensus_db_path = node_config.storage.dir().join(CONSENSUS_DB_NAME);
    // Verify that consensus db exists and
    // we are not deleting a non-existent directory
    assert!(consensus_db_path.as_path().exists());
    // Delete the consensus db to simulate consensus db is nuked
    fs::remove_dir_all(consensus_db_path).unwrap();
    node.start().unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        // after the node recovers, it'll exit with 0
        if let Err(HealthCheckError::NotRunning(_)) = node.health_check().await {
            break;
        }
    }

    node.restart().await.unwrap();
    node.wait_until_healthy(Instant::now() + Duration::from_secs(10))
        .await
        .unwrap();

    let client_0 = swarm.validator(node_to_restart).unwrap().rest_client();
    // Wait for the txn to by synced to the restarted node
    client_0.wait_for_signed_transaction(&txn).await.unwrap();
    assert_balance(&client_0, &account_0, 90).await;
    assert_balance(&client_0, &account_1, 20).await;

    let txn = transfer_coins(
        &client_1,
        &transaction_factory,
        &mut account_0,
        &account_1,
        10,
    )
    .await;
    client_0.wait_for_signed_transaction(&txn).await.unwrap();

    assert_balance(&client_0, &account_0, 80).await;
    assert_balance(&client_0, &account_1, 30).await;
}
