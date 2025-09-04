// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    smoke_test_environment::{new_local_swarm_with_velor, SwarmBuilder},
    utils::{
        assert_balance, check_create_mint_transfer, create_and_fund_account, transfer_coins,
        MAX_HEALTHY_WAIT_SECS,
    },
};
use velor_cached_packages::velor_stdlib;
use velor_forge::{NodeExt, Swarm};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

#[tokio::test]
async fn test_create_mint_transfer_block_metadata() {
    let mut swarm = new_local_swarm_with_velor(1).await;
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
    let mut swarm = new_local_swarm_with_velor(4).await;
    swarm.validators_mut().nth(3).unwrap().stop();
    check_create_mint_transfer(&mut swarm).await;
}

#[tokio::test]
async fn test_basic_restartability() {
    let mut swarm = new_local_swarm_with_velor(4).await;
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
        .wait_until_healthy(Instant::now() + Duration::from_secs(MAX_HEALTHY_WAIT_SECS))
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
    let mut swarm = new_local_swarm_with_velor(1).await;
    let client = swarm.validators().next().unwrap().rest_client();
    let transaction_factory = swarm.chain_info().transaction_factory();

    let mut account_0 = create_and_fund_account(&mut swarm, 100).await;
    let account_1 = create_and_fund_account(&mut swarm, 10).await;

    assert_balance(&client, &account_0, 100).await;
    assert_balance(&client, &account_1, 10).await;

    for _ in 0..20 {
        let txn = account_0.sign_with_transaction_builder(
            transaction_factory.payload(velor_stdlib::velor_coin_transfer(account_1.address(), 1)),
        );
        client.submit_and_wait(&txn).await.unwrap();
    }
    transfer_coins(&client, &transaction_factory, &mut account_0, &account_1, 1).await;
    // assert_balance(&client, &account_0, 79).await;
    assert_balance(&client, &account_1, 31).await;
}

#[tokio::test]
async fn test_latest_events_and_transactions() {
    let mut swarm = SwarmBuilder::new_local(1)
        .with_velor()
        .with_init_config(Arc::new(|_, conf, _| {
            conf.indexer_db_config.enable_event = true;
        }))
        .build()
        .await;
    let client = swarm.validators().next().unwrap().rest_client();
    let start_events = client
        .get_new_block_events_bcs(None, Some(2))
        .await
        .unwrap()
        .into_inner();
    let start_transations = client
        .get_transactions(None, Some(2))
        .await
        .unwrap()
        .into_inner();

    create_and_fund_account(&mut swarm, 100).await;
    let cur_events = client
        .get_new_block_events_bcs(None, Some(2))
        .await
        .unwrap()
        .into_inner();
    let (cur_transations, cur_ledger) = client
        .get_transactions(None, Some(2))
        .await
        .unwrap()
        .into_parts();

    assert!(start_events[0].event.round() < cur_events[0].event.round());
    assert!(cur_events[0].event.round() < cur_events[1].event.round());
    assert_eq!(cur_events.len(), 2);

    assert!(start_transations[0].version() < cur_transations[0].version());
    assert!(cur_transations[0].version() < cur_transations[1].version());
    assert_eq!(cur_transations.len(), 2);
    assert_eq!(cur_transations[1].version().unwrap(), cur_ledger.version);
}
