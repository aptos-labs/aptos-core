// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::new_test_context;
use aptos_api_test_context::current_function_name;
use serde_json::json;
use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_package_builder::PackageBuilder;

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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_view_tuple() {
    let mut context = new_test_context(current_function_name!());

    let source = r#"
        module 0xa550c18::test_module {

            #[view]
            public fun return_tuple(): (u64, u64) {
                (1, 2)
            }
        }
        "#;
    let mut builder = PackageBuilder::new("test_package");
    builder.add_source("test_module.move", source);
    let path = builder.write_to_temp().expect("Should be able to write to tmp directory");

    let package = BuiltPackage::build(path.path().to_path_buf(), BuildOptions::default()).expect("Should be able to build a package");
    let code = package.extract_code();
    let metadata = package.extract_metadata().expect("Should be able to extract metadata");
    let payload = aptos_cached_packages::aptos_stdlib::code_publish_package_txn(
        bcs::to_bytes(&metadata).expect("Should be able to serialize metadata"),
        code,
    );

    let root_account = context.root_account().await;
    let module_txn = root_account
        .sign_with_transaction_builder(context.transaction_factory().payload(payload));

    context.commit_block(&vec![module_txn]).await;

    let resp = context
        .post(
            "/view",
            json!({
                "function":"0xa550c18::test_module::return_tuple",
                "arguments": [],
                "type_arguments": [],
            }),
        )
        .await;
    context.check_golden_output_no_prune(resp);
}
