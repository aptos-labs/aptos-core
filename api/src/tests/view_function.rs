// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{new_test_context, new_test_context_with_config};
use aptos_api_test_context::{current_function_name, TestContext};
use aptos_cached_packages::aptos_stdlib;
use aptos_config::config::{NodeConfig, ViewFilter, ViewFunctionId};
use aptos_types::account_address::AccountAddress;
use serde_json::{json, Value};
use std::{path::PathBuf, str::FromStr};

fn build_coin_balance_request(address: &AccountAddress) -> Value {
    json!({
        "function":"0x1::coin::balance",
        "arguments": vec![address.to_string()],
        "type_arguments": vec!["0x1::aptos_coin::AptosCoin"],
    })
}

fn build_coin_decimals_request() -> Value {
    let arguments: Vec<String> = Vec::new();
    json!({
        "function":"0x1::coin::decimals",
        "arguments": arguments,
        "type_arguments": vec!["0x1::aptos_coin::AptosCoin"],
    })
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_simple_view() {
    let mut context = new_test_context(current_function_name!());
    let creator = &mut context.gen_account();
    let owner = &mut context.gen_account();
    let txn1 = context.mint_user_account(creator).await;
    let txn2 = context.account_transfer(creator, owner, 100_000);

    context.commit_block(&vec![txn1, txn2]).await;

    let resp = context
        .post("/view", build_coin_balance_request(&owner.address()))
        .await;

    context.check_golden_output_no_prune(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_view_gas_used_header() {
    let mut context = new_test_context(current_function_name!());
    let creator = &mut context.gen_account();
    let owner = &mut context.gen_account();
    let txn1 = context.mint_user_account(creator).await;
    let txn2 = context.account_transfer(creator, owner, 100_000);

    context.commit_block(&vec![txn1, txn2]).await;
    context.wait_for_internal_indexer_caught_up().await;

    let req = warp::test::request()
        .method("POST")
        .path("/v1/view")
        .json(&build_coin_balance_request(&owner.address()));
    let resp = context.reply(req).await;

    // Confirm the gas used header is present.
    assert!(
        resp.headers()
            .get("X-Aptos-Gas-Used")
            .unwrap()
            .to_str()
            .unwrap()
            .parse::<u64>()
            .unwrap()
            > 0
    );

    context.check_golden_output_no_prune(serde_json::from_slice(resp.body()).unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_view_allowlist() {
    let mut node_config = NodeConfig::default();

    // Allowlist only the balance function.
    node_config.api.view_filter = ViewFilter::Allowlist(vec![ViewFunctionId {
        address: AccountAddress::from_str("0x1").unwrap(),
        module: "coin".to_string(),
        function_name: "balance".to_string(),
    }]);

    let mut context = new_test_context_with_config(current_function_name!(), node_config);

    let creator = &mut context.gen_account();
    let owner = &mut context.gen_account();
    let txn1 = context.mint_user_account(creator).await;
    let txn2 = context.account_transfer(creator, owner, 100_000);

    context.commit_block(&vec![txn1, txn2]).await;

    // See that an allowed function works.
    let resp1 = context
        .expect_status_code(200)
        .post("/view", build_coin_balance_request(&owner.address()))
        .await;

    // See that a non-allowed function is rejected.
    let resp2 = context
        .expect_status_code(403)
        .post("/view", build_coin_decimals_request())
        .await;

    context.check_golden_output_no_prune(json!(vec![resp1, resp2]));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_view_blocklist() {
    let mut node_config = NodeConfig::default();

    // Blocklist the balance function.
    node_config.api.view_filter = ViewFilter::Blocklist(vec![ViewFunctionId {
        address: AccountAddress::from_str("0x1").unwrap(),
        module: "coin".to_string(),
        function_name: "balance".to_string(),
    }]);

    let mut context = new_test_context_with_config(current_function_name!(), node_config);

    let creator = &mut context.gen_account();
    let owner = &mut context.gen_account();
    let txn1 = context.mint_user_account(creator).await;
    let txn2 = context.account_transfer(creator, owner, 100_000);

    context.commit_block(&vec![txn1, txn2]).await;

    // See that a blocked function is rejected.
    let resp1 = context
        .expect_status_code(403)
        .post("/view", build_coin_balance_request(&owner.address()))
        .await;

    // See that a non-blocked function is allowed.
    let resp2 = context
        .expect_status_code(200)
        .post("/view", build_coin_decimals_request())
        .await;

    context.check_golden_output_no_prune(json!(vec![resp1, resp2]));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_view_error_type_resolution_error() {
    let mut context = new_test_context(current_function_name!());
    let creator = &mut context.gen_account();
    let owner = &mut context.gen_account();
    let txn1 = context.mint_user_account(creator).await;
    let txn2 = context.account_transfer(creator, owner, 100_000);

    context.commit_block(&vec![txn1, txn2]).await;

    let resp = context
        .expect_status_code(400)
        .post(
            "/view",
            json!({
                "function":"0x1::coin::is_account_registered",
                "arguments": vec![AccountAddress::random().to_string()],
                "type_arguments": ["0x1::aptos_coin::NewCoin"], // Does not exist
            }),
        )
        .await;
    context.check_golden_output_no_prune(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_view_error_move_abort() {
    let mut context = new_test_context(current_function_name!());
    let creator = &mut context.gen_account();
    let owner = &mut context.gen_account();
    let txn1 = context.mint_user_account(creator).await;
    let txn2 = context.account_transfer(creator, owner, 100_000);

    context.commit_block(&vec![txn1, txn2]).await;

    let resp = context
        .expect_status_code(400)
        .post(
            "/view",
            json!({
                "function":"0x1::account::get_rotation_capability_offer_for", // Rotation capability does not exist
                "arguments": vec![owner.address().to_string()],
                "type_arguments": [],
            }),
        )
        .await;
    context.check_golden_output_no_prune(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_view_does_not_exist() {
    let mut context = new_test_context(current_function_name!());
    let creator = &mut context.gen_account();
    let owner = &mut context.gen_account();
    let txn1 = context.mint_user_account(creator).await;
    let txn2 = context.account_transfer(creator, owner, 100_000);

    context.commit_block(&vec![txn1, txn2]).await;

    let resp = context
        .expect_status_code(400)
        .post(
            "/view",
            json!({
                "function":"0x1::aptos_account::fake_function",
                "arguments": vec![owner.address().to_string()],
                "type_arguments": [],
            }),
        )
        .await;

    context.check_golden_output_no_prune(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_simple_view_invalid() {
    let mut context = new_test_context(current_function_name!());
    let creator = &mut context.gen_account();
    let owner = &mut context.gen_account();
    let txn1 = context.mint_user_account(creator).await;
    let txn2 = context.account_transfer(creator, owner, 100_000);

    context.commit_block(&vec![txn1, txn2]).await;

    let resp = context
        .expect_status_code(400)
        .post(
            "/view",
            json!({
                "function":"0x1::aptos_account::assert_account_exists",
                "arguments": vec![owner.address().to_string()],
                "type_arguments": [],
            }),
        )
        .await;

    context.check_golden_output_no_prune(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_versioned_simple_view() {
    let mut context = new_test_context(current_function_name!());
    let creator = &mut context.gen_account();
    let owner = &mut context.gen_account();
    let txn1 = context.mint_user_account(creator).await;
    let txn2 = context.account_transfer(creator, owner, 100_000);
    let txn3 = context.account_transfer(creator, owner, 100_000);

    context.commit_block(&vec![txn1, txn2, txn3]).await;

    let resp = context
        .post(
            "/view?ledger_version=3",
            json!({
                "function":"0x1::coin::balance",
                "arguments": vec![owner.address().to_string()],
                "type_arguments": vec!["0x1::aptos_coin::AptosCoin"],
            }),
        )
        .await;

    context.check_golden_output_no_prune(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_view_tuple() {
    let mut context = new_test_context(current_function_name!());
    let payload = aptos_stdlib::publish_module_source(
        "test_module",
        r#"
        module 0xa550c18::test_module {
            #[view]
            public fun return_tuple(): (u64, u64) {
                (1, 2)
            }
        }
        "#,
    );

    let root_account = context.root_account().await;
    let module_txn =
        root_account.sign_with_transaction_builder(context.transaction_factory().payload(payload));

    context.commit_block(&vec![module_txn]).await;

    let resp = context
        .post(
            "/view",
            json!({
                "function": "0xa550c18::test_module::return_tuple",
                "arguments": [],
                "type_arguments": [],
            }),
        )
        .await;
    context.check_golden_output_no_prune(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_view_aggregator() {
    let mut context = new_test_context(current_function_name!());
    let account = context.root_account().await;

    let named_addresses = vec![("addr".to_string(), account.address())];
    let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR")).join("src/tests/move/pack_counter");
    let payload = TestContext::build_package(path, named_addresses);
    let txn = account.sign_with_transaction_builder(context.transaction_factory().payload(payload));
    context.commit_block(&vec![txn]).await;

    let function = format!("{}::counter::add_and_get_counter_value", account.address());
    let resp = context
        .post(
            "/view",
            json!({
                "function": function,
                "arguments": [],
                "type_arguments": [],
            }),
        )
        .await;
    context.check_golden_output_no_prune(resp);
}
