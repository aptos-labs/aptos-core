// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::tests::new_test_context;

use serde_json::{json, Value};

#[tokio::test]
async fn test_invalid_type_argument() {
    let mut req = signing_message_request();
    req["payload"]["type_arguments"][0] = json!("Invalid");

    assert_error_msg(req, "parse Move type \"Invalid\" failed").await;
}

#[tokio::test]
async fn test_missing_type_arguments() {
    let mut req = signing_message_request();
    req["payload"]["type_arguments"] = json!([]);

    assert_error_msg(req,
 "expect 1 type arguments for script function 0x1::AccountCreationScripts::create_parent_vasp_account, but got 0").await;
}

#[tokio::test]
async fn test_invalid_type_argument_data_type() {
    let mut req = signing_message_request();
    req["payload"]["type_arguments"] = json!([true]);

    assert_error_msg(
        req,
        "deserialize Move type failed, invalid type: boolean `true`, expected a string",
    )
    .await;
}

#[tokio::test]
async fn test_invalid_script_function_argument_data_type() {
    let mut req = signing_message_request();
    req["payload"]["arguments"][0] = json!(true);

    assert_error_msg(req, "parse arguments[0] failed, expect string<u64>, caused by error: invalid type: boolean `true`, expected a string").await;
}

#[tokio::test]
async fn test_invalid_script_function_argument_u64_string() {
    let mut req = signing_message_request();
    req["payload"]["arguments"][0] = json!("invalid");

    assert_error_msg(req, "parse arguments[0] failed, expect string<u64>, caused by error: parse u64 string \"invalid\" failed, caused by error: invalid digit found in string").await;
}

#[tokio::test]
async fn test_invalid_script_function_argument_address_type() {
    let mut req = signing_message_request();
    req["payload"]["arguments"][1] = json!(1);

    assert_error_msg(req, "parse arguments[1] failed, expect string<address>, caused by error: invalid type: integer `1`, expected a string").await;
}

#[tokio::test]
async fn test_invalid_script_function_argument_address_string() {
    let mut req = signing_message_request();
    req["payload"]["arguments"][1] = json!("invalid");

    assert_error_msg(req, "parse arguments[1] failed, expect string<address>, caused by error: invalid account address \"invalid\"").await;
}

#[tokio::test]
async fn test_invalid_script_function_argument_hex_encoded_bytes_type() {
    let mut req = signing_message_request();
    req["payload"]["arguments"][2] = json!({});

    assert_error_msg(req, "parse arguments[2] failed, expect string<hex>, caused by error: invalid type: map, expected a string").await;
}

#[tokio::test]
async fn test_invalid_script_function_argument_hex_string() {
    let mut req = signing_message_request();
    req["payload"]["arguments"][2] = json!("0xZZZ");

    assert_error_msg(req, "parse arguments[2] failed, expect string<hex>, caused by error: decode hex-encoded string(\"0xZZZ\") failed").await;
}

#[tokio::test]
async fn test_invalid_script_function_argument_boolean_type() {
    let mut req = signing_message_request();
    req["payload"]["arguments"][4] = json!("0x1");

    assert_error_msg(req, "parse arguments[4] failed, expect boolean, caused by error: invalid type: string \"0x1\", expected a boolean").await;
}

#[tokio::test]
async fn test_missing_script_function_arguments() {
    let mut req = signing_message_request();
    req["payload"]["arguments"] = json!(["0", 1, true]);

    assert_error_msg(req, "expected 5 arguments [string<u64>, string<address>, string<hex>, string<hex>, boolean], but got 3 ([String(\"0\"), Number(1), Bool(true)])").await;
}

#[tokio::test]
async fn test_invalid_script_function_function_name() {
    let mut req = signing_message_request();
    req["payload"]["function"] = json!("0x1::AccountCreationScripts::invalid");

    assert_error_msg(
        req,
        "could not find script function by 0x1::AccountCreationScripts::invalid",
    )
    .await;
}

#[tokio::test]
async fn test_invalid_script_function_module_name() {
    let mut req = signing_message_request();
    req["payload"]["function"] = json!("0x1::Invalid::invalid");

    assert_error_msg(req, "Module ModuleId { address: 00000000000000000000000000000001, name: Identifier(\"Invalid\") } can't be found").await;
}

#[tokio::test]
async fn test_invalid_script_function_module_address() {
    let mut req = signing_message_request();
    req["payload"]["function"] = json!("0x2342342342::Invalid::invalid");

    assert_error_msg(req, "Module ModuleId { address: 00000000000000000000002342342342, name: Identifier(\"Invalid\") } can't be found").await;
}

#[tokio::test]
async fn test_invalid_script_function_function_id() {
    let mut req = signing_message_request();
    req["payload"]["function"] = json!("invalid");

    assert_error_msg(req, "invalid script function id \"invalid\"").await;
}

#[tokio::test]
async fn test_invalid_payload_type() {
    let mut req = signing_message_request();
    req["payload"]["type"] = json!("invalid");

    assert_error_msg(req, "unknown variant `invalid`, expected one of `script_function_payload`, `script_payload`, `module_bundle_payload`, `write_set_payload`").await;
}

#[tokio::test]
async fn test_invalid_payload_data_type() {
    let mut req = signing_message_request();
    req["payload"] = json!(1234);

    assert_error_msg(
        req,
        "invalid type: integer `1234`, expected internally tagged enum TransactionPayload",
    )
    .await;
}

#[tokio::test]
async fn test_invalid_sender_address() {
    let mut req = signing_message_request();
    req["sender"] = json!("invalid");

    assert_error_msg(req, "invalid account address \"invalid\"").await;
}

#[tokio::test]
async fn test_invalid_sequence_number() {
    let mut req = signing_message_request();
    req["sequence_number"] = json!("invalid");
    assert_error_msg(req, "parse u64 string \"invalid\" failed").await;
}

async fn assert_error_msg(req: Value, msg: &str) {
    let err_msg = response_error_msg(req).await;
    assert!(err_msg.contains(msg), "expect {} contains {}", err_msg, msg);
}

async fn response_error_msg(req: Value) -> String {
    let context = new_test_context();
    let resp = context
        .expect_status_code(400)
        .post("/transactions/signing_message", req)
        .await;
    resp["message"].as_str().unwrap().to_owned()
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
            "function": "0x1::AccountCreationScripts::create_parent_vasp_account",
            "type_arguments": [
                "0x1::XUS::XUS"
            ],
            "arguments": [
                "0",     // sliding_nonce
                "0x11223344",  // new account address
                "0x5307b5f4bc67829097a8ba9b43dba3b88261eeccd1f709d9bde240fc100fbb69",  // auth key
                "0x68656c6c6f20776f726c64", // human name
                true, // add_all_currencies
            ]
        }
    })
}
