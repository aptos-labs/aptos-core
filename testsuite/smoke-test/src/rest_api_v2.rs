// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Smoke tests for the v2 REST API running against a real local node.
//!
//! Unlike the integration tests in `api/src/v2/tests.rs` (which use a mock DB
//! and mempool), these tests spin up a full `LocalSwarm` with a real validator
//! node and exercise the v2 endpoints end-to-end over HTTP.

use crate::smoke_test_environment::SwarmBuilder;
use aptos_cached_packages::aptos_stdlib;
use aptos_forge::Swarm;
use std::sync::Arc;

/// Helper: start a 1-validator swarm with the v2 API enabled in co-hosting
/// mode (v2 shares the same port as v1). Returns the swarm.
async fn swarm_with_v2() -> Box<dyn Swarm> {
    Box::new(
        SwarmBuilder::new_local(1)
            .with_aptos()
            .with_init_config(Arc::new(|_, conf, _| {
                conf.api_v2.enabled = true;
                // Co-host mode: v2 shares the v1 port (address = None).
            }))
            .build()
            .await,
    )
}

/// Helper: get the base URL for making requests (e.g. `http://127.0.0.1:8080`).
fn base_url(swarm: &dyn Swarm) -> String {
    swarm.aptos_public_info().url().trim_end_matches('/').to_string()
}

// ======================================================================
// Health & info
// ======================================================================

#[tokio::test]
async fn test_v2_health() {
    let swarm = swarm_with_v2().await;
    let url = format!("{}/v2/health", base_url(&*swarm));

    let resp = reqwest::get(&url).await.unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn test_v2_info() {
    let swarm = swarm_with_v2().await;
    let url = format!("{}/v2/info", base_url(&*swarm));

    let resp = reqwest::get(&url).await.unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    // The v2 info endpoint returns a V2Response envelope with ledger metadata.
    assert!(body["ledger"].is_object(), "Should have ledger metadata");
    assert!(
        body["ledger"]["chain_id"].is_number(),
        "Should have chain_id in ledger metadata"
    );
    assert!(
        body["ledger"]["ledger_version"].is_number(),
        "Should have ledger_version"
    );
    assert!(
        body["data"].is_object(),
        "Should have data object with node info"
    );
}

// ======================================================================
// Co-hosting: v1 and v2 on the same port
// ======================================================================

#[tokio::test]
async fn test_v2_cohost_v1_still_works() {
    let swarm = swarm_with_v2().await;
    let base = base_url(&*swarm);

    // v2 health should work.
    let v2_resp = reqwest::get(format!("{}/v2/health", base))
        .await
        .unwrap();
    assert_eq!(v2_resp.status(), 200);

    // v1 ledger info should also work (proxied from Axum to Poem).
    let v1_resp = reqwest::get(format!("{}/v1", base)).await.unwrap();
    assert_eq!(v1_resp.status(), 200);

    let v1_body: serde_json::Value = v1_resp.json().await.unwrap();
    assert!(
        v1_body["chain_id"].is_number(),
        "v1 ledger info should have chain_id"
    );
}

// ======================================================================
// Accounts & resources
// ======================================================================

#[tokio::test]
async fn test_v2_accounts_resources() {
    let swarm = swarm_with_v2().await;
    let base = base_url(&*swarm);

    // 0x1 (framework) should always have resources.
    let url = format!("{}/v2/accounts/0x1/resources", base);
    let resp = reqwest::get(&url).await.unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["data"].is_array(), "Resources should be an array");
    assert!(
        !body["data"].as_array().unwrap().is_empty(),
        "0x1 should have resources"
    );
    assert!(body["ledger"].is_object(), "Should have ledger metadata");
}

#[tokio::test]
async fn test_v2_account_info() {
    let swarm = swarm_with_v2().await;
    let base = base_url(&*swarm);

    let url = format!("{}/v2/accounts/0x1", base);
    let resp = reqwest::get(&url).await.unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(
        body["data"]["sequence_number"].is_number()
            || body["data"]["sequence_number"].is_string(),
        "Account info should have sequence_number, got: {}",
        body
    );
}

#[tokio::test]
async fn test_v2_account_not_found() {
    let swarm = swarm_with_v2().await;
    let base = base_url(&*swarm);

    // An address with no account should 404.
    let url = format!(
        "{}/v2/accounts/0x{:064x}",
        base, 0xdeadbeef_u64
    );
    let resp = reqwest::get(&url).await.unwrap();
    assert_eq!(resp.status(), 404);
}

// ======================================================================
// Modules
// ======================================================================

#[tokio::test]
async fn test_v2_modules() {
    let swarm = swarm_with_v2().await;
    let base = base_url(&*swarm);

    let url = format!("{}/v2/accounts/0x1/modules", base);
    let resp = reqwest::get(&url).await.unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["data"].is_array(), "Modules should be an array");
    assert!(
        !body["data"].as_array().unwrap().is_empty(),
        "0x1 should have modules"
    );
}

// ======================================================================
// Transactions
// ======================================================================

#[tokio::test]
async fn test_v2_transactions_list() {
    let swarm = swarm_with_v2().await;
    let base = base_url(&*swarm);

    let url = format!("{}/v2/transactions", base);
    let resp = reqwest::get(&url).await.unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["data"].is_array(), "Transactions should be an array");
    // Genesis + block metadata transactions should exist.
    assert!(
        !body["data"].as_array().unwrap().is_empty(),
        "Should have at least genesis transactions"
    );
}

#[tokio::test]
async fn test_v2_submit_and_retrieve_transaction() {
    let swarm = swarm_with_v2().await;
    let mut info = swarm.aptos_public_info();
    let base = base_url(&*swarm);

    // Create and fund accounts via v1 (proven infrastructure).
    let account1 = info
        .create_and_fund_user_account(10_000_000_000)
        .await
        .unwrap();
    let account2 = info
        .create_and_fund_user_account(10_000_000_000)
        .await
        .unwrap();

    // Submit a transfer via v1.
    let txn = account1.sign_with_transaction_builder(
        info.transaction_factory()
            .payload(aptos_stdlib::aptos_coin_transfer(account2.address(), 1_000)),
    );
    let pending = info.client().submit(&txn).await.unwrap().into_inner();
    info.client()
        .wait_for_transaction(&pending)
        .await
        .unwrap();

    // Retrieve the transaction via v2 by hash (use the hash from the
    // pending transaction response, which is always available).
    let hash = pending.hash.to_string();
    let url = format!("{}/v2/transactions/{}", base, hash);
    let resp = reqwest::get(&url).await.unwrap();
    assert_eq!(
        resp.status(),
        200,
        "v2 should find committed transaction by hash"
    );

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["ledger"].is_object(), "Should have ledger metadata");
}

// ======================================================================
// Blocks
// ======================================================================

#[tokio::test]
async fn test_v2_blocks_latest() {
    let swarm = swarm_with_v2().await;
    let base = base_url(&*swarm);

    let url = format!("{}/v2/blocks/latest", base);
    let resp = reqwest::get(&url).await.unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["data"].is_object(), "Block should be an object");
    assert!(
        body["data"]["block_height"].is_number() || body["data"]["block_height"].is_string(),
        "Block should have block_height, got: {}",
        body
    );
}

#[tokio::test]
async fn test_v2_blocks_by_height() {
    let swarm = swarm_with_v2().await;
    let base = base_url(&*swarm);

    // Block 0 (genesis) should always exist.
    let url = format!("{}/v2/blocks/0", base);
    let resp = reqwest::get(&url).await.unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["data"].is_object(), "Block should be an object");
}

// ======================================================================
// View function
// ======================================================================

#[tokio::test]
async fn test_v2_view_function() {
    let swarm = swarm_with_v2().await;
    let base = base_url(&*swarm);

    let url = format!("{}/v2/view", base);

    // Call 0x1::chain_id::get() which takes no args and returns u8.
    let body = serde_json::json!({
        "function": "0x1::chain_id::get",
        "type_arguments": [],
        "arguments": []
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp_body: serde_json::Value = resp.json().await.unwrap();
    // View function returns an array of return values.
    assert!(
        resp_body["data"].is_array(),
        "View should return data array, got: {}",
        resp_body
    );
}

// ======================================================================
// Gas estimation
// ======================================================================

#[tokio::test]
async fn test_v2_gas_estimation() {
    let swarm = swarm_with_v2().await;
    let base = base_url(&*swarm);

    let url = format!("{}/v2/estimate_gas_price", base);
    let resp = reqwest::get(&url).await.unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(
        body["data"].is_object(),
        "Gas estimation should return a data object, got: {}",
        body
    );
}

// ======================================================================
// OpenAPI spec
// ======================================================================

#[tokio::test]
async fn test_v2_openapi_spec() {
    let swarm = swarm_with_v2().await;
    let base = base_url(&*swarm);

    let url = format!("{}/v2/spec.json", base);
    let resp = reqwest::get(&url).await.unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(
        body["openapi"].is_string(),
        "Should be a valid OpenAPI spec"
    );
    assert!(
        body["paths"].is_object(),
        "Should have paths"
    );
}

// ======================================================================
// Balance
// ======================================================================

#[tokio::test]
async fn test_v2_balance() {
    let swarm = swarm_with_v2().await;
    let mut info = swarm.aptos_public_info();
    let base = base_url(&*swarm);

    // Create a funded account.
    let account = info
        .create_and_fund_user_account(10_000_000_000)
        .await
        .unwrap();

    let url = format!(
        "{}/v2/accounts/{}/balance/0x1::aptos_coin::AptosCoin",
        base,
        account.address()
    );
    let resp = reqwest::get(&url).await.unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(
        body["data"].is_object(),
        "Balance should return data, got: {}",
        body
    );
}

// ======================================================================
// JSON-RPC batch
// ======================================================================

#[tokio::test]
async fn test_v2_batch_request() {
    let swarm = swarm_with_v2().await;
    let base = base_url(&*swarm);

    let url = format!("{}/v2/batch", base);
    let batch = serde_json::json!([
        {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "get_ledger_info",
            "params": {}
        },
        {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "get_resources",
            "params": { "address": "0x1" }
        }
    ]);

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .json(&batch)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body.is_array(), "Batch should return array");
    assert_eq!(
        body.as_array().unwrap().len(),
        2,
        "Should have 2 responses"
    );
    // Both should succeed.
    for item in body.as_array().unwrap() {
        assert!(
            item["result"].is_object() || item["result"].is_array(),
            "Each batch item should have a result, got: {}",
            item
        );
    }
}
