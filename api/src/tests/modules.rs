// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::new_test_context;
use aptos_api_test_context::{current_function_name, TestContext};
use std::path::PathBuf;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_abi() {
    let mut context = new_test_context(current_function_name!());
    let mut root_account = context.root_account().await;
    let mut account = context.gen_account();
    let txn = context.create_user_account_by(&mut root_account, &account);
    context.commit_block(&vec![txn]).await;

    // Publish packages
    let named_addresses = vec![("abi".to_string(), account.address())];
    let txn = futures::executor::block_on(async move {
        let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR")).join("src/tests/move/pack_abi");
        TestContext::build_package(path, named_addresses)
    });
    context.publish_package(&mut account, txn).await;

    // Get abi.
    let modules = context
        .get(format!("/accounts/{}/modules", account.address(),).as_str())
        .await;
    let expose_functions: Vec<&str> = modules.as_array().unwrap()[0]["abi"]["exposed_functions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["name"].as_str().unwrap())
        .collect();
    // All entry (including private entry) and public functions should be in the ABI.
    // Private (non-entry) functions should not be included.
    assert_eq!(expose_functions, [
        "private_entry_function",
        "public_entry_function",
        "public_function"
    ]);
}
