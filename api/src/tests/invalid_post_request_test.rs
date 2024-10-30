// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done
use super::new_test_context_with_orderless_flags;
use aptos_api_test_context::{current_function_name, TestContext};
use rstest::rstest;
use serde_json::{json, Value};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(use_orderless_transactions, case(false), case(true))]
async fn test_invalid_type_argument_data_type(use_orderless_transactions: bool) {
    let context = new_test_context_with_orderless_flags(
        current_function_name!(),
        true,
        use_orderless_transactions,
    )
    .await;
    let mut req = signing_message_request(context.use_orderless_transactions);
    req["payload"]["type_arguments"] = json!([true]);

    response_error_msg(req, context).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(use_orderless_transactions, case(false), case(true))]
async fn test_invalid_entry_function_argument_data_type(use_orderless_transactions: bool) {
    let context = new_test_context_with_orderless_flags(
        current_function_name!(),
        true,
        use_orderless_transactions,
    )
    .await;
    let mut req = signing_message_request(context.use_orderless_transactions);
    req["payload"]["arguments"][0] = json!(true);

    response_error_msg(req, context).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(use_orderless_transactions, case(false), case(true))]
async fn test_invalid_entry_function_argument_u64_string(use_orderless_transactions: bool) {
    let context = new_test_context_with_orderless_flags(
        current_function_name!(),
        true,
        use_orderless_transactions,
    )
    .await;
    let mut req = signing_message_request(context.use_orderless_transactions);
    req["payload"]["arguments"][0] = json!("invalid");

    response_error_msg(req, context).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(use_orderless_transactions, case(false), case(true))]
async fn test_invalid_entry_function_argument_address_type(use_orderless_transactions: bool) {
    let context = new_test_context_with_orderless_flags(
        current_function_name!(),
        true,
        use_orderless_transactions,
    )
    .await;
    let mut req = signing_message_request(context.use_orderless_transactions);
    req["payload"]["arguments"][0] = json!(1);

    response_error_msg(req, context).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(use_orderless_transactions, case(false), case(true))]
async fn test_invalid_entry_function_argument_address_string(use_orderless_transactions: bool) {
    let context = new_test_context_with_orderless_flags(
        current_function_name!(),
        true,
        use_orderless_transactions,
    )
    .await;
    let mut req = signing_message_request(context.use_orderless_transactions);
    req["payload"]["arguments"][0] = json!("invalid");

    response_error_msg(req, context).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(use_orderless_transactions, case(false), case(true))]
async fn test_missing_entry_function_arguments(use_orderless_transactions: bool) {
    let context = new_test_context_with_orderless_flags(
        current_function_name!(),
        true,
        use_orderless_transactions,
    )
    .await;
    let mut req = signing_message_request(context.use_orderless_transactions);
    req["payload"]["arguments"] = json!([]);

    response_error_msg(req, context).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(use_orderless_transactions, case(false), case(true))]
async fn test_invalid_entry_function_function_name(use_orderless_transactions: bool) {
    let context = new_test_context_with_orderless_flags(
        current_function_name!(),
        true,
        use_orderless_transactions,
    )
    .await;
    let mut req = signing_message_request(context.use_orderless_transactions);
    req["payload"]["function"] = json!("0x1::account::invalid");

    response_error_msg(req, context).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(use_orderless_transactions, case(false), case(true))]
async fn test_invalid_entry_function_module_name(use_orderless_transactions: bool) {
    let context = new_test_context_with_orderless_flags(
        current_function_name!(),
        true,
        use_orderless_transactions,
    )
    .await;
    let mut req = signing_message_request(context.use_orderless_transactions);
    req["payload"]["function"] = json!("0x1::invalid::invalid");

    response_error_msg(req, context).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(use_orderless_transactions, case(false), case(true))]
async fn test_invalid_entry_function_module_address(use_orderless_transactions: bool) {
    let context = new_test_context_with_orderless_flags(
        current_function_name!(),
        true,
        use_orderless_transactions,
    )
    .await;
    let mut req = signing_message_request(context.use_orderless_transactions);
    req["payload"]["function"] = json!("0x2342342342::Invalid::invalid");

    response_error_msg(req, context).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(use_orderless_transactions, case(false), case(true))]
async fn test_invalid_entry_function_function_id(use_orderless_transactions: bool) {
    let context = new_test_context_with_orderless_flags(
        current_function_name!(),
        true,
        use_orderless_transactions,
    )
    .await;
    let mut req = signing_message_request(context.use_orderless_transactions);
    req["payload"]["function"] = json!("invalid");

    response_error_msg(req, context).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(use_orderless_transactions, case(false), case(true))]
async fn test_invalid_payload_type(use_orderless_transactions: bool) {
    let context = new_test_context_with_orderless_flags(
        current_function_name!(),
        true,
        use_orderless_transactions,
    )
    .await;
    let mut req = signing_message_request(context.use_orderless_transactions);
    req["payload"]["type"] = json!("invalid");

    response_error_msg(req, context).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(use_orderless_transactions, case(false), case(true))]
async fn test_invalid_payload_data_type(use_orderless_transactions: bool) {
    let context = new_test_context_with_orderless_flags(
        current_function_name!(),
        true,
        use_orderless_transactions,
    )
    .await;
    let mut req = signing_message_request(context.use_orderless_transactions);
    req["payload"] = json!(1234);

    response_error_msg(req, context).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(use_orderless_transactions, case(false), case(true))]
async fn test_invalid_sender_address(use_orderless_transactions: bool) {
    let context = new_test_context_with_orderless_flags(
        current_function_name!(),
        true,
        use_orderless_transactions,
    )
    .await;
    let mut req = signing_message_request(context.use_orderless_transactions);
    req["sender"] = json!("invalid");

    response_error_msg(req, context).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[rstest(use_orderless_transactions, case(false), case(true))]
async fn test_invalid_sequence_number(use_orderless_transactions: bool) {
    let context = new_test_context_with_orderless_flags(
        current_function_name!(),
        true,
        use_orderless_transactions,
    )
    .await;
    let mut req = signing_message_request(context.use_orderless_transactions);
    req["sequence_number"] = json!("invalid");

    response_error_msg(req, context).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_invalid_replay_protection_nonce() {
    let context = new_test_context_with_orderless_flags(current_function_name!(), true, true).await;
    let mut req = signing_message_request(context.use_orderless_transactions);
    req["replay_protection_nonce"] = json!("invalid");

    response_error_msg(req, context).await;
}

async fn response_error_msg(req: Value, mut context: TestContext) {
    let resp = context
        .expect_status_code(400)
        .post("/transactions/encode_submission", req)
        .await;
    context.check_golden_output(resp)
}

fn signing_message_request(use_orderless_transactions: bool) -> Value {
    if use_orderless_transactions {
        json!({
            "sender": "0xdd",
            "sequence_number": "0",
            "gas_unit_price": "0",
            "max_gas_amount": "1000000",
            "expiration_timestamp_secs": "9991638487317",
            "payload": {
                "type": "entry_function_payload",
                "function": "0x1::aptos_account::create_account",
                "type_arguments": [
                ],
                "arguments": [
                    "0x00000000000000000000000001234567", // address
                ]
            },
            "replay_protection_nonce": "1342341341",
        })
    } else {
        json!({
            "sender": "0xdd",
            "sequence_number": "0",
            "gas_unit_price": "0",
            "max_gas_amount": "1000000",
            "expiration_timestamp_secs": "9991638487317",
            "payload": {
                "type": "entry_function_payload",
                "function": "0x1::aptos_account::create_account",
                "type_arguments": [
                ],
                "arguments": [
                    "0x00000000000000000000000001234567", // address
                ]
            },
        })
    }
}
