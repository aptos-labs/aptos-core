// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::{new_test_context, new_test_context_with_db_sharding_and_internal_indexer};
use aptos_api_test_context::current_function_name;
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;

static MODULE_EVENT_MIGRATION: u64 = 57;

const SLEEP_DURATION: Duration = Duration::from_millis(250);

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_feature_enable_disable() {
    let mut context = new_test_context(current_function_name!());
    context.enable_feature(MODULE_EVENT_MIGRATION).await;
    assert!(context.is_feature_enabled(MODULE_EVENT_MIGRATION).await);
    context.disable_feature(MODULE_EVENT_MIGRATION).await;
    assert!(!context.is_feature_enabled(MODULE_EVENT_MIGRATION).await);
    context.enable_feature(MODULE_EVENT_MIGRATION).await;
    assert!(context.is_feature_enabled(MODULE_EVENT_MIGRATION).await);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_event_v2_translation_simulation() {
    // let context = &mut new_test_context(current_function_name!());
    let context =
        &mut new_test_context_with_db_sharding_and_internal_indexer(current_function_name!());

    let account1 = &mut context.api_create_account().await;
    context.wait_for_internal_indexer_caught_up().await;
    // let account1 = &mut context.create_account().await;
    let account2 = &mut context.api_create_account().await;
    context.wait_for_internal_indexer_caught_up().await;
    // let account2 = &mut context.create_account().await;
    context.enable_feature(MODULE_EVENT_MIGRATION).await;

    context.wait_for_internal_indexer_caught_up().await; 
    let payload = json!({
        "type": "entry_function_payload",
        "function": "0x1::coin::transfer",
        "type_arguments": ["0x1::aptos_coin::AptosCoin"],
        "arguments": [
            account1.address().to_hex_literal(), "100"
        ]
    });
    context.wait_for_internal_indexer_caught_up().await;
    let resp = context.simulate_transaction(account2, payload, 200).await;
    // sleep(SLEEP_DURATION).await;

    // The V2 event should not appear.
    assert!(!resp[0]["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x["type"] == "0x1::coin::CoinDeposit"));

    // The translated V1 event should appear.
    assert!(resp[0]["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x["type"] == "0x1::coin::DepositEvent"
            && x["guid"]["creation_number"] == "2"
            && x["guid"]["account_address"] == account1.address().to_hex_literal()));
}
