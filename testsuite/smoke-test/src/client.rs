// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    smoke_test_environment::new_local_swarm,
    test_utils::{
        assert_balance, check_create_mint_transfer, create_and_fund_account, transfer_coins,
    },
};
use diem_json_rpc_types::Id;
use diem_sdk::client::stream::{
    request::StreamMethod, response::StreamJsonRpcResponseView, StreamingClient,
    StreamingClientConfig,
};
use forge::{Node, NodeExt, Swarm};
use futures::StreamExt;
use std::time::{Duration, Instant};
use tokio::{
    runtime::Runtime,
    time::{sleep, timeout},
};

#[test]
fn test_create_mint_transfer_block_metadata() {
    let mut swarm = new_local_swarm(1);

    // This script does 4 transactions
    check_create_mint_transfer(&mut swarm);

    // Test if we commit not only user transactions but also block metadata transactions,
    // assert committed version > # of user transactions
    let client = swarm.validators().next().unwrap().rest_client();
    let version = Runtime::new()
        .unwrap()
        .block_on(client.get_ledger_information())
        .unwrap()
        .into_inner()
        .version;
    assert!(
        version > 4,
        "BlockMetadata txn not produced, current version: {}",
        version
    );
}

#[test]
fn test_get_events_via_websocket_stream() {
    let mut swarm = new_local_swarm(1);

    // Update all nodes to enable websockets
    for validator in swarm.validators_mut() {
        let mut node_config = validator.config().clone();
        node_config.json_rpc.stream_rpc.enabled = true;
        node_config.save(validator.config_path()).unwrap();
        validator.restart().unwrap();
        validator
            .wait_until_healthy(Instant::now() + Duration::from_secs(10))
            .unwrap();
    }

    let client = swarm.validators().next().unwrap().json_rpc_client();

    let currencies = client
        .get_currencies()
        .expect("Could not get currency info")
        .into_inner();

    let rt = Runtime::new().unwrap();
    let _guard = rt.enter();

    let ms_500 = Duration::from_millis(500);

    let config = Some(StreamingClientConfig {
        channel_size: 1,
        ok_timeout_millis: 1_000,
    });

    let mut streaming_url = swarm.validators().next().unwrap().json_rpc_endpoint();
    streaming_url
        .set_scheme("ws")
        .expect("Could not set scheme");
    // Path from /json-rpc/src/stream_rpc/transport/websocket.rs#L43
    streaming_url.set_path("/v1/stream/ws");
    println!("ws_url: {}", &streaming_url);

    let mut s_client = rt
        .block_on(timeout(
            ms_500,
            StreamingClient::new(streaming_url, config.unwrap_or_default(), None),
        ))
        .unwrap_or_else(|e| panic!("Timeout creating StreamingClient: {}", e))
        .unwrap_or_else(|e| panic!("Error connecting to WS endpoint: {}", e));

    for (i, currency) in currencies.iter().enumerate() {
        println!("Subscribing to events for {}", &currency.code);

        let mut subscription_stream = rt
            .block_on(timeout(
                ms_500,
                s_client.subscribe_events(currency.mint_events_key, 0),
            ))
            .unwrap_or_else(|e| panic!("Timeout subscribing to {}: {}", &currency.code, e))
            .unwrap_or_else(|e| {
                panic!("Error subscribing to currency '{}': {}", &currency.code, e)
            });

        assert_eq!(subscription_stream.id(), &Id::Number(i as u64));

        let count_before = rt
            .block_on(timeout(ms_500, s_client.subscription_count()))
            .unwrap_or_else(|e| panic!("Timeout count for {}: {}", &currency.code, e));
        assert_eq!(count_before, 1, "Only one subscription should be running");

        // If we're here, then the subscription has already sent the 'OK' message
        let count_after;
        if &currency.code == "XUS" {
            println!("Getting msg 1 for {}", &currency.code);

            let response = rt
                .block_on(timeout(ms_500, subscription_stream.next()))
                .unwrap_or_else(|e| panic!("Timeout getting message 1: {}", e))
                .unwrap_or_else(|| panic!("Currency '{}' response 1 is None", &currency.code))
                .unwrap_or_else(|e| {
                    panic!("Currency '{}' response 1 is Err: {}", &currency.code, e)
                });

            println!("Got msg 1 for {}: {:?}", &currency.code, &response);

            let response_view = response
                .parse_result(&StreamMethod::SubscribeToEvents)
                .unwrap_or_else(|e| {
                    panic!(
                        "Currency '{}' response 1 view is err: {}",
                        &currency.code, e
                    )
                })
                .unwrap_or_else(|| panic!("Currency '{}' response 1 view is None", &currency.code));

            match response_view {
                StreamJsonRpcResponseView::Event(_) => {}
                _ => panic!("Expected 'Event', but got: {:?}", response_view),
            }
        }

        drop(subscription_stream);

        rt.block_on(sleep(ms_500));

        count_after = rt
            .block_on(timeout(ms_500, s_client.subscription_count()))
            .unwrap_or_else(|e| panic!("Timeout count for {}: {}", &currency.code, e));

        assert_eq!(
            count_after, 0,
            "No subscriptions should be running at the end"
        );
    }
}

#[test]
fn test_basic_fault_tolerance() {
    // A configuration with 4 validators should tolerate single node failure.
    let mut swarm = new_local_swarm(4);
    swarm.validators_mut().nth(3).unwrap().stop();
    check_create_mint_transfer(&mut swarm);
}

#[test]
fn test_basic_restartability() {
    let mut swarm = new_local_swarm(4);
    let client = swarm.validators().next().unwrap().rest_client();
    let transaction_factory = swarm.chain_info().transaction_factory();

    let mut account_0 = create_and_fund_account(&mut swarm, 100);
    let account_1 = create_and_fund_account(&mut swarm, 10);

    let runtime = Runtime::new().unwrap();
    runtime.block_on(async {
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
    });

    let validator = swarm.validators_mut().next().unwrap();
    validator.restart().unwrap();
    validator
        .wait_until_healthy(Instant::now() + Duration::from_secs(10))
        .unwrap();

    runtime.block_on(async {
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
    });
}

#[test]
fn test_concurrent_transfers_single_node() {
    let mut swarm = new_local_swarm(1);
    let client = swarm.validators().next().unwrap().rest_client();
    let transaction_factory = swarm.chain_info().transaction_factory();

    let mut account_0 = create_and_fund_account(&mut swarm, 100);
    let account_1 = create_and_fund_account(&mut swarm, 10);

    let runtime = Runtime::new().unwrap();
    runtime.block_on(async {
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
    });
}
