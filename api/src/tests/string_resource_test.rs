// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::new_test_context;
use aptos_api_test_context::{current_function_name, TestContext};
use serde_json::json;
use std::path::PathBuf;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_renders_move_acsii_string_into_utf8_string() {
    let mut context = new_test_context(current_function_name!());
    let mut account = context.root_account().await;
    let addr = account.address();

    let named_addresses = vec![("addr".to_string(), addr)];
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/tests/move/pack_string_resource");
    let payload = TestContext::build_package(path, named_addresses);
    let txn = account.sign_with_transaction_builder(context.transaction_factory().payload(payload));
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
