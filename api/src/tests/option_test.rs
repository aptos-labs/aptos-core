// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Innovation-Enabling Source Code License

use super::new_test_context_with_orderless_flags;
use aptos_api_test_context::{current_function_name, TestContext};
use rstest::rstest;
use serde_json::json;
use std::path::PathBuf;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(
    use_txn_payload_v2_format,
    use_orderless_transactions,
    case(false, false),
    case(true, false),
    case(true, true)
)]
async fn run_option(use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    );
    let mut account = context.create_account().await;
    let account_addr = account.address();

    // Publish packages
    let named_addresses = vec![("account".to_string(), account_addr)];
    let txn = futures::executor::block_on(async move {
        let path =
            PathBuf::from(std::env!("CARGO_MANIFEST_DIR")).join("src/tests/move/test_option");
        TestContext::build_package_with_latest_language(path, named_addresses)
    });
    context.publish_package(&mut account, txn).await;

    // Run view function
    let function = format!("{}::test_module::return_some", account.address());
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

    // Publish packages
    let named_addresses = vec![("account".to_string(), account_addr)];
    let txn = futures::executor::block_on(async move {
        let path = PathBuf::from(std::env!("CARGO_MANIFEST_DIR"))
            .join("src/tests/move/pack_function_values_with_struct");
        TestContext::build_package_with_latest_language(path, named_addresses)
    });
    context.publish_package(&mut account, txn).await;

    // Run entry function with option as argument
    context
        .api_execute_entry_function(
            &mut account,
            &format!("0x{}::test::entry_function", account_addr.to_hex()),
            json!([]),
            json!([{"vec": ["1"]}]),
        )
        .await;

    let resource = format!("{}::test::R2<0x1::option::Option<u128>>", account_addr);
    let response = &context
        .gen_resource(&account_addr, &resource)
        .await
        .unwrap();

    assert_eq!(
        response["data"],
        json!({
            "_0": {"vec":["1"]},
        })
    );
}
