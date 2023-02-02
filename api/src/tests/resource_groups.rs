// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::new_test_context;
use aptos_api_test_context::{current_function_name, TestContext};
use aptos_cached_packages::aptos_stdlib;
use aptos_framework::BuiltPackage;
use aptos_logger::info;
use aptos_sdk::types::LocalAccount;
use aptos_types::{account_address::AccountAddress, transaction::TransactionPayload};
use serde_json::{json, Value};
use std::{path::PathBuf, thread};

// This test verifies that both READ APIs can seamlessly translate from resource group to resource
// 1. Create accounts
// 2. Publish a resource group package
// 3. Verify default data exists
// 4. Read the resources from that resource group anad verify they don't exist
// 5. Init data for that resource group / member
// 6. Read and ensure data is present
// 7. Publish another resource group member
// 8. Read the resources from the new resource group anad verify they don't exist
// 9. Init data for that resource group / member
// 10. Read and ensure data is present
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_read_resoure_group() {
    println!("BCHO 0");
    let mut context = new_test_context(current_function_name!());

    // Prepare accounts
    println!("BCHO 1");
    let mut root = context.root_account();
    let mut admin0 = create_account(&mut context, &mut root).await;
    let admin0_address = admin0.address().clone();
    let mut admin1 = create_account(&mut context, &mut root).await;
    let admin1_address = admin1.address().clone();
    let mut user = create_account(&mut context, &mut root).await;

    // Publish packages
    println!("BCHO 2");
    let named_addresses = vec![
        ("resource_groups_primary".to_string(), admin0_address),
        ("resource_groups_secondary".to_string(), admin1_address),
    ];

    let named_addresses_clone = named_addresses.clone();
    let txn = futures::executor::block_on(async move {
        println!("BCHO 3");
        let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
            .join("../aptos-move/move-examples/resource_groups/primary");
        build_package(path, named_addresses_clone)
    });
    publish_package(&mut context, &mut admin0, txn).await;

    let named_addresses_clone = named_addresses.clone();
    let txn = futures::executor::block_on(async move {
        println!("BCHO 4");
        let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
            .join("../aptos-move/move-examples/resource_groups/secondary");
        build_package(path, named_addresses_clone)
    });
    publish_package(&mut context, &mut admin1, txn).await;

    // Read default data
    println!("BCHO 5");
    let primary = format!("0x{}::{}::{}", admin0_address, "primary", "Primary");
    let secondary = format!("0x{}::{}::{}", admin1_address, "secondary", "Secondary");

    let response = read_resource(&context, &admin0_address, &primary).await;
    assert_eq!(response["data"]["value"], "3");

    let response = maybe_read_resource(&context, &admin0_address, &primary).await;
    assert_eq!(response.unwrap()["data"]["value"], "3");

    // Verify account is empty
    println!("BCHO 6");
    let response = maybe_read_resource(&context, &user.address(), &primary).await;
    assert!(response.is_none());
    let response = maybe_read_resource(&context, &user.address(), &secondary).await;
    assert!(response.is_none());

    // Init secondary
    println!("BCHO 7");
    execute_entry_function(
        &mut context,
        &mut user,
        &format!("0x{}::secondary::init", admin1_address),
        json!([]),
        json!([55]),
    )
    .await;
    let response = read_resource(&context, &user.address(), &secondary).await;
    assert_eq!(response["data"]["value"], 55);

    let response = maybe_read_resource(&context, &user.address(), &secondary).await;
    assert_eq!(response.unwrap()["data"]["value"], 55);

    let response = maybe_read_resource(&context, &user.address(), &primary).await;
    assert!(response.is_none());

    // Init primary
    println!("BCHO 8");
    execute_entry_function(
        &mut context,
        &mut user,
        &format!("0x{}::primary::init", admin0_address),
        json!([]),
        json!(["35"]),
    )
    .await;
    let response = read_resource(&context, &user.address(), &primary).await;
    assert_eq!(response["data"]["value"], "35");

    let response = maybe_read_resource(&context, &user.address(), &primary).await;
    assert_eq!(response.unwrap()["data"]["value"], "35");

    let response = read_resource(&context, &user.address(), &secondary).await;
    assert_eq!(response["data"]["value"], 55);

    let response = maybe_read_resource(&context, &user.address(), &secondary).await;
    assert_eq!(response.unwrap()["data"]["value"], 55);
}

// TODO: The TestContext code is a bit of a mess, the following likely should be added and that
// code likely needs a good cleanup to merge to a common approach.

async fn create_account(context: &mut TestContext, root: &mut LocalAccount) -> LocalAccount {
    let account = context.gen_account();
    let factory = context.transaction_factory();
    let txn = root.sign_with_transaction_builder(
        factory
            .account_transfer(account.address(), 10_000_000)
            .expiration_timestamp_secs(u64::MAX),
    );

    let bcs_txn = bcs::to_bytes(&txn).unwrap();
    context
        .expect_status_code(202)
        .post_bcs_txn("/transactions", bcs_txn)
        .await;
    context.commit_mempool_txns(1).await;
    account
}

async fn maybe_read_resource(
    context: &TestContext,
    account_address: &AccountAddress,
    resource: &str,
) -> Option<Value> {
    let response = read_resources(context, account_address).await;
    response
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["type"] == resource)
        .cloned()
}

async fn read_resources(context: &TestContext, account_address: &AccountAddress) -> Value {
    let request = format!("/accounts/{}/resources", account_address);
    context.get(&request).await
}

async fn read_resource(
    context: &TestContext,
    account_address: &AccountAddress,
    resource: &str,
) -> Value {
    let request = format!("/accounts/{}/resource/{}", account_address, resource);
    context.get(&request).await
}

fn build_package(
    path: PathBuf,
    named_addresses: Vec<(String, AccountAddress)>,
) -> TransactionPayload {
    println!("BCHO publish_package 0");
    let mut build_options = aptos_framework::BuildOptions::default();
    let _ = named_addresses
        .into_iter()
        .map(|(name, address)| build_options.named_addresses.insert(name, address))
        .collect::<Vec<_>>();

    println!("BCHO publish_package 1");
    let package = BuiltPackage::build(path.to_owned(), build_options).unwrap();
    println!("BCHO publish_package 2");
    let code = package.extract_code();
    println!("BCHO publish_package 3");
    let metadata = package.extract_metadata().unwrap();

    println!("BCHO publish_package 4");
    aptos_stdlib::code_publish_package_txn(bcs::to_bytes(&metadata).unwrap(), code)
}

async fn publish_package(
    context: &mut TestContext,
    publisher: &mut LocalAccount,
    payload: TransactionPayload,
) {
    println!("BCHO publish_package 5");
    let txn =
        publisher.sign_with_transaction_builder(context.transaction_factory().payload(payload));
    println!("BCHO publish_package 6");
    let bcs_txn = bcs::to_bytes(&txn).unwrap();
    println!("BCHO publish_package 7");
    context
        .expect_status_code(202)
        .post_bcs_txn("/transactions", bcs_txn)
        .await;
    println!("BCHO publish_package 8");
    context.commit_mempool_txns(1).await;
    println!("BCHO publish_package 9");
}

async fn execute_entry_function(
    context: &mut TestContext,
    account: &mut LocalAccount,
    function: &str,
    type_args: serde_json::Value,
    args: serde_json::Value,
) {
    context
        .api_execute_txn(
            account,
            json!({
                "type": "entry_function_payload",
                "function": function,
                "type_arguments": type_args,
                "arguments": args
            }),
        )
        .await;
}
