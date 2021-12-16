// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    smoke_test_environment::new_local_swarm,
    test_utils::{
        assert_balance, check_create_mint_transfer, create_and_fund_account, transfer_coins,
    },
};
use forge::{NodeExt, Swarm};
use std::time::{Duration, Instant};

#[tokio::test]
async fn test_create_mint_transfer_block_metadata() {
    let mut swarm = new_local_swarm(1).await;

    // This script does 4 transactions
    check_create_mint_transfer(&mut swarm).await;

    // Test if we commit not only user transactions but also block metadata transactions,
    // assert committed version > # of user transactions
    let client = swarm.validators().next().unwrap().rest_client();
    let version = client
        .get_ledger_information()
        .await
        .unwrap()
        .into_inner()
        .version;
    assert!(
        version > 4,
        "BlockMetadata txn not produced, current version: {}",
        version
    );
}

#[tokio::test]
async fn test_basic_fault_tolerance() {
    // A configuration with 4 validators should tolerate single node failure.
    let mut swarm = new_local_swarm(4).await;
    swarm.validators_mut().nth(3).unwrap().stop();
    check_create_mint_transfer(&mut swarm).await;
}

#[tokio::test]
async fn test_basic_restartability() {
    let mut swarm = new_local_swarm(4).await;
    let client = swarm.validators().next().unwrap().rest_client();
    let transaction_factory = swarm.chain_info().transaction_factory();

    let mut account_0 = create_and_fund_account(&mut swarm, 100).await;
    let account_1 = create_and_fund_account(&mut swarm, 10).await;

    transfer_coins(
        &client,
        &transaction_factory,
        &mut account_0,
        &account_1,
        10,
    )
    .await;
    assert_balance(&client, &account_0, 90).await;
    assert_balance(&client, &account_1, 20).await;

    let validator = swarm.validators_mut().next().unwrap();
    validator.restart().await.unwrap();
    validator
        .wait_until_healthy(Instant::now() + Duration::from_secs(10))
        .await
        .unwrap();

    assert_balance(&client, &account_0, 90).await;
    assert_balance(&client, &account_1, 20).await;

    transfer_coins(
        &client,
        &transaction_factory,
        &mut account_0,
        &account_1,
        10,
    )
    .await;
    assert_balance(&client, &account_0, 80).await;
    assert_balance(&client, &account_1, 30).await;
}

#[tokio::test]
async fn test_concurrent_transfers_single_node() {
    let mut swarm = new_local_swarm(1).await;
    let client = swarm.validators().next().unwrap().rest_client();
    let transaction_factory = swarm.chain_info().transaction_factory();

    let mut account_0 = create_and_fund_account(&mut swarm, 100).await;
    let account_1 = create_and_fund_account(&mut swarm, 10).await;

    assert_balance(&client, &account_0, 100).await;
    assert_balance(&client, &account_1, 10).await;

    for _ in 0..20 {
        let txn = account_0.sign_with_transaction_builder(transaction_factory.peer_to_peer(
            diem_sdk::transaction_builder::Currency::XUS,
            account_1.address(),
            1,
        ));
        client.submit_and_wait(&txn).await.unwrap();
    }
    transfer_coins(&client, &transaction_factory, &mut account_0, &account_1, 1).await;
    assert_balance(&client, &account_0, 79).await;
    assert_balance(&client, &account_1, 31).await;
}
