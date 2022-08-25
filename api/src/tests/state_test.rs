// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::new_test_context;
use aptos_api_test_context::{current_function_name, TestContext};
use aptos_sdk::types::LocalAccount;
use move_deps::{move_core_types::account_address::AccountAddress, move_package::BuildConfig};
use serde::Serialize;
use serde_json::{json, Value};
use std::{convert::TryInto, path::PathBuf};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_account_resource() {
    let mut context = new_test_context(current_function_name!());
    let resp = context
        .get(&get_account_resource("0xA550C18", "0x1::account::Account"))
        .await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_account_resource_by_invalid_address() {
    let mut context = new_test_context(current_function_name!());
    let invalid_addresses = vec!["1", "0xzz", "01"];
    for invalid_address in &invalid_addresses {
        let resp = context
            .expect_status_code(400)
            .get(&get_account_resource(
                invalid_address,
                "0x1::guid::Generator",
            ))
            .await;
        context.check_golden_output(resp);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_account_resource_by_invalid_struct_tag() {
    let mut context = new_test_context(current_function_name!());
    let resp = context
        .expect_status_code(400)
        .get(&get_account_resource("0xA550C18", "0x1::GUID_Generator"))
        .await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_account_resource_address_not_found() {
    let mut context = new_test_context(current_function_name!());
    let resp = context
        .expect_status_code(404)
        .get(&get_account_resource("0xA550C19", "0x1::guid::Generator"))
        .await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_account_resource_struct_tag_not_found() {
    let mut context = new_test_context(current_function_name!());
    let resp = context
        .expect_status_code(404)
        .get(&get_account_resource("0xA550C19", "0x1::guid::GeneratorX"))
        .await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_account_resource_with_version() {
    let mut context = new_test_context(current_function_name!());
    let ledger_version = context.get_latest_ledger_info().version();
    let resp = context
        .get(&get_account_resource_with_version(
            "0xA550C18",
            "0x1::account::Account",
            ledger_version,
        ))
        .await;

    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_account_resource_with_version_too_large() {
    let mut context = new_test_context(current_function_name!());
    let resp = context
        .expect_status_code(404)
        .get(&get_account_resource_with_version(
            "0xA550C18",
            "0x1::account::Account",
            100000000,
        ))
        .await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_account_module() {
    let mut context = new_test_context(current_function_name!());
    let resp = context.get(&get_account_module("0x1", "guid")).await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_account_module_by_invalid_address() {
    let mut context = new_test_context(current_function_name!());
    let resp = context
        .expect_status_code(400)
        .get(&get_account_module("1", "guid"))
        .await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_account_module_not_found() {
    let mut context = new_test_context(current_function_name!());
    let resp = context
        .expect_status_code(404)
        .get(&get_account_module("0x1", "NoNoNo"))
        .await;
    context.check_golden_output(resp);
}

#[ignore] // TODO: deactivate because of module-bundle publish not longer there; reactivate.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_table_item() {
    let mut context = new_test_context(current_function_name!());
    let ctx = &mut context;
    let mut account = ctx.gen_account();
    let acc = &mut account;
    let txn = ctx.create_user_account(acc);
    ctx.commit_block(&vec![txn.clone()]).await;
    make_test_tables(ctx, acc).await;

    // get the TestTables instance
    let tt = ctx
        .api_get_account_resource(
            acc,
            &acc.address().to_hex_literal(),
            "TableTestData",
            "TestTables",
        )
        .await["data"]
        .to_owned();

    assert_table_item(ctx, &tt["u8_table"], "u8", "u8", 1u8, 1u8).await;
    assert_table_item(ctx, &tt["u64_table"], "u64", "u64", "1", "1").await;
    assert_table_item(ctx, &tt["u128_table"], "u128", "u128", "1", "1").await;
    assert_table_item(ctx, &tt["bool_table"], "bool", "bool", true, true).await;
    assert_table_item(
        ctx,
        &tt["address_table"],
        "address",
        "address",
        "0x1",
        "0x1",
    )
    .await;
    assert_table_item(
        ctx,
        &tt["string_table"],
        "0x1::string::String",
        "0x1::string::String",
        "abc",
        "abc",
    )
    .await;
    assert_table_item(
        ctx,
        &tt["vector_u8_table"],
        "vector<u8>",
        "vector<u8>",
        "0x0102",
        "0x0102",
    )
    .await;
    assert_table_item(
        ctx,
        &tt["vector_string_table"],
        "vector<0x1::string::String>",
        "vector<0x1::string::String>",
        ["abc", "abc"],
        ["abc", "abc"],
    )
    .await;
    let id = &tt["id_table_id"];
    assert_table_item(
        ctx,
        &tt["id_table"],
        "0x1::guid::ID",
        "0x1::guid::ID",
        id,
        id,
    )
    .await;
    let nested_table = api_get_table_item(
        ctx,
        &tt["table_table"],
        "u8",
        "0x1::table::Table<u8, u8>",
        1u8,
    )
    .await;
    assert_table_item(ctx, &nested_table, "u8", "u8", 2, 3).await;
}

fn get_account_resource(address: &str, struct_tag: &str) -> String {
    format!("/accounts/{}/resource/{}", address, struct_tag)
}

fn get_account_resource_with_version(address: &str, struct_tag: &str, version: u64) -> String {
    format!(
        "/accounts/{}/resource/{}?ledger_version={}",
        address, struct_tag, version
    )
}

fn get_account_module(address: &str, name: &str) -> String {
    format!("/accounts/{}/module/{}", address, name)
}

fn get_table_item(handle: AccountAddress) -> String {
    format!("/tables/{}/item", handle)
}

async fn make_test_tables(ctx: &mut TestContext, account: &mut LocalAccount) {
    let module = build_test_module(account.address()).await;

    ctx.api_publish_module(account, module.try_into().unwrap())
        .await;
    ctx.api_execute_entry_function(
        account,
        "TableTestData",
        "make_test_tables",
        json!([]),
        json!([]),
    )
    .await
}

async fn build_test_module(account: AccountAddress) -> Vec<u8> {
    let package_dir = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("api/move-test-package");
    let build_config = BuildConfig {
        generate_docs: false,
        install_dir: Some(package_dir.clone()),
        additional_named_addresses: [("TestAccount".to_string(), account)].into(),
        ..Default::default()
    };
    let package = build_config
        .compile_package(&package_dir, &mut Vec::new())
        .unwrap();

    let mut out = Vec::new();
    package
        .root_modules_map()
        .iter_modules()
        .first()
        .unwrap()
        .serialize(&mut out)
        .unwrap();
    out
}

async fn api_get_table_item<T: Serialize>(
    ctx: &mut TestContext,
    table: &Value,
    key_type: &str,
    value_type: &str,
    key: T,
) -> Value {
    let handle = table["handle"].as_str().unwrap().parse().unwrap();
    ctx.post(
        &get_table_item(handle),
        json!({
            "key_type": key_type,
            "value_type": value_type,
            "key": key,
        }),
    )
    .await
}

async fn assert_table_item<T: Serialize, U: Serialize>(
    ctx: &mut TestContext,
    table: &Value,
    key_type: &str,
    value_type: &str,
    key: T,
    value: U,
) {
    let response = api_get_table_item(ctx, table, key_type, value_type, key).await;
    assert_eq!(response, json!(value));
}
