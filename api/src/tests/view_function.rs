// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::new_test_context;
use aptos_api_test_context::current_function_name;
use serde_json::json;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_simple_view() {
    let mut context = new_test_context(current_function_name!());
    let creator = &mut context.gen_account();
    let owner = &mut context.gen_account();
    let txn1 = context.mint_user_account(creator).await;
    let txn2 = context.account_transfer(creator, owner, 100_000);

    context.commit_block(&vec![txn1, txn2]).await;

    let resp = context
        .post(
            "/view",
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

#[ignore] // TODO: reactivate with real source
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_view_tuple() {
    aptos_vm::aptos_vm::allow_module_bundle_for_test();
    let mut context = new_test_context(current_function_name!());

    // address 0xA550C18 {
    //     module TableTestData {
    //         public fun return_tuple(): (u64, u64) {
    //             (1, 2)
    //         }
    //     }
    // }
    let tuple_module = hex::decode("a11ceb0b0500000006010002030205050704070b1b0826200c461900000001000100000203030d5461626c6554657374446174610c72657475726e5f7475706c65000000000000000000000000000000000000000000000000000000000a550c180001000000030601000000000000000602000000000000000200").unwrap();
    let root_account = context.root_account().await;
    let module_txn = root_account
        .sign_with_transaction_builder(context.transaction_factory().module(tuple_module));

    context.commit_block(&vec![module_txn]).await;

    let resp = context
        .post(
            "/view",
            json!({
                "function":"0xa550c18::TableTestData::return_tuple",
                "arguments": [],
                "type_arguments": [],
            }),
        )
        .await;
    context.check_golden_output_no_prune(resp);
}
