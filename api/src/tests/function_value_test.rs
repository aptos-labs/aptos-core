// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::new_test_context;
use aptos_api_test_context::{current_function_name, TestContext};
use serde_json::json;
use std::path::PathBuf;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_function_values() {
    let mut context = new_test_context(current_function_name!());
    let mut account = context.create_account().await;
    let account_addr = account.address();

    // Publish packages
    let named_addresses = vec![("account".to_string(), account_addr)];
    let txn = futures::executor::block_on(async move {
        let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
            .join("../aptos-move/move-examples/function_values/calculator");
        TestContext::build_package_latest_language(path, named_addresses)
    });
    context.publish_package(&mut account, txn).await;

    let state_resource = format!("{}::{}::{}", account_addr, "calculator", "State");

    let state = context
        .gen_resource(&account_addr, &state_resource)
        .await
        .unwrap();
    assert_eq!(state["data"]["__variant__"], "Empty");

    context
        .api_execute_entry_function(
            &mut account,
            &format!("0x{}::calculator::number", account_addr.to_hex()),
            json!([]),
            json!(["22"]),
        )
        .await;
    let state = context
        .gen_resource(&account_addr, &state_resource)
        .await
        .unwrap();
    assert_eq!(state["data"]["__variant__"], "Value");
    assert_eq!(state["data"]["_0"], "22");

    context
        .api_execute_entry_function(
            &mut account,
            &format!("0x{}::calculator::add", account_addr.to_hex()),
            json!([]),
            json!([]),
        )
        .await;
    let state = context
        .gen_resource(&account_addr, &state_resource)
        .await
        .unwrap();
    assert_eq!(state["data"]["__variant__"], "WaitForNumber");
    let closure = &state["data"]["_0"];
    // Closure has the form:
    // {
    //     __fun_name__: string<qualified_name>,
    //     __mask__    : string<number>,
    //     __captured__: array<value>
    // }
    assert_eq!(
        closure["__fun_name__"],
        format!("{}::calculator::storable_add", account_addr)
    );
    assert_eq!(closure["__mask__"], "1");
    assert_eq!(closure["__captured__"][0], "22");

    context
        .api_execute_entry_function(
            &mut account,
            &format!("0x{}::calculator::number", account_addr.to_hex()),
            json!([]),
            json!(["11"]),
        )
        .await;
    let state = context
        .gen_resource(&account_addr, &state_resource)
        .await
        .unwrap();
    assert_eq!(state["data"]["__variant__"], "Value");
    assert_eq!(state["data"]["_0"], "33");
}
