// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::new_test_context;
use aptos_api_test_context::{current_function_name, TestContext};
use serde_json::json;
use std::path::PathBuf;

// This test verifies that both READ APIs can seamlessly translate from resource group to resource
// 1. Create accounts
// 2. Publish a resource group package
// 3. Verify default data exists
// 4. Read the resources from that resource group and verify they don't exist
// 5. Init data for that resource group / member
// 6. Read and ensure data is present
// 7. Publish another resource group member
// 8. Read the resources from the new resource group and verify they don't exist
// 9. Init data for that resource group / member
// 10. Read and ensure data is present
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_gen_resource_group() {
    let mut context = new_test_context(current_function_name!());

    // Prepare accounts
    let mut admin0 = context.create_account().await;
    let mut admin1 = context.create_account().await;
    let mut user = context.create_account().await;

    // Publish packages
    let named_addresses = vec![
        ("resource_groups_primary".to_string(), admin0.address()),
        ("resource_groups_secondary".to_string(), admin1.address()),
    ];

    let named_addresses_clone = named_addresses.clone();
    let txn = futures::executor::block_on(async move {
        let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
            .join("../aptos-move/move-examples/resource_groups/primary");
        TestContext::build_package(path, named_addresses_clone)
    });
    context.publish_package(&mut admin0, txn).await;

    let named_addresses_clone = named_addresses.clone();
    let txn = futures::executor::block_on(async move {
        let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
            .join("../aptos-move/move-examples/resource_groups/secondary");
        TestContext::build_package(path, named_addresses_clone)
    });
    context.publish_package(&mut admin1, txn).await;

    // Read default data
    let primary = format!("{}::{}::{}", admin0.address(), "primary", "Primary");
    let secondary = format!("{}::{}::{}", admin1.address(), "secondary", "Secondary");

    let response = context.gen_resource(&admin0.address(), &primary).await;
    assert_eq!(response.unwrap()["data"]["value"], "3");

    // Verify account is empty
    let response = context.gen_resource(&user.address(), &primary).await;
    assert!(response.is_none());
    let response = context.gen_resource(&user.address(), &secondary).await;
    assert!(response.is_none());

    // Init secondary
    context
        .api_execute_entry_function(
            &mut user,
            &format!("{}::secondary::init", admin1.address()),
            json!([]),
            json!([55]),
        )
        .await;
    let response = context.gen_resource(&user.address(), &secondary).await;
    assert_eq!(response.unwrap()["data"]["value"], 55);

    let response = context.gen_resource(&user.address(), &primary).await;
    assert!(response.is_none());

    // Init primary
    context
        .api_execute_entry_function(
            &mut user,
            &format!("{}::primary::init", admin0.address()),
            json!([]),
            json!(["35"]),
        )
        .await;
    let response = context.gen_resource(&user.address(), &primary).await;
    assert_eq!(response.unwrap()["data"]["value"], "35");

    let response = context.gen_resource(&user.address(), &secondary).await;
    assert_eq!(response.unwrap()["data"]["value"], 55);

    let resp = context
        .get(format!("/accounts/{}/transactions", user.address()).as_str())
        .await;
    let secondary_tx = &resp.as_array().unwrap()[0];
    assert_writeset_contains_secondary_changes(&resp.as_array().unwrap()[1]);
    let resp = context
        .get(
            format!(
                "/transactions/by_hash/{}",
                secondary_tx["hash"].as_str().unwrap()
            )
            .as_str(),
        )
        .await;
    assert_writeset_contains_secondary_changes(&resp);
    let resp = context
        .get(
            format!(
                "/transactions/by_version/{}",
                secondary_tx["version"].as_str().unwrap()
            )
            .as_str(),
        )
        .await;
    assert_writeset_contains_secondary_changes(&resp);
}

fn assert_writeset_contains_secondary_changes(writeset: &serde_json::Value) {
    let changes = &writeset["changes"].as_array().unwrap();
    assert!(changes.iter().any(|c| c.get("data").is_some_and(|d| d
        .get("type")
        .is_some_and(|t| t.as_str().unwrap().contains("secondary")))));
}
