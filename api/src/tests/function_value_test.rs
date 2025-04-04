// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::new_test_context_with_orderless_flags;
use aptos_api_test_context::{current_function_name, TestContext};
use rstest::rstest;
use serde_json::json;
use std::path::PathBuf;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn test_function_values(use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let mut account = context.create_account().await;
    let account_addr = account.address();

    // Publish packages
    let named_addresses = vec![("account".to_string(), account_addr)];
    let txn = futures::executor::block_on(async move {
        let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
            .join("../aptos-move/move-examples/function_values/calculator");
        TestContext::build_package_with_latest_language(path, named_addresses)
    });
    context.publish_package(&mut account, txn).await;

    let state_resource = format!("{}::{}::{}", account_addr, "calculator", "State");
    let state = &context
        .gen_resource(&account_addr, &state_resource)
        .await
        .unwrap()["data"];
    assert_eq!(state, &json!({"__variant__": "Empty"}));

    context
        .api_execute_entry_function(
            &mut account,
            &format!("0x{}::calculator::number", account_addr.to_hex()),
            json!([]),
            json!(["22"]),
        )
        .await;
    let state = &context
        .gen_resource(&account_addr, &state_resource)
        .await
        .unwrap()["data"];
    assert_eq!(state, &json!({"__variant__": "Value", "_0": "22"}));

    context
        .api_execute_entry_function(
            &mut account,
            &format!("0x{}::calculator::add", account_addr.to_hex()),
            json!([]),
            json!([]),
        )
        .await;
    let state = &context
        .gen_resource(&account_addr, &state_resource)
        .await
        .unwrap()["data"];
    let expected_fun = format!("{}::calculator::storable_add", account_addr);
    assert_eq!(
        state,
        &json!({
            "__variant__": "WaitForNumber",
            "_0": {
                "__fun_name__": &expected_fun,
                "__mask__": "1",
                "__captured__": [ "22" ]
            }
        })
    );

    context
        .api_execute_entry_function(
            &mut account,
            &format!("0x{}::calculator::number", account_addr.to_hex()),
            json!([]),
            json!(["11"]),
        )
        .await;
    let state = &context
        .gen_resource(&account_addr, &state_resource)
        .await
        .unwrap()["data"];
    assert_eq!(state, &json!({"__variant__": "Value", "_0": "33"}));
}
