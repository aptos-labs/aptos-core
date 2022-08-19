// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::new_test_context;
use aptos_api_test_context::{current_function_name, find_value};
use serde_json::json;

/* TODO: reactivate once cause of failure for `"8"` vs `8` in the JSON output is known.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_account_resources_returns_empty_array_for_account_has_no_resources() {
    let mut context = new_test_context(current_function_name!());
    let address = "0x1";

    let resp = context.get(&account_resources(address)).await;
    context.check_golden_output(resp);
}
 */

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_account_resources_by_address_0x0() {
    let mut context = new_test_context(current_function_name!());
    let address = "0x0";

    let resp = context
        .expect_status_code(404)
        .get(&account_resources(address))
        .await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_account_resources_by_invalid_address_missing_0x_prefix() {
    let mut context = new_test_context(current_function_name!());
    let invalid_addresses = vec!["1", "0xzz", "01"];
    for invalid_address in &invalid_addresses {
        let resp = context
            .expect_status_code(400)
            .get(&account_resources(invalid_address))
            .await;
        context.check_golden_output(resp);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_account_resources_by_valid_account_address() {
    let context = new_test_context(current_function_name!());
    let addresses = vec!["0x1", "0x00000000000000000000000000000001"];
    for address in &addresses {
        context.get(&account_resources(address)).await;
    }
}

// Unstable due to framework changes
#[ignore]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_account_resources_response() {
    let mut context = new_test_context(current_function_name!());
    let address = "0x1";

    let resp = context.get(&account_resources(address)).await;
    context.check_golden_output(resp);
}

// Unstable due to framework changes
#[ignore]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_account_modules() {
    let mut context = new_test_context(current_function_name!());
    let address = "0x1";

    let resp = context.get(&account_modules(address)).await;
    context.check_golden_output(resp);
}

// Unstable due to framework changes
#[ignore]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_module_with_entry_functions() {
    let mut context = new_test_context(current_function_name!());
    let address = "0x1";

    let resp = context.get(&account_modules(address)).await;
    context.check_golden_output(resp);
}

// Unstable due to framework changes
#[ignore]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_module_aptos_config() {
    let mut context = new_test_context(current_function_name!());
    let address = "0x1";

    let resp = context.get(&account_modules(address)).await;
    context.check_golden_output(resp);
}

// Unstable due to framework changes
#[ignore]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_account_modules_structs() {
    let mut context = new_test_context(current_function_name!());
    let address = "0x1";

    let resp = context.get(&account_modules(address)).await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_account_resources_by_ledger_version() {
    let mut context = new_test_context(current_function_name!());
    let account = context.gen_account();
    let txn = context.create_user_account(&account);
    context.commit_block(&vec![txn.clone()]).await;

    let ledger_version_1_resources = context
        .get(&account_resources(
            &context.root_account().address().to_hex_literal(),
        ))
        .await;
    let root_account = find_value(&ledger_version_1_resources, |f| {
        f["type"] == "0x1::account::Account"
    });
    assert_eq!(root_account["data"]["sequence_number"], "1");

    let ledger_version_0_resources = context
        .get(&account_resources_with_ledger_version(
            &context.root_account().address().to_hex_literal(),
            0,
        ))
        .await;
    let root_account = find_value(&ledger_version_0_resources, |f| {
        f["type"] == "0x1::account::Account"
    });
    assert_eq!(root_account["data"]["sequence_number"], "0");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_account_resources_by_ledger_version_is_too_large() {
    let mut context = new_test_context(current_function_name!());
    let resp = context
        .expect_status_code(404)
        .get(&account_resources_with_ledger_version(
            &context.root_account().address().to_hex_literal(),
            1000000000000000000,
        ))
        .await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_account_resources_by_invalid_ledger_version() {
    let mut context = new_test_context(current_function_name!());
    let resp = context
        .expect_status_code(400)
        .get(&account_resources_with_ledger_version(
            &context.root_account().address().to_hex_literal(),
            -1,
        ))
        .await;
    context.check_golden_output(resp);
}

// figure out a working module code, no idea where the existing one comes from
#[ignore] // TODO(issue 81): re-enable after cleaning up the compiled code in the test
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_account_modules_by_ledger_version() {
    let mut context = new_test_context(current_function_name!());
    let code = "a11ceb0b0300000006010002030205050703070a0c0816100c260900000001000100000102084d794d6f64756c650269640000000000000000000000000b1e55ed00010000000231010200";
    let mut root_account = context.root_account();
    let txn = root_account.sign_with_transaction_builder(
        context
            .transaction_factory()
            .module(hex::decode(code).unwrap()),
    );
    context.commit_block(&vec![txn.clone()]).await;
    let modules = context
        .get(&account_modules(
            &context.root_account().address().to_hex_literal(),
        ))
        .await;

    assert_ne!(modules, json!([]));

    let modules = context
        .get(&account_modules_with_ledger_version(
            &context.root_account().address().to_hex_literal(),
            0,
        ))
        .await;
    assert_eq!(modules, json!([]));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_core_account_data() {
    let mut context = new_test_context(current_function_name!());
    let resp = context.get("/accounts/0x1").await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_core_account_data_not_found() {
    let mut context = new_test_context(current_function_name!());
    let resp = context.expect_status_code(404).get("/accounts/0xf").await;
    context.check_golden_output(resp);
}

fn account_resources(address: &str) -> String {
    format!("/accounts/{}/resources", address)
}

fn account_resources_with_ledger_version(address: &str, ledger_version: i128) -> String {
    format!(
        "{}?ledger_version={}",
        account_resources(address),
        ledger_version
    )
}

fn account_modules(address: &str) -> String {
    format!("/accounts/{}/modules", address)
}

fn account_modules_with_ledger_version(address: &str, ledger_version: i128) -> String {
    format!(
        "{}?ledger_version={}",
        account_modules(address),
        ledger_version
    )
}
