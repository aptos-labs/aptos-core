// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{current_function_name, tests::new_test_context};
use serde_json::{json, Value};

#[tokio::test]
async fn test_invalid_type_argument_data_type() {
    let mut req = signing_message_request();
    req["payload"]["type_arguments"] = json!([true]);

    response_error_msg(req, current_function_name!()).await;
}

#[tokio::test]
async fn test_invalid_script_function_argument_data_type() {
    let mut req = signing_message_request();
    req["payload"]["arguments"][0] = json!(true);

    response_error_msg(req, current_function_name!()).await;
}

#[tokio::test]
async fn test_invalid_script_function_argument_u64_string() {
    let mut req = signing_message_request();
    req["payload"]["arguments"][0] = json!("invalid");

    response_error_msg(req, current_function_name!()).await;
}

#[tokio::test]
async fn test_invalid_script_function_argument_address_type() {
    let mut req = signing_message_request();
    req["payload"]["arguments"][0] = json!(1);

    response_error_msg(req, current_function_name!()).await;
}

#[tokio::test]
async fn test_invalid_script_function_argument_address_string() {
    let mut req = signing_message_request();
    req["payload"]["arguments"][0] = json!("invalid");

    response_error_msg(req, current_function_name!()).await;
}

#[tokio::test]
async fn test_missing_script_function_arguments() {
    let mut req = signing_message_request();
    req["payload"]["arguments"] = json!(["0"]);

    response_error_msg(req, current_function_name!()).await;
}

#[tokio::test]
async fn test_invalid_script_function_function_name() {
    let mut req = signing_message_request();
    req["payload"]["function"] = json!("0x1::Account::invalid");

    response_error_msg(req, current_function_name!()).await;
}

#[tokio::test]
async fn test_invalid_script_function_module_name() {
    let mut req = signing_message_request();
    req["payload"]["function"] = json!("0x1::Invalid::invalid");

    response_error_msg(req, current_function_name!()).await;
}

#[tokio::test]
async fn test_invalid_script_function_module_address() {
    let mut req = signing_message_request();
    req["payload"]["function"] = json!("0x2342342342::Invalid::invalid");

    response_error_msg(req, current_function_name!()).await;
}

#[tokio::test]
async fn test_invalid_script_function_function_id() {
    let mut req = signing_message_request();
    req["payload"]["function"] = json!("invalid");

    response_error_msg(req, current_function_name!()).await;
}

#[tokio::test]
async fn test_invalid_payload_type() {
    let mut req = signing_message_request();
    req["payload"]["type"] = json!("invalid");

    response_error_msg(req, current_function_name!()).await;
}

#[tokio::test]
async fn test_invalid_payload_data_type() {
    let mut req = signing_message_request();
    req["payload"] = json!(1234);

    response_error_msg(req, current_function_name!()).await;
}

#[tokio::test]
async fn test_invalid_sender_address() {
    let mut req = signing_message_request();
    req["sender"] = json!("invalid");
}

#[tokio::test]
async fn test_invalid_sequence_number() {
    let mut req = signing_message_request();
    req["sequence_number"] = json!("invalid");
}

async fn response_error_msg(req: Value, test_name: &'static str) {
    let mut context = new_test_context(test_name);
    let resp = context
        .expect_status_code(400)
        .post("/transactions/signing_message", req)
        .await;
    context.check_golden_output(resp)
}

fn signing_message_request() -> Value {
    json!({
        "sender": "0xdd",
        "sequence_number": "0",
        "gas_unit_price": "0",
        "max_gas_amount": "1000000",
        "gas_currency_code": "XUS",
        "expiration_timestamp_secs": "9991638487317",
        "payload": {
            "type": "script_function_payload",
            "function": "0x1::Account::create_account",
            "type_arguments": [
            ],
            "arguments": [
                "0x00000000000000000000000001234567", // address
            ]
        }
    })
}
