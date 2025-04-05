// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done
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
async fn test_renders_move_acsii_string_into_utf8_string(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) {
    let mut context = new_test_context_with_orderless_flags(
        current_function_name!(),
        use_txn_payload_v2_format,
        use_orderless_transactions,
    )
    .await;
    let mut account = context.root_account().await;
    let addr = account.address();

    let named_addresses = vec![("addr".to_string(), addr)];
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/tests/move/pack_string_resource");
    let payload = TestContext::build_package(path, named_addresses);
    let txn = account.sign_with_transaction_builder(
        context
            .transaction_factory()
            .payload(payload)
            .expiration_timestamp_secs(context.get_expiration_time())
            .upgrade_payload(
                context.use_txn_payload_v2_format,
                context.use_orderless_transactions,
            ),
    );
    context.commit_block(&vec![txn]).await;

    context
        .api_execute_entry_function(
            &mut account,
            &format!("0x{}::message::set_message", addr.to_hex()),
            json!([]),
            json!([hex::encode(b"hello world")]),
        )
        .await;

    let message = context
        .api_get_account_resource(addr, &addr.to_hex_literal(), "message", "MessageHolder")
        .await;
    assert_eq!("hello world", message["data"]["message"]);
}
