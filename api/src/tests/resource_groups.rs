// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::new_test_context;
use aptos_api_test_context::{current_function_name, TestContext};
use aptos_sdk::types::LocalAccount;
use aptos_types::{account_address::AccountAddress, transaction::TransactionPayload};
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
async fn test_read_resource_group() {
    let mut context = new_test_context(current_function_name!());

    // Prepare accounts
    let mut root = context.root_account();
    let mut admin0 = context.create_account(&mut root).await;
    let mut admin1 = context.create_account(&mut root).await;
    let mut user = context.create_account(&mut root).await;

    // Publish packages
    let named_addresses = vec![
        ("resource_groups_primary".to_string(), admin0.address()),
        ("resource_groups_secondary".to_string(), admin1.address()),
    ];

    let named_addresses_clone = named_addresses.clone();
    let txn = futures::executor::block_on(async move {
        let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
            .join("../aptos-move/move-examples/resource_groups/primary");
        context.build_package(path, named_addresses_clone)
    });
    context.publish_package(&mut admin0, txn).await;

    let named_addresses_clone = named_addresses.clone();
    let txn = futures::executor::block_on(async move {
        let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
            .join("../aptos-move/move-examples/resource_groups/secondary");
        context.build_package(path, named_addresses_clone)
    });
    context.publish_package(&mut admin1, txn).await;

    // Read default data
    let primary = format!("0x{}::{}::{}", admin0.address(), "primary", "Primary");
    let secondary = format!("0x{}::{}::{}", admin1.address(), "secondary", "Secondary");

    let response = context.read_resource(&admin0.address(), &primary).await;
    assert_eq!(response["data"]["value"], "3");

    let response = context.maybe_read_resource(&admin0.address(), &primary).await;
    assert_eq!(response.unwrap()["data"]["value"], "3");

    // Verify account is empty
    let response = context.maybe_read_resource(&user.address(), &primary).await;
    assert!(response.is_none());
    let response = context.maybe_read_resource(&user.address(), &secondary).await;
    assert!(response.is_none());

   // Init secondary
   execute_entry_function(
       &mut context,
       &mut user,
       &format!("0x{}::secondary::init", admin1.address()),
       json!([]),
       json!([55]),
   )
   .await;
   let response = context.read_resource(&user.address(), &secondary).await;
   assert_eq!(response["data"]["value"], 55);

   let response = context.maybe_read_resource(&user.address(), &secondary).await;
   assert_eq!(response.unwrap()["data"]["value"], 55);

   let response = context.maybe_read_resource(&user.address(), &primary).await;
   assert!(response.is_none());

   // Init primary
   execute_entry_function(
       &mut context,
       &mut user,
       &format!("0x{}::primary::init", admin0.address()),
       json!([]),
       json!(["35"]),
   )
   .await;
   let response = context.read_resource(&user.address(), &primary).await;
   assert_eq!(response["data"]["value"], "35");

   let response = context.maybe_read_resource(&user.address(), &primary).await;
   assert_eq!(response.unwrap()["data"]["value"], "35");

   let response = context.read_resource(&user.address(), &secondary).await;
   assert_eq!(response["data"]["value"], 55);

   let response = context.maybe_read_resource(&user.address(), &secondary).await;
   assert_eq!(response.unwrap()["data"]["value"], 55);
}


