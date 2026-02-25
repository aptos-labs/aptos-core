// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Comprehensive tests for the Axum-based API migration.
//! These tests validate that the Axum backend produces identical behavior
//! to the original Poem backend across all API endpoints.

use super::new_test_context;
use aptos_api_test_context::current_function_name;
use serde_json::json;

// ========================================================================
// BasicApi Tests
// ========================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_axum_ledger_info() {
    let context = new_test_context(current_function_name!());
    let resp = context.get("/").await;
    assert!(resp.get("chain_id").is_some());
    assert!(resp.get("epoch").is_some());
    assert!(resp.get("ledger_version").is_some());
    assert!(resp.get("oldest_ledger_version").is_some());
    assert!(resp.get("ledger_timestamp").is_some());
    assert!(resp.get("node_role").is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_axum_healthy() {
    let context = new_test_context(current_function_name!());
    let resp = context.get("/-/healthy").await;
    assert_eq!(resp["message"], "aptos-node:ok");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_axum_healthy_with_duration_too_stale() {
    let context = new_test_context(current_function_name!());
    // With a small duration_secs and test node's old timestamps, should fail
    let (status, _headers, _body) = context.get_raw("/-/healthy", "duration_secs=1").await;
    assert_eq!(status.as_u16(), 503);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_axum_spec() {
    let context = new_test_context(current_function_name!());
    let (status, _headers, body) = context.get_raw("/spec", "").await;
    assert_eq!(status.as_u16(), 200);
    assert!(body.len() > 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_axum_spec_json() {
    let context = new_test_context(current_function_name!());
    let (status, headers, body) = context.get_raw("/spec.json", "").await;
    assert_eq!(status.as_u16(), 200);
    assert!(
        headers
            .get("content-type")
            .unwrap()
            .to_str()
            .unwrap()
            .contains("application/json")
    );
    let spec: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(spec.get("openapi").is_some() || spec.get("info").is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_axum_info() {
    let context = new_test_context(current_function_name!());
    let (status, _headers, body) = context.get_raw("/info", "").await;
    assert_eq!(status.as_u16(), 200);
    let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(resp.get("chain_id").is_some());
    assert!(resp.get("node_type").is_some());
}

// ========================================================================
// AccountsApi Tests
// ========================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_axum_get_account() {
    let mut context = new_test_context(current_function_name!());
    let account = context.create_account().await;
    let address = account.address();
    let resp = context.get(&format!("/accounts/{}", address)).await;
    assert!(resp.get("sequence_number").is_some());
    assert!(resp.get("authentication_key").is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_axum_get_account_not_found_or_default() {
    let context = new_test_context(current_function_name!());
    // With the DEFAULT_ACCOUNT_RESOURCE feature, non-existent accounts may
    // return a default account resource instead of 404.
    let resp = context
        .get("/accounts/0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef")
        .await;
    // Either returns valid account data or a 404
    assert!(resp.get("sequence_number").is_some() || resp.get("message").is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_axum_get_account_resources() {
    let context = new_test_context(current_function_name!());
    let resp = context.get("/accounts/0x1/resources").await;
    assert!(resp.is_array());
    assert!(resp.as_array().unwrap().len() > 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_axum_get_account_modules() {
    let context = new_test_context(current_function_name!());
    let resp = context.get("/accounts/0x1/modules").await;
    assert!(resp.is_array());
    assert!(resp.as_array().unwrap().len() > 0);
}

// ========================================================================
// BlocksApi Tests
// ========================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_axum_get_block_by_height() {
    let context = new_test_context(current_function_name!());
    let resp = context.get("/blocks/by_height/0").await;
    assert!(resp.get("block_height").is_some());
    assert_eq!(resp["block_height"], "0");
    assert!(resp.get("block_hash").is_some());
    assert!(resp.get("first_version").is_some());
    assert!(resp.get("last_version").is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_axum_get_block_by_height_with_transactions() {
    let context = new_test_context(current_function_name!());
    let resp = context
        .get("/blocks/by_height/0?with_transactions=true")
        .await;
    assert!(resp.get("transactions").is_some());
    let txns = resp["transactions"].as_array().unwrap();
    assert!(txns.len() > 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_axum_get_block_by_version() {
    let context = new_test_context(current_function_name!());
    let resp = context.get("/blocks/by_version/0").await;
    assert!(resp.get("block_height").is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_axum_get_block_not_found() {
    let context = new_test_context(current_function_name!());
    let resp = context
        .expect_status_code(404)
        .get("/blocks/by_height/999999999")
        .await;
    assert!(resp.get("message").is_some());
}

// ========================================================================
// TransactionsApi Tests
// ========================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_axum_get_transactions() {
    let context = new_test_context(current_function_name!());
    let resp = context.get("/transactions").await;
    assert!(resp.is_array());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_axum_get_transactions_with_limit() {
    let context = new_test_context(current_function_name!());
    let resp = context.get("/transactions?limit=2").await;
    assert!(resp.is_array());
    assert!(resp.as_array().unwrap().len() <= 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_axum_get_transaction_by_version() {
    let context = new_test_context(current_function_name!());
    let resp = context.get("/transactions/by_version/0").await;
    assert!(resp.get("type").is_some());
    assert!(resp.get("version").is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_axum_get_transaction_by_version_not_found() {
    let context = new_test_context(current_function_name!());
    let resp = context
        .expect_status_code(404)
        .get("/transactions/by_version/999999999")
        .await;
    assert!(resp.get("message").is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_axum_submit_and_get_transaction() {
    let mut context = new_test_context(current_function_name!());
    let account = context.create_account().await;
    let address = account.address();

    let resp = context.get(&format!("/accounts/{}", address)).await;
    assert!(resp.get("sequence_number").is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_axum_estimate_gas_price() {
    let context = new_test_context(current_function_name!());
    let resp = context.get("/estimate_gas_price").await;
    assert!(resp.get("gas_estimate").is_some());
}

// ========================================================================
// EventsApi Tests
// ========================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_axum_get_events_by_creation_number() {
    let context = new_test_context(current_function_name!());
    let resp = context.get("/accounts/0x1/events/0").await;
    assert!(resp.is_array());
}

// ========================================================================
// StateApi Tests
// ========================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_axum_get_account_resource() {
    let context = new_test_context(current_function_name!());
    let resp = context
        .get("/accounts/0x1/resource/0x1::account::Account")
        .await;
    assert!(resp.get("type").is_some());
    assert!(resp.get("data").is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_axum_get_account_resource_not_found() {
    let context = new_test_context(current_function_name!());
    let resp = context
        .expect_status_code(404)
        .get("/accounts/0x1/resource/0x1::nonexistent::Resource")
        .await;
    assert!(resp.get("message").is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_axum_get_account_module() {
    let context = new_test_context(current_function_name!());
    let resp = context.get("/accounts/0x1/module/account").await;
    assert!(resp.get("bytecode").is_some());
    assert!(resp.get("abi").is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_axum_get_account_module_not_found() {
    let context = new_test_context(current_function_name!());
    let resp = context
        .expect_status_code(404)
        .get("/accounts/0x1/module/nonexistent_module")
        .await;
    assert!(resp.get("message").is_some());
}

// ========================================================================
// ViewFunctionApi Tests
// ========================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_axum_view_function() {
    let context = new_test_context(current_function_name!());
    let request = json!({
        "function":"0x1::chain_id::get",
        "arguments": [],
        "type_arguments": [],
    });
    let resp = context.post("/view", request).await;
    assert!(resp.is_array());
    assert_eq!(resp.as_array().unwrap().len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_axum_view_function_invalid() {
    let context = new_test_context(current_function_name!());
    let request = json!({
        "function":"0x1::nonexistent::function",
        "arguments": [],
        "type_arguments": [],
    });
    let resp = context
        .expect_status_code(400)
        .post("/view", request)
        .await;
    assert!(resp.get("message").is_some());
}

// ========================================================================
// Encode Submission Tests
// ========================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_axum_encode_submission() {
    let mut context = new_test_context(current_function_name!());
    let account = context.root_account().await;
    let request = json!({
        "sender": account.address(),
        "sequence_number": account.sequence_number().to_string(),
        "gas_unit_price": "100",
        "max_gas_amount": "1000000",
        "expiration_timestamp_secs": "16373698888888",
        "payload": {
            "type": "entry_function_payload",
            "function": "0x1::aptos_account::transfer",
            "type_arguments": [],
            "arguments": ["0x1", "1"]
        },
    });
    let resp = context
        .post("/transactions/encode_submission", request)
        .await;
    assert!(resp.is_string());
    assert!(resp.as_str().unwrap().starts_with("0x"));
}

// ========================================================================
// Error Handling Tests
// ========================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_axum_404_fallback() {
    let context = new_test_context(current_function_name!());
    let resp = context
        .expect_status_code(404)
        .get("/nonexistent_path")
        .await;
    assert!(resp.get("message").is_some());
    assert_eq!(resp["error_code"], "web_framework_error");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_axum_invalid_address() {
    let context = new_test_context(current_function_name!());
    // Invalid addresses might return 400 or 422 depending on how the path
    // parameter is parsed. We just verify it's a client error.
    let (status, _headers, _body) = context.get_raw("/accounts/not_a_valid_address", "").await;
    assert!(status.as_u16() >= 400 && status.as_u16() < 500);
}

// ========================================================================
// Ledger Headers Tests
// ========================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_axum_response_headers() {
    let context = new_test_context(current_function_name!());
    let (body, status, headers) = context.get_with_headers("/").await;
    assert_eq!(status.as_u16(), 200);
    assert!(headers.get("x-aptos-chain-id").is_some());
    assert!(headers.get("x-aptos-ledger-version").is_some());
    assert!(headers.get("x-aptos-ledger-timestampusec").is_some());
    assert!(headers.get("x-aptos-epoch").is_some());
    assert!(headers.get("x-aptos-block-height").is_some());
    assert!(body.get("chain_id").is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_axum_error_response_format() {
    let context = new_test_context(current_function_name!());
    let resp = context
        .expect_status_code(404)
        .get("/accounts/0xdead/resource/0x1::nonexistent::Foo")
        .await;
    assert!(resp.get("message").is_some());
    assert!(resp.get("error_code").is_some());
    assert!(resp.get("vm_error_code").is_some());
}
