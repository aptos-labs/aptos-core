// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::new_test_context;
use aptos_api_test_context::{current_function_name, TestContext};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde_json::json;
use std::path::PathBuf;

static ACCOUNT_ADDRESS: &str = "0xa550c18";
static CREATION_NUMBER: &str = "0";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_events() {
    let mut context = new_test_context(current_function_name!());

    let resp = context
        .get(format!("/accounts/{}/events/{}", ACCOUNT_ADDRESS, CREATION_NUMBER).as_str())
        .await;

    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_events_filter_by_start_sequence_number() {
    let mut context = new_test_context(current_function_name!());

    let resp = context
        .get(
            format!(
                "/accounts/{}/events/{}?start=1",
                ACCOUNT_ADDRESS, CREATION_NUMBER
            )
            .as_str(),
        )
        .await;
    context.check_golden_output(resp.clone());

    // assert the same resp after db sharding migration with internal indexer turned on
    let shard_context = new_test_context(current_function_name!());
    let new_resp = shard_context
        .get(
            format!(
                "/accounts/{}/events/{}?start=1",
                ACCOUNT_ADDRESS, CREATION_NUMBER
            )
            .as_str(),
        )
        .await;
    assert_eq!(resp, new_resp);
}

// turn it back until we have multiple events in genesis
#[ignore]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_events_filter_by_limit_page_size() {
    let context = new_test_context(current_function_name!());

    let resp = context
        .get(
            format!(
                "/accounts/{}/events/{}?start=1&limit=1",
                ACCOUNT_ADDRESS, CREATION_NUMBER
            )
            .as_str(),
        )
        .await;
    assert_eq!(resp.as_array().unwrap().len(), 1);

    let resp = context
        .get(
            format!(
                "/accounts/{}/events/{}?start=1&limit=2",
                ACCOUNT_ADDRESS, CREATION_NUMBER
            )
            .as_str(),
        )
        .await;
    assert_eq!(resp.as_array().unwrap().len(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_events_by_invalid_creation_number() {
    let mut context = new_test_context(current_function_name!());

    let resp = context
        .expect_status_code(400)
        .get(format!("/accounts/{}/events/invalid", ACCOUNT_ADDRESS).as_str())
        .await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_events_by_account_event_handle() {
    let mut context = new_test_context(current_function_name!());
    let resp = context
        .get("/accounts/0x1/events/0x1::reconfiguration::Configuration/events")
        .await;
    context.check_golden_output(resp.clone());

    let shard_context = new_test_context(current_function_name!());
    let new_resp = shard_context
        .get("/accounts/0x1/events/0x1::reconfiguration::Configuration/events")
        .await;
    assert_eq!(resp, new_resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_events_by_invalid_account_event_handle_struct_address() {
    let mut context = new_test_context(current_function_name!());
    let resp = context
        .expect_status_code(404)
        .get("/accounts/0x1/events/0x9::Reconfiguration::Configuration/events")
        .await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_events_by_invalid_account_event_handle_struct_module() {
    let mut context = new_test_context(current_function_name!());
    let resp = context
        .expect_status_code(404)
        .get("/accounts/0x1/events/0x1::NotFound::Configuration/events")
        .await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_events_by_invalid_account_event_handle_struct_name() {
    let mut context = new_test_context(current_function_name!());
    let resp = context
        .expect_status_code(404)
        .get("/accounts/0x1/events/0x1::reconfiguration::NotFound/events")
        .await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_events_by_invalid_account_event_handle_field_name() {
    let mut context = new_test_context(current_function_name!());
    let resp = context
        .expect_status_code(404)
        .get("/accounts/0x1/events/0x1::reconfiguration::Configuration/not_found")
        .await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_events_by_invalid_account_event_handle_field_type() {
    let mut context = new_test_context(current_function_name!());

    let resp = context
        .expect_status_code(400)
        .get("/accounts/0x1/events/0x1::reconfiguration::Configuration/epoch")
        .await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_module_events() {
    let mut context = new_test_context(current_function_name!());

    // Prepare accounts
    let mut user = context.create_account().await;

    let user_addr = user.address();
    // Publish packages
    let named_addresses = vec![("event".to_string(), user_addr)];
    let txn = futures::executor::block_on(async move {
        let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
            .join("../aptos-move/move-examples/event");
        TestContext::build_package(path, named_addresses)
    });
    context.publish_package(&mut user, txn).await;

    context
        .api_execute_entry_function(
            &mut user,
            &format!("0x{}::event::emit", user_addr.to_hex()),
            json!([]),
            json!(["7"]),
        )
        .await;

    let resp = context
        .get(format!("/accounts/{}/transactions", user.address()).as_str())
        .await;
    let txn = &resp.as_array().unwrap()[1];
    let resp = context
        .get(format!("/transactions/by_hash/{}", txn["hash"].as_str().unwrap()).as_str())
        .await;

    let events = resp["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| {
            e.get("guid")
                .unwrap()
                .get("account_address")
                .unwrap()
                .as_str()
                .unwrap()
                == "0x0"
        })
        .collect::<Vec<_>>();
    assert_eq!(events.len(), 8);
}

// until we have generics in the genesis
#[ignore]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_events_by_struct_type_has_generic_type_parameter() {
    let mut context = new_test_context(current_function_name!());

    // This test is for making sure we can look up right struct with generic
    // type specified in the URL path.
    // Instead of creating the example, we just look up an event handle that does not exist.
    let path = format!(
        "/accounts/0x1/events/{}/coin",
        utf8_percent_encode(
            "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>",
            NON_ALPHANUMERIC,
        )
    );
    let resp = context.expect_status_code(404).get(path.as_str()).await;
    context.check_golden_output(resp);
}
