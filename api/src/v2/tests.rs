// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Integration tests for the v2 API.
//!
//! These tests spin up a real v2 Axum server on a random port with a real DB
//! and mempool, then make HTTP requests using reqwest.

use super::context::{V2Config, V2Context};
use super::router::{build_combined_router, build_v2_router};
use crate::context::Context;
use aptos_api_test_context::new_test_context as create_test_context;
use aptos_config::config::NodeConfig;
use aptos_types::chain_id::ChainId;
use std::net::SocketAddr;

/// Helper: build a v2 server from a fresh test context.
///
/// To avoid the "two versions of aptos_api" problem (aptos-api dev-depends
/// on aptos-api-test-context, which depends on aptos-api), we extract the
/// DB, mempool sender, and indexer reader from the TestContext (these types
/// are from non-duplicated crates) and construct a fresh `crate::context::Context`.
async fn start_v2_server() -> (SocketAddr, tokio::task::JoinHandle<()>) {
    // Disable storage sharding so that the direct DB path is used for
    // prefix iteration (otherwise the indexer reader is required).
    let mut node_config = NodeConfig::default();
    node_config.storage.rocksdb_configs.enable_storage_sharding = false;

    let test_ctx = create_test_context("v2_test".to_string(), node_config.clone(), false);

    // Build a crate-local Context from the test context's components.
    // All component types (AptosDB, MempoolClientSender, etc.) are from
    // crates that appear only once in the dependency graph.
    let context = Context::new(
        ChainId::test(),
        test_ctx.db.clone(),
        test_ctx.mempool.ac_client.clone(),
        node_config.clone(),
        None, // indexer_reader: not needed when sharding is disabled
    );

    let v2_config = V2Config::from_configs(&node_config.api_v2, &node_config.api);
    let v2_ctx = V2Context::new(context, v2_config);
    let router = build_v2_router(v2_ctx);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind");
    let addr = listener.local_addr().unwrap();

    let handle = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    (addr, handle)
}

fn base_url(addr: SocketAddr) -> String {
    format!("http://{}", addr)
}

// ---- Health & Info ----

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_health() {
    let (addr, _handle) = start_v2_server().await;

    let resp = reqwest::get(format!("{}/v2/health", base_url(addr)))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    assert!(body["ledger"]["chain_id"].is_number());
    assert!(body["ledger"]["ledger_version"].is_number());
    assert!(body["ledger"]["epoch"].is_number());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_info() {
    let (addr, _handle) = start_v2_server().await;

    let resp = reqwest::get(format!("{}/v2/info", base_url(addr)))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["data"]["chain_id"].is_number());
    assert_eq!(body["data"]["api_version"], "2.0.0");
    assert!(body["ledger"]["ledger_version"].is_number());
}

// ---- Resources ----

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_get_resources() {
    let (addr, _handle) = start_v2_server().await;

    // 0x1 is the framework account which always has resources
    let resp = reqwest::get(format!(
        "{}/v2/accounts/0x1/resources",
        base_url(addr)
    ))
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["data"].is_array());
    assert!(!body["data"].as_array().unwrap().is_empty());
    assert!(body["ledger"]["ledger_version"].is_number());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_get_single_resource() {
    let (addr, _handle) = start_v2_server().await;

    let resp = reqwest::get(format!(
        "{}/v2/accounts/0x1/resource/0x1::account::Account",
        base_url(addr)
    ))
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["data"]["type"].is_string());
    assert!(body["ledger"]["ledger_version"].is_number());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_resource_not_found() {
    let (addr, _handle) = start_v2_server().await;

    let resp = reqwest::get(format!(
        "{}/v2/accounts/0x1/resource/0x1::nonexistent::DoesNotExist",
        base_url(addr)
    ))
    .await
    .unwrap();
    assert_eq!(resp.status(), 404);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "RESOURCE_NOT_FOUND");
}

// ---- Modules ----

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_get_modules() {
    let (addr, _handle) = start_v2_server().await;

    let resp = reqwest::get(format!(
        "{}/v2/accounts/0x1/modules",
        base_url(addr)
    ))
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["data"].is_array());
    assert!(!body["data"].as_array().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_get_single_module() {
    let (addr, _handle) = start_v2_server().await;

    let resp = reqwest::get(format!(
        "{}/v2/accounts/0x1/module/account",
        base_url(addr)
    ))
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["data"]["bytecode"].is_string());
    assert!(body["data"]["abi"].is_object());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_module_not_found() {
    let (addr, _handle) = start_v2_server().await;

    let resp = reqwest::get(format!(
        "{}/v2/accounts/0x1/module/nonexistent_module",
        base_url(addr)
    ))
    .await
    .unwrap();
    assert_eq!(resp.status(), 404);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "MODULE_NOT_FOUND");
}

// ---- Transactions ----

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_list_transactions() {
    let (addr, _handle) = start_v2_server().await;

    let resp = reqwest::get(format!("{}/v2/transactions", base_url(addr)))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["data"].is_array());
    // Genesis creates transactions
    assert!(!body["data"].as_array().unwrap().is_empty());
    assert!(body["ledger"]["ledger_version"].is_number());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_transaction_not_found() {
    let (addr, _handle) = start_v2_server().await;

    // Zero hash that shouldn't exist
    let resp = reqwest::get(format!(
        "{}/v2/transactions/0x0000000000000000000000000000000000000000000000000000000000000000",
        base_url(addr)
    ))
    .await
    .unwrap();
    assert_eq!(resp.status(), 404);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "TRANSACTION_NOT_FOUND");
}

// ---- Blocks ----

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_get_latest_block() {
    let (addr, _handle) = start_v2_server().await;

    let resp = reqwest::get(format!("{}/v2/blocks/latest", base_url(addr)))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["data"]["block_height"].is_string());
    assert!(body["data"]["block_hash"].is_string());
    assert!(body["data"]["block_timestamp"].is_string());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_get_block_by_height() {
    let (addr, _handle) = start_v2_server().await;

    let resp = reqwest::get(format!("{}/v2/blocks/0", base_url(addr)))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["data"]["block_height"].is_string());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_block_not_found() {
    let (addr, _handle) = start_v2_server().await;

    let resp = reqwest::get(format!("{}/v2/blocks/999999999", base_url(addr)))
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "BLOCK_NOT_FOUND");
}

// ---- Batch (JSON-RPC 2.0) ----

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_batch_single_request() {
    let (addr, _handle) = start_v2_server().await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/v2/batch", base_url(addr)))
        .json(&serde_json::json!([
            {
                "jsonrpc": "2.0",
                "method": "get_ledger_info",
                "params": {},
                "id": 1
            }
        ]))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(body.len(), 1);
    assert_eq!(body[0]["jsonrpc"], "2.0");
    assert_eq!(body[0]["id"], 1);
    assert!(body[0]["result"]["chain_id"].is_number());
    assert!(body[0]["error"].is_null());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_batch_multiple_requests() {
    let (addr, _handle) = start_v2_server().await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/v2/batch", base_url(addr)))
        .json(&serde_json::json!([
            {
                "jsonrpc": "2.0",
                "method": "get_ledger_info",
                "params": {},
                "id": 1
            },
            {
                "jsonrpc": "2.0",
                "method": "get_resources",
                "params": {"address": "0x1"},
                "id": 2
            },
            {
                "jsonrpc": "2.0",
                "method": "unknown_method",
                "params": {},
                "id": 3
            }
        ]))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(body.len(), 3);

    // First: ledger_info should succeed
    assert!(body[0]["result"].is_object());

    // Second: resources should succeed
    assert!(body[1]["result"]["data"].is_array());

    // Third: unknown method should fail with JSON-RPC error
    assert!(body[2]["error"].is_object());
    assert_eq!(body[2]["error"]["code"], -32600);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_batch_empty_request() {
    let (addr, _handle) = start_v2_server().await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/v2/batch", base_url(addr)))
        .json(&serde_json::json!([]))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "INVALID_INPUT");
}

// ---- Middleware ----

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_request_id_header() {
    let (addr, _handle) = start_v2_server().await;

    let resp = reqwest::get(format!("{}/v2/health", base_url(addr)))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let request_id = resp.headers().get("x-request-id");
    assert!(request_id.is_some(), "x-request-id header should be present");
    assert!(!request_id.unwrap().to_str().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_cors_headers() {
    let (addr, _handle) = start_v2_server().await;

    let client = reqwest::Client::new();
    let resp = client
        .request(
            reqwest::Method::OPTIONS,
            format!("{}/v2/health", base_url(addr)),
        )
        .header("Origin", "http://example.com")
        .header("Access-Control-Request-Method", "GET")
        .send()
        .await
        .unwrap();

    let cors_origin = resp.headers().get("access-control-allow-origin");
    assert!(
        cors_origin.is_some(),
        "CORS allow-origin header should be present"
    );
}

// ---- Error format ----

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_error_format_no_ledger_metadata() {
    let (addr, _handle) = start_v2_server().await;

    let resp = reqwest::get(format!(
        "{}/v2/accounts/0x1/resource/0x1::nonexistent::Type",
        base_url(addr)
    ))
    .await
    .unwrap();
    assert_eq!(resp.status(), 404);

    let body: serde_json::Value = resp.json().await.unwrap();
    // Error should have code and message
    assert!(body["code"].is_string());
    assert!(body["message"].is_string());
    // Error should NOT have ledger metadata (key design decision)
    assert!(body.get("ledger").is_none());
}

// ---- Pagination ----

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_pagination_cursor_follows() {
    let (addr, _handle) = start_v2_server().await;

    // 0x1 has many resources; check pagination works
    let resp = reqwest::get(format!(
        "{}/v2/accounts/0x1/resources",
        base_url(addr)
    ))
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    let data = body["data"].as_array().unwrap();

    // If there's a cursor, follow it
    if let Some(cursor) = body.get("cursor").and_then(|c| c.as_str()) {
        assert!(!cursor.is_empty());
        assert!(!data.is_empty());

        let resp2 = reqwest::get(format!(
            "{}/v2/accounts/0x1/resources?cursor={}",
            base_url(addr),
            cursor
        ))
        .await
        .unwrap();
        assert_eq!(resp2.status(), 200);

        let body2: serde_json::Value = resp2.json().await.unwrap();
        assert!(body2["data"].is_array());
    }
}

// ---- View function ----

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_view_function() {
    let (addr, _handle) = start_v2_server().await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/v2/view", base_url(addr)))
        .json(&serde_json::json!({
            "function": "0x1::account::exists_at",
            "type_arguments": [],
            "arguments": ["0x1"]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["data"].is_array());
    // exists_at returns a bool
    assert_eq!(body["data"][0], true);
}

// ---- Invalid address ----

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_invalid_address() {
    let (addr, _handle) = start_v2_server().await;

    let resp = reqwest::get(format!(
        "{}/v2/accounts/not_a_valid_address/resources",
        base_url(addr)
    ))
    .await
    .unwrap();
    assert_eq!(resp.status(), 400);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "INVALID_INPUT");
}

// ---- Response structure consistency ----

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_response_envelope_structure() {
    let (addr, _handle) = start_v2_server().await;

    // Test multiple endpoints for consistent V2Response envelope
    let endpoints = [
        format!("{}/v2/info", base_url(addr)),
        format!("{}/v2/accounts/0x1/resources", base_url(addr)),
        format!("{}/v2/accounts/0x1/modules", base_url(addr)),
        format!("{}/v2/transactions", base_url(addr)),
        format!("{}/v2/blocks/latest", base_url(addr)),
    ];

    for url in &endpoints {
        let resp = reqwest::get(url).await.unwrap();
        assert_eq!(resp.status(), 200, "Failed for endpoint: {}", url);

        let body: serde_json::Value = resp.json().await.unwrap();
        assert!(
            body.get("data").is_some(),
            "Missing 'data' field in response from {}",
            url
        );
        assert!(
            body.get("ledger").is_some(),
            "Missing 'ledger' field in response from {}",
            url
        );
    }
}

// ---- Ledger version query parameter ----

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_resources_at_version() {
    let (addr, _handle) = start_v2_server().await;

    // First get the current version
    let resp = reqwest::get(format!("{}/v2/health", base_url(addr)))
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    let version = body["ledger"]["ledger_version"].as_u64().unwrap();

    // Query resources at that specific version
    let resp = reqwest::get(format!(
        "{}/v2/accounts/0x1/resources?ledger_version={}",
        base_url(addr),
        version
    ))
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["data"].is_array());
}

// ==== Same-port co-hosting tests ====

/// Helper: start a combined server (v2 + v1 proxy) for co-hosting tests.
///
/// Starts a Poem v1 server on an internal random port, then starts a combined
/// Axum server that serves v2 directly and proxies everything else to Poem.
async fn start_combined_server() -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let mut node_config = NodeConfig::default();
    node_config.storage.rocksdb_configs.enable_storage_sharding = false;

    let test_ctx = create_test_context("v2_cohost_test".to_string(), node_config.clone(), false);

    let context = Context::new(
        ChainId::test(),
        test_ctx.db.clone(),
        test_ctx.mempool.ac_client.clone(),
        node_config.clone(),
        None,
    );

    // Start Poem v1 on internal port (simulates what runtime.rs does).
    let poem_context = context.clone();
    let poem_config = node_config.clone();
    let poem_addr = crate::runtime::attach_poem_to_runtime(
        &tokio::runtime::Handle::current(),
        poem_context,
        &poem_config,
        true, // random_port
        None,
    )
    .expect("Failed to start internal Poem");

    // Build combined router (v2 + v1 proxy).
    let v2_config = V2Config::from_configs(&node_config.api_v2, &node_config.api);
    let v2_ctx = V2Context::new(context, v2_config);
    let combined = build_combined_router(v2_ctx, poem_addr);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind combined server");
    let addr = listener.local_addr().unwrap();

    let handle = tokio::spawn(async move {
        axum::serve(listener, combined).await.unwrap();
    });

    // Give the server a moment to start.
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    (addr, handle)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cohost_v2_health() {
    let (addr, _handle) = start_combined_server().await;

    let resp = reqwest::get(format!("{}/v2/health", base_url(addr)))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cohost_v1_proxy_index() {
    let (addr, _handle) = start_combined_server().await;

    // Request to root "/" should be proxied to Poem v1
    let resp = reqwest::get(format!("{}/", base_url(addr)))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body = resp.text().await.unwrap();
    assert!(body.contains("Aptos Node API"), "Root page should contain 'Aptos Node API'");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cohost_v1_proxy_ledger_info() {
    let (addr, _handle) = start_combined_server().await;

    // GET /v1/ (ledger info) should be proxied to Poem
    let resp = reqwest::get(format!("{}/v1", base_url(addr)))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["chain_id"].is_number(), "v1 ledger info should have chain_id");
    assert!(body["ledger_version"].is_string(), "v1 ledger info should have ledger_version");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cohost_v1_proxy_resources() {
    let (addr, _handle) = start_combined_server().await;

    // GET /v1/accounts/0x1/resources should be proxied to Poem
    let resp = reqwest::get(format!("{}/v1/accounts/0x1/resources", base_url(addr)))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body.is_array(), "v1 resources should return an array");
    assert!(!body.as_array().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cohost_v1_proxy_health() {
    let (addr, _handle) = start_combined_server().await;

    // GET /v1/-/healthy should be proxied to Poem
    let resp = reqwest::get(format!("{}/v1/-/healthy", base_url(addr)))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cohost_both_versions_on_same_port() {
    let (addr, _handle) = start_combined_server().await;

    // v2 health endpoint (served by Axum directly)
    let v2_resp = reqwest::get(format!("{}/v2/health", base_url(addr)))
        .await
        .unwrap();
    assert_eq!(v2_resp.status(), 200);
    let v2_body: serde_json::Value = v2_resp.json().await.unwrap();
    assert_eq!(v2_body["status"], "ok");

    // v1 ledger info (proxied to Poem)
    let v1_resp = reqwest::get(format!("{}/v1", base_url(addr)))
        .await
        .unwrap();
    assert_eq!(v1_resp.status(), 200);
    let v1_body: serde_json::Value = v1_resp.json().await.unwrap();
    assert!(v1_body["chain_id"].is_number());

    // Both should report the same chain ID
    let v2_chain_id = v2_body["ledger"]["chain_id"].as_u64().unwrap();
    let v1_chain_id = v1_body["chain_id"].as_u64().unwrap();
    assert_eq!(v2_chain_id, v1_chain_id, "Both APIs should report the same chain_id");
}
