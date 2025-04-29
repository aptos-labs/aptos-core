// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::new_test_context;
use aptos_api_test_context::{current_function_name, TestContext};
use aptos_sdk::{transaction_builder::aptos_stdlib::aptos_token_stdlib, types::LocalAccount};
use aptos_storage_interface::DbReader;
use move_core_types::account_address::AccountAddress;
use serde::Serialize;
use serde_json::{json, Value};
use std::path::PathBuf;

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
    let invalid_addresses = vec!["00x1", "0xzz"];
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
        .get(&get_account_module("xyz", "guid"))
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_merkle_leaves_with_nft_transfer() {
    let mut context = new_test_context(current_function_name!());
    let num_block_resource = 1;

    let ctx = &mut context;
    let creator = &mut ctx.gen_account();
    let owner = &mut ctx.gen_account();
    let txn1 = ctx.mint_user_account(creator).await;
    let txn2 = ctx.account_transfer(creator, owner, 100_000);

    let collection_name = "collection name".to_owned().into_bytes();
    let token_name = "token name".to_owned().into_bytes();
    let collection_builder =
        ctx.transaction_factory()
            .payload(aptos_token_stdlib::token_create_collection_script(
                collection_name.clone(),
                "description".to_owned().into_bytes(),
                "uri".to_owned().into_bytes(),
                20_000_000,
                vec![false, false, false],
            ));

    let collection_txn = creator.sign_with_transaction_builder(collection_builder);

    let token_builder =
        ctx.transaction_factory()
            .payload(aptos_token_stdlib::token_create_token_script(
                collection_name.clone(),
                token_name.clone(),
                "collection description".to_owned().into_bytes(),
                3,
                4,
                "uri".to_owned().into_bytes(),
                creator.address(),
                1,
                0,
                vec![false, false, false, false, true],
                vec!["age".as_bytes().to_vec()],
                vec!["3".as_bytes().to_vec()],
                vec!["int".as_bytes().to_vec()],
            ));

    let token_txn = creator.sign_with_transaction_builder(token_builder);

    ctx.commit_block(&vec![txn1, txn2, collection_txn, token_txn])
        .await;

    let num_leaves_at_beginning = ctx
        .db
        .get_state_item_count(ctx.db.get_latest_ledger_info_version().unwrap())
        .unwrap();

    let transfer_to_owner_txn = creator.sign_multi_agent_with_transaction_builder(
        vec![owner],
        ctx.transaction_factory()
            .payload(aptos_token_stdlib::token_direct_transfer_script(
                creator.address(),
                collection_name.clone(),
                token_name.clone(),
                0,
                1,
            )),
    );
    ctx.commit_block(&vec![transfer_to_owner_txn]).await;
    let num_leaves_after_transfer_nft = ctx
        .db
        .get_state_item_count(ctx.db.get_latest_ledger_info_version().unwrap())
        .unwrap();
    assert_eq!(
        num_leaves_after_transfer_nft,
        num_leaves_at_beginning + 3 /* 1 token store + 1 token + 1 account resource */ + num_block_resource
    );

    let transfer_to_creator_txn = owner.sign_multi_agent_with_transaction_builder(
        vec![creator],
        ctx.transaction_factory()
            .payload(aptos_token_stdlib::token_direct_transfer_script(
                creator.address(),
                collection_name.clone(),
                token_name.clone(),
                0,
                1,
            )),
    );
    ctx.commit_block(&vec![transfer_to_creator_txn]).await;
    let num_leaves_after_return_nft = ctx
        .db
        .get_state_item_count(ctx.db.get_latest_ledger_info_version().unwrap())
        .unwrap();

    assert_eq!(
        num_leaves_after_return_nft,
        num_leaves_at_beginning + 2 + num_block_resource * 2
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_table_item() {
    let mut context = new_test_context(current_function_name!());
    let ctx = &mut context;
    let mut acc = ctx.root_account().await;
    make_test_tables(ctx, &mut acc).await;

    // get the TestTables instance
    let tt = ctx
        .api_get_account_resource(
            acc.address(),
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
    let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("api/move-test-package");
    let txn =
        TestContext::build_package(path, vec![("TestAccount".to_string(), account.address())]);
    ctx.publish_package(account, txn).await;

    ctx.api_execute_entry_function(
        account,
        &format!(
            "0x{}::TableTestData::make_test_tables",
            account.address().to_hex()
        ),
        json!([]),
        json!([]),
    )
    .await
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
