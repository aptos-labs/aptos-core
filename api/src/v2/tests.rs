// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Integration tests for the v2 API.
//!
//! These tests spin up a real v2 Axum server on a random port with a real DB
//! and mempool, then make HTTP requests using reqwest.

use super::context::{V2Config, V2Context};
use super::router::{build_combined_router, build_v2_router};
use crate::context::Context;
use aptos_api_test_context::{
    new_test_context as create_test_context, new_test_context_no_api, TestContext,
};
use aptos_config::config::NodeConfig;
use aptos_types::chain_id::ChainId;
use std::net::SocketAddr;
use std::time::Duration;

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

    // Use new_test_context_no_api to avoid starting a v1 Poem server, which
    // prevents the circular-dep Prometheus AlreadyReg panic in parallel tests.
    let test_ctx = new_test_context_no_api("v2_test".to_string(), node_config.clone(), false);

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

/// Helper: build a v2 server with custom V2Config overrides.
///
/// The `config_fn` closure receives a mutable reference to the V2Config
/// and can modify it before the server starts.
async fn start_v2_server_with_config(
    config_fn: impl FnOnce(&mut V2Config),
) -> (SocketAddr, V2Context, tokio::task::JoinHandle<()>) {
    let mut node_config = NodeConfig::default();
    node_config.storage.rocksdb_configs.enable_storage_sharding = false;

    let test_ctx =
        new_test_context_no_api("v2_test_custom".to_string(), node_config.clone(), false);

    let context = Context::new(
        ChainId::test(),
        test_ctx.db.clone(),
        test_ctx.mempool.ac_client.clone(),
        node_config.clone(),
        None,
    );

    let mut v2_config = V2Config::from_configs(&node_config.api_v2, &node_config.api);
    config_fn(&mut v2_config);
    let v2_ctx = V2Context::new(context, v2_config);
    let router = build_v2_router(v2_ctx.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind");
    let addr = listener.local_addr().unwrap();

    let shutdown_rx = v2_ctx.shutdown_receiver();
    let handle = tokio::spawn(async move {
        let mut rx1 = shutdown_rx.clone();
        let mut rx2 = shutdown_rx;
        let server = axum::serve(listener, router).with_graceful_shutdown(async move {
            let _ = rx1.wait_for(|&v| v).await;
        });
        tokio::select! {
            result = server => { result.unwrap(); }
            _ = async {
                let _ = rx2.wait_for(|&v| v).await;
                tokio::time::sleep(Duration::from_secs(30)).await;
            } => {}
        }
    });

    (addr, v2_ctx, handle)
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
    let resp = reqwest::get(format!("{}/v2/accounts/0x1/resources", base_url(addr)))
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

    let resp = reqwest::get(format!("{}/v2/accounts/0x1/modules", base_url(addr)))
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

    let resp = reqwest::get(format!("{}/v2/accounts/0x1/module/account", base_url(addr)))
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
    assert!(
        request_id.is_some(),
        "x-request-id header should be present"
    );
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
    let resp = reqwest::get(format!("{}/v2/accounts/0x1/resources", base_url(addr)))
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

    // Use new_test_context_no_api to skip the Poem server that TestContext
    // would otherwise start via the aptos-api-test-context copy. We start
    // our own Poem server below using crate::runtime::attach_poem_to_runtime
    // (the test binary's own copy) to avoid duplicate metric registration.
    let test_ctx =
        new_test_context_no_api("v2_cohost_test".to_string(), node_config.clone(), false);

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
    let resp = reqwest::get(format!("{}/", base_url(addr))).await.unwrap();
    assert_eq!(resp.status(), 200);

    let body = resp.text().await.unwrap();
    assert!(
        body.contains("Aptos Node API"),
        "Root page should contain 'Aptos Node API'"
    );
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
    assert!(
        body["chain_id"].is_number(),
        "v1 ledger info should have chain_id"
    );
    assert!(
        body["ledger_version"].is_string(),
        "v1 ledger info should have ledger_version"
    );
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
    assert_eq!(
        v2_chain_id, v1_chain_id,
        "Both APIs should report the same chain_id"
    );
}

// ======================================================================
// WebSocket tests
// ======================================================================

use tokio_tungstenite::tungstenite;

/// Helper: connect to the v2 WebSocket endpoint.
async fn ws_connect(
    addr: SocketAddr,
) -> tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>> {
    let url = format!("ws://127.0.0.1:{}/v2/ws", addr.port());
    let (ws_stream, _) = tokio_tungstenite::connect_async(&url)
        .await
        .expect("Failed to connect WebSocket");
    ws_stream
}

/// Helper: send a JSON message and receive a JSON response.
async fn ws_send_recv(
    ws: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    msg: serde_json::Value,
) -> serde_json::Value {
    use futures::{SinkExt, StreamExt};
    ws.send(tungstenite::Message::Text(msg.to_string()))
        .await
        .expect("Failed to send WS message");

    // Read with timeout.
    let resp = tokio::time::timeout(std::time::Duration::from_secs(5), ws.next())
        .await
        .expect("Timed out waiting for WS response")
        .expect("WS stream ended")
        .expect("WS read error");

    match resp {
        tungstenite::Message::Text(text) => {
            serde_json::from_str(&text).expect("Invalid JSON from server")
        },
        other => panic!("Expected text message, got: {:?}", other),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_ws_ping_pong() {
    let (addr, _handle) = start_v2_server().await;
    let mut ws = ws_connect(addr).await;

    let resp = ws_send_recv(
        &mut ws,
        serde_json::json!({
            "action": "ping",
            "nonce": 42
        }),
    )
    .await;

    assert_eq!(resp["type"], "pong");
    assert_eq!(resp["nonce"], 42);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_ws_subscribe_new_blocks() {
    let (addr, _handle) = start_v2_server().await;
    let mut ws = ws_connect(addr).await;

    let resp = ws_send_recv(
        &mut ws,
        serde_json::json!({
            "action": "subscribe",
            "type": "new_blocks"
        }),
    )
    .await;

    assert_eq!(resp["type"], "subscribed");
    assert!(resp["id"].is_string(), "Should return a subscription ID");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_ws_subscribe_with_custom_id() {
    let (addr, _handle) = start_v2_server().await;
    let mut ws = ws_connect(addr).await;

    let resp = ws_send_recv(
        &mut ws,
        serde_json::json!({
            "action": "subscribe",
            "id": "my_blocks",
            "type": "new_blocks"
        }),
    )
    .await;

    assert_eq!(resp["type"], "subscribed");
    assert_eq!(resp["id"], "my_blocks");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_ws_unsubscribe() {
    let (addr, _handle) = start_v2_server().await;
    let mut ws = ws_connect(addr).await;

    // Subscribe first.
    let sub_resp = ws_send_recv(
        &mut ws,
        serde_json::json!({
            "action": "subscribe",
            "id": "to_remove",
            "type": "new_blocks"
        }),
    )
    .await;
    assert_eq!(sub_resp["type"], "subscribed");

    // Unsubscribe.
    let unsub_resp = ws_send_recv(
        &mut ws,
        serde_json::json!({
            "action": "unsubscribe",
            "id": "to_remove"
        }),
    )
    .await;
    assert_eq!(unsub_resp["type"], "unsubscribed");
    assert_eq!(unsub_resp["id"], "to_remove");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_ws_unsubscribe_unknown() {
    let (addr, _handle) = start_v2_server().await;
    let mut ws = ws_connect(addr).await;

    let resp = ws_send_recv(
        &mut ws,
        serde_json::json!({
            "action": "unsubscribe",
            "id": "nonexistent"
        }),
    )
    .await;

    assert_eq!(resp["type"], "error");
    assert_eq!(resp["code"], "UNKNOWN_SUBSCRIPTION");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_ws_invalid_message() {
    let (addr, _handle) = start_v2_server().await;
    let mut ws = ws_connect(addr).await;

    // Send something that doesn't parse as WsClientMessage.
    let resp = ws_send_recv(&mut ws, serde_json::json!({"bogus": true})).await;

    assert_eq!(resp["type"], "error");
    assert_eq!(resp["code"], "INVALID_MESSAGE");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_ws_subscribe_events_with_filter() {
    let (addr, _handle) = start_v2_server().await;
    let mut ws = ws_connect(addr).await;

    let resp = ws_send_recv(
        &mut ws,
        serde_json::json!({
            "action": "subscribe",
            "type": "events",
            "event_type": "0x1::coin::DepositEvent"
        }),
    )
    .await;

    assert_eq!(resp["type"], "subscribed");
}

// ---- Advanced event filter subscription tests ----

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_ws_subscribe_events_with_multiple_types() {
    let (addr, _handle) = start_v2_server().await;
    let mut ws = ws_connect(addr).await;

    let resp = ws_send_recv(
        &mut ws,
        serde_json::json!({
            "action": "subscribe",
            "type": "events",
            "event_types": ["0x1::coin::DepositEvent", "0x1::coin::WithdrawEvent"]
        }),
    )
    .await;

    assert_eq!(resp["type"], "subscribed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_ws_subscribe_events_with_wildcard() {
    let (addr, _handle) = start_v2_server().await;
    let mut ws = ws_connect(addr).await;

    // Module wildcard
    let resp = ws_send_recv(
        &mut ws,
        serde_json::json!({
            "action": "subscribe",
            "type": "events",
            "event_type": "0x1::coin::*"
        }),
    )
    .await;

    assert_eq!(resp["type"], "subscribed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_ws_subscribe_events_with_address_wildcard() {
    let (addr, _handle) = start_v2_server().await;
    let mut ws = ws_connect(addr).await;

    let resp = ws_send_recv(
        &mut ws,
        serde_json::json!({
            "action": "subscribe",
            "type": "events",
            "event_type": "0x1::*"
        }),
    )
    .await;

    assert_eq!(resp["type"], "subscribed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_ws_subscribe_events_with_sender_filter() {
    let (addr, _handle) = start_v2_server().await;
    let mut ws = ws_connect(addr).await;

    let resp = ws_send_recv(
        &mut ws,
        serde_json::json!({
            "action": "subscribe",
            "type": "events",
            "sender": "0x1"
        }),
    )
    .await;

    assert_eq!(resp["type"], "subscribed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_ws_subscribe_events_with_start_version() {
    let (addr, _handle) = start_v2_server().await;
    let mut ws = ws_connect(addr).await;

    let resp = ws_send_recv(
        &mut ws,
        serde_json::json!({
            "action": "subscribe",
            "type": "events",
            "start_version": 100
        }),
    )
    .await;

    assert_eq!(resp["type"], "subscribed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_ws_subscribe_events_with_combined_filters() {
    let (addr, _handle) = start_v2_server().await;
    let mut ws = ws_connect(addr).await;

    // Combine: multiple types + sender + start_version
    let resp = ws_send_recv(
        &mut ws,
        serde_json::json!({
            "action": "subscribe",
            "type": "events",
            "event_types": ["0x1::coin::DepositEvent", "0x1::coin::WithdrawEvent"],
            "sender": "0xABCD",
            "start_version": 50
        }),
    )
    .await;

    assert_eq!(resp["type"], "subscribed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_ws_subscribe_events_merged_type_and_types() {
    let (addr, _handle) = start_v2_server().await;
    let mut ws = ws_connect(addr).await;

    // Both event_type and event_types should be merged (backward compat).
    let resp = ws_send_recv(
        &mut ws,
        serde_json::json!({
            "action": "subscribe",
            "type": "events",
            "event_type": "0x1::coin::DepositEvent",
            "event_types": ["0x2::nft::TransferEvent"]
        }),
    )
    .await;

    assert_eq!(resp["type"], "subscribed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_ws_subscribe_events_no_filter_matches_all() {
    let (addr, _handle) = start_v2_server().await;
    let mut ws = ws_connect(addr).await;

    // No type, sender, or version filters — should match all events.
    let resp = ws_send_recv(
        &mut ws,
        serde_json::json!({
            "action": "subscribe",
            "type": "events"
        }),
    )
    .await;

    assert_eq!(resp["type"], "subscribed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_ws_subscribe_tx_status_invalid_hash() {
    let (addr, _handle) = start_v2_server().await;
    let mut ws = ws_connect(addr).await;

    let resp = ws_send_recv(
        &mut ws,
        serde_json::json!({
            "action": "subscribe",
            "type": "transaction_status",
            "hash": "not_a_valid_hash"
        }),
    )
    .await;

    assert_eq!(resp["type"], "error");
    assert_eq!(resp["code"], "INVALID_HASH");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_ws_subscribe_tx_status_sends_pending_then_not_found() {
    let (addr, _handle) = start_v2_server().await;
    let mut ws = ws_connect(addr).await;

    // Subscribe to a fake tx hash that won't be found.
    // Use a very short timeout so the test finishes quickly.
    let fake_hash = "0x0000000000000000000000000000000000000000000000000000000000000001";

    use futures::{SinkExt, StreamExt};
    ws.send(tungstenite::Message::Text(
        serde_json::json!({
            "action": "subscribe",
            "id": "tx_track",
            "type": "transaction_status",
            "hash": fake_hash
        })
        .to_string(),
    ))
    .await
    .unwrap();

    // Should get "subscribed" ack, then "pending" status, then eventually "not_found".
    let mut got_subscribed = false;
    let mut got_pending = false;
    let mut got_not_found = false;

    let deadline = std::time::Duration::from_secs(35);
    let start = std::time::Instant::now();

    while start.elapsed() < deadline {
        let msg = tokio::time::timeout(std::time::Duration::from_secs(32), ws.next()).await;
        match msg {
            Ok(Some(Ok(tungstenite::Message::Text(text)))) => {
                let val: serde_json::Value = serde_json::from_str(&text).unwrap();
                match val["type"].as_str() {
                    Some("subscribed") => got_subscribed = true,
                    Some("transaction_status_update") => match val["data"]["status"].as_str() {
                        Some("pending") => got_pending = true,
                        Some("not_found") => {
                            got_not_found = true;
                            break;
                        },
                        _ => {},
                    },
                    _ => {},
                }
            },
            _ => break,
        }
    }

    assert!(got_subscribed, "Should have received 'subscribed' ack");
    assert!(got_pending, "Should have received 'pending' status");
    assert!(got_not_found, "Should have received 'not_found' status");
}

// ======================================================================
// OpenAPI spec tests
// ======================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_openapi_spec_json() {
    let (addr, _handle) = start_v2_server().await;

    let resp = reqwest::get(format!("{}/v2/spec.json", base_url(addr)))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    // Must be a valid OpenAPI 3.1.x doc.
    assert!(
        body["openapi"].as_str().unwrap().starts_with("3."),
        "Expected OpenAPI 3.x, got: {}",
        body["openapi"]
    );
    assert_eq!(body["info"]["title"], "Aptos Node API v2");
    assert_eq!(body["info"]["version"], "2.0.0");

    // Check that paths are populated.
    let paths = body["paths"]
        .as_object()
        .expect("paths should be an object");
    assert!(
        paths.len() >= 10,
        "Expected at least 10 paths, got {}",
        paths.len()
    );

    // Verify some specific paths exist.
    assert!(paths.contains_key("/v2/health"), "Missing /v2/health");
    assert!(paths.contains_key("/v2/info"), "Missing /v2/info");
    assert!(
        paths.contains_key("/v2/transactions"),
        "Missing /v2/transactions"
    );
    assert!(
        paths.contains_key("/v2/blocks/latest"),
        "Missing /v2/blocks/latest"
    );
    assert!(paths.contains_key("/v2/view"), "Missing /v2/view");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_openapi_spec_yaml() {
    let (addr, _handle) = start_v2_server().await;

    let resp = reqwest::get(format!("{}/v2/spec.yaml", base_url(addr)))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let content_type = resp
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(
        content_type.contains("yaml"),
        "Expected YAML content type, got: {}",
        content_type
    );

    let body = resp.text().await.unwrap();
    assert!(
        body.contains("openapi:"),
        "YAML should contain openapi field"
    );
    assert!(
        body.contains("Aptos Node API v2"),
        "YAML should contain API title"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_openapi_spec_schemas() {
    let (addr, _handle) = start_v2_server().await;

    let resp = reqwest::get(format!("{}/v2/spec.json", base_url(addr)))
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();

    let schemas = body["components"]["schemas"]
        .as_object()
        .expect("schemas should be an object");

    // Check that our custom schemas are present.
    assert!(
        schemas.contains_key("LedgerMetadata"),
        "Missing LedgerMetadata schema"
    );
    assert!(schemas.contains_key("V2Error"), "Missing V2Error schema");
    assert!(
        schemas.contains_key("ErrorCode"),
        "Missing ErrorCode schema"
    );
    assert!(
        schemas.contains_key("HealthResponse"),
        "Missing HealthResponse schema"
    );
    assert!(schemas.contains_key("NodeInfo"), "Missing NodeInfo schema");
    assert!(
        schemas.contains_key("SubmitResult"),
        "Missing SubmitResult schema"
    );
    assert!(
        schemas.contains_key("TransactionSummary"),
        "Missing TransactionSummary schema"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_openapi_spec_tags() {
    let (addr, _handle) = start_v2_server().await;

    let resp = reqwest::get(format!("{}/v2/spec.json", base_url(addr)))
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();

    let tags = body["tags"].as_array().expect("tags should be an array");
    let tag_names: Vec<&str> = tags.iter().filter_map(|t| t["name"].as_str()).collect();

    assert!(tag_names.contains(&"Health"), "Missing Health tag");
    assert!(tag_names.contains(&"Accounts"), "Missing Accounts tag");
    assert!(
        tag_names.contains(&"Transactions"),
        "Missing Transactions tag"
    );
    assert!(tag_names.contains(&"Blocks"), "Missing Blocks tag");
    assert!(tag_names.contains(&"View"), "Missing View tag");
    assert!(tag_names.contains(&"Events"), "Missing Events tag");
}

// ======================================================================
// TLS tests
// ======================================================================

/// Self-signed EC test certificate (CN=localhost, valid 1 year).
const TEST_TLS_CERT: &str = r#"-----BEGIN CERTIFICATE-----
MIIBfTCCASOgAwIBAgIUcOjGtWC925LfCcMdCIl+3UOKdg4wCgYIKoZIzj0EAwIw
FDESMBAGA1UEAwwJbG9jYWxob3N0MB4XDTI2MDIwODAzMzYzNVoXDTI3MDIwODAz
MzYzNVowFDESMBAGA1UEAwwJbG9jYWxob3N0MFkwEwYHKoZIzj0CAQYIKoZIzj0D
AQcDQgAEiJdKi1sKI8Qi5xwlhsV0gTPN5TdJl/9DC/qjNOFwdh9kvjl3bEqJ6MKO
xdBJ88gx5TSXmkmEQXTK6KurvfYBS6NTMFEwHQYDVR0OBBYEFE9tQ7FIQkqr3ju/
5nLutCmVZpruMB8GA1UdIwQYMBaAFE9tQ7FIQkqr3ju/5nLutCmVZpruMA8GA1Ud
EwEB/wQFMAMBAf8wCgYIKoZIzj0EAwIDSAAwRQIhAJAVqDQ7/jlwHsGEhU5tCn4L
+8PM9+QL2N0anMERrfrtAiAKHwclC6A9qWIIG0ITy/i989VGOxZtx/CWAytxu6TE
7g==
-----END CERTIFICATE-----"#;

/// PKCS8 private key for the test certificate.
const TEST_TLS_KEY: &str = r#"-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgFlINaDZ+BWjxSOw/
yRRqNdN9kPFVz4VWyn4nAZFDmdGhRANCAASIl0qLWwojxCLnHCWGxXSBM83lN0mX
/0ML+qM04XB2H2S+OXdsSonowo7F0EnzyDHlNJeaSYRBdMroq6u99gFL
-----END PRIVATE KEY-----"#;

/// Write test cert and key to temp files, return their paths.
fn write_test_tls_files() -> (tempfile::NamedTempFile, tempfile::NamedTempFile) {
    use std::io::Write;
    let mut cert_file = tempfile::NamedTempFile::new().expect("Failed to create temp cert file");
    cert_file
        .write_all(TEST_TLS_CERT.as_bytes())
        .expect("Failed to write cert");
    cert_file.flush().unwrap();

    let mut key_file = tempfile::NamedTempFile::new().expect("Failed to create temp key file");
    key_file
        .write_all(TEST_TLS_KEY.as_bytes())
        .expect("Failed to write key");
    key_file.flush().unwrap();

    (cert_file, key_file)
}

/// Helper: start a v2 server with TLS enabled.
async fn start_tls_v2_server() -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let (cert_file, key_file) = write_test_tls_files();

    let mut node_config = NodeConfig::default();
    node_config.storage.rocksdb_configs.enable_storage_sharding = false;

    let test_ctx = new_test_context_no_api("v2_tls_test".to_string(), node_config.clone(), false);

    let context = Context::new(
        ChainId::test(),
        test_ctx.db.clone(),
        test_ctx.mempool.ac_client.clone(),
        node_config.clone(),
        None,
    );

    let v2_config = V2Config::from_configs(&node_config.api_v2, &node_config.api);
    let v2_ctx = V2Context::new(context, v2_config);
    let router = build_v2_router(v2_ctx);

    let tls_acceptor = crate::v2::tls::build_tls_acceptor(
        cert_file.path().to_str().unwrap(),
        key_file.path().to_str().unwrap(),
    )
    .expect("Failed to build TLS acceptor");

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind");
    let addr = listener.local_addr().unwrap();

    let handle = tokio::spawn(async move {
        // Keep temp files alive for the duration of the server (not strictly
        // needed since we already built the acceptor, but avoids confusion).
        let _cert = cert_file;
        let _key = key_file;

        // The shutdown_tx must stay alive for the lifetime of the server.
        // If the sender is dropped, serve_tls's `wait_for` receives a
        // channel-closed error and exits the accept loop immediately.
        let (_shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        crate::v2::tls::serve_tls(listener, tls_acceptor, router, None, shutdown_rx, 30_000).await;
    });

    // Give the server a moment to start.
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    (addr, handle)
}

/// Build a reqwest client that accepts the test self-signed certificate.
///
/// We use `use_rustls_tls()` to match the server's TLS stack (the default
/// native-tls on macOS rejects the test cert even with
/// `danger_accept_invalid_certs`). The `rustls-tls` feature is enabled
/// as a dev-dependency in Cargo.toml.
fn tls_test_client() -> reqwest::Client {
    reqwest::Client::builder()
        .use_rustls_tls()
        .danger_accept_invalid_certs(true)
        .build()
        .expect("Failed to build TLS client")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_tls_health_endpoint() {
    let (addr, _handle) = start_tls_v2_server().await;
    let client = tls_test_client();

    // Use 127.0.0.1 instead of localhost to ensure IPv4 (the server binds
    // on 127.0.0.1 and localhost may resolve to ::1 on some systems).
    let resp = client
        .get(format!("https://127.0.0.1:{}/v2/health", addr.port()))
        .send()
        .await
        .expect("TLS request failed");

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_tls_info_endpoint() {
    let (addr, _handle) = start_tls_v2_server().await;
    let client = tls_test_client();

    let resp = client
        .get(format!("https://127.0.0.1:{}/v2/info", addr.port()))
        .send()
        .await
        .expect("TLS request failed");

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    // The info endpoint returns a V2Response envelope.
    assert!(
        body["data"].is_object() || body["ledger"].is_object(),
        "Expected a structured info response, got: {}",
        body
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_tls_resources_endpoint() {
    let (addr, _handle) = start_tls_v2_server().await;
    let client = tls_test_client();

    let resp = client
        .get(format!(
            "https://127.0.0.1:{}/v2/accounts/0x1/resources",
            addr.port()
        ))
        .send()
        .await
        .expect("TLS request failed");

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["data"].is_array());
    assert!(!body["data"].as_array().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_tls_build_acceptor_invalid_cert() {
    // Should fail with a descriptive error.
    let result =
        crate::v2::tls::build_tls_acceptor("/nonexistent/cert.pem", "/nonexistent/key.pem");
    assert!(result.is_err());
    let err_msg = format!("{}", result.err().unwrap());
    assert!(
        err_msg.contains("Failed to open TLS cert file"),
        "Error should mention cert file: {}",
        err_msg
    );
}

// ---- Account Info ----

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_get_account() {
    let (addr, _handle) = start_v2_server().await;

    // 0x1 should exist (framework account)
    let resp = reqwest::get(format!("{}/v2/accounts/0x1", base_url(addr)))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "Expected 200 for 0x1 account");

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(
        body["data"]["sequence_number"].is_string(),
        "sequence_number should be present"
    );
    assert!(
        body["data"]["authentication_key"].is_string(),
        "authentication_key should be present"
    );
    assert!(body["ledger"]["ledger_version"].is_number());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_get_account_not_found() {
    let (addr, _handle) = start_v2_server().await;

    // Random address that doesn't exist
    let resp = reqwest::get(format!(
        "{}/v2/accounts/0xdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef",
        base_url(addr)
    ))
    .await
    .unwrap();
    // Expect 404 (or possibly 200 if stateless accounts are enabled)
    let status = resp.status().as_u16();
    assert!(
        status == 404 || status == 200,
        "Expected 404 or 200, got {}",
        status
    );
}

// ---- Gas Estimation ----

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_estimate_gas_price() {
    let (addr, _handle) = start_v2_server().await;

    let resp = reqwest::get(format!("{}/v2/estimate_gas_price", base_url(addr)))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(
        body["data"]["gas_estimate"].is_number(),
        "gas_estimate should be present"
    );
    assert!(body["ledger"]["ledger_version"].is_number());
}

// ---- Transaction by Version ----

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_get_transaction_by_version() {
    let (addr, _handle) = start_v2_server().await;

    // Version 0 should be the genesis transaction
    let resp = reqwest::get(format!("{}/v2/transactions/by_version/0", base_url(addr)))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "Expected 200 for version 0");

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(
        body["data"]["type"].is_string(),
        "Transaction should have a type field"
    );
    assert!(body["ledger"]["ledger_version"].is_number());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_get_transaction_by_version_not_found() {
    let (addr, _handle) = start_v2_server().await;

    // Version far in the future
    let resp = reqwest::get(format!(
        "{}/v2/transactions/by_version/999999999",
        base_url(addr)
    ))
    .await
    .unwrap();
    assert_eq!(resp.status(), 404);
}

// ---- Block by Version ----

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_get_block_by_version() {
    let (addr, _handle) = start_v2_server().await;

    // Version 0 should be in a block
    let resp = reqwest::get(format!("{}/v2/blocks/by_version/0", base_url(addr)))
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "Expected 200 for block containing version 0"
    );

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(
        body["data"]["block_height"].is_string(),
        "block_height should be present"
    );
    assert!(
        body["data"]["block_hash"].is_string(),
        "block_hash should be present"
    );
    assert!(body["ledger"]["ledger_version"].is_number());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_get_block_by_version_not_found() {
    let (addr, _handle) = start_v2_server().await;

    let resp = reqwest::get(format!("{}/v2/blocks/by_version/999999999", base_url(addr)))
        .await
        .unwrap();
    // Should be 404 (version in the future) or 500 (from v1 error)
    let status = resp.status().as_u16();
    assert!(
        status == 404 || status == 500,
        "Expected 404 or 500, got {}",
        status
    );
}

// ---- Batch with new methods ----

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_batch_get_account() {
    let (addr, _handle) = start_v2_server().await;

    let batch_body = serde_json::json!([{
        "jsonrpc": "2.0",
        "method": "get_account",
        "params": { "address": "0x1" },
        "id": 1
    }]);

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/v2/batch", base_url(addr)))
        .json(&batch_body)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    let results = body.as_array().unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0]["result"]["sequence_number"].is_string());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_batch_estimate_gas_price() {
    let (addr, _handle) = start_v2_server().await;

    let batch_body = serde_json::json!([{
        "jsonrpc": "2.0",
        "method": "estimate_gas_price",
        "params": {},
        "id": 1
    }]);

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/v2/batch", base_url(addr)))
        .json(&batch_body)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    let results = body.as_array().unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0]["result"]["gas_estimate"].is_number());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_batch_get_transaction_by_version() {
    let (addr, _handle) = start_v2_server().await;

    let batch_body = serde_json::json!([{
        "jsonrpc": "2.0",
        "method": "get_transaction_by_version",
        "params": { "version": 0 },
        "id": 1
    }]);

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/v2/batch", base_url(addr)))
        .json(&batch_body)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    let results = body.as_array().unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0]["result"]["type"].is_string());
}

// ---- Balance ----

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_get_balance_apt() {
    let (addr, _handle) = start_v2_server().await;

    // Query AptosCoin balance for 0x1 (framework account)
    let resp = reqwest::get(format!(
        "{}/v2/accounts/0x1/balance/0x1::aptos_coin::AptosCoin",
        base_url(addr)
    ))
    .await
    .unwrap();
    assert_eq!(resp.status(), 200, "Expected 200 for APT balance");

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(
        body["data"]["balance"].is_number(),
        "balance should be a number"
    );
    assert!(body["ledger"]["ledger_version"].is_number());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_get_balance_invalid_asset() {
    let (addr, _handle) = start_v2_server().await;

    let resp = reqwest::get(format!(
        "{}/v2/accounts/0x1/balance/not_a_valid_type",
        base_url(addr)
    ))
    .await
    .unwrap();
    assert_eq!(resp.status(), 400, "Expected 400 for invalid asset type");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_get_balance_nonexistent_coin() {
    let (addr, _handle) = start_v2_server().await;

    // Query a coin that doesn't exist -- should return 0 balance (not an error),
    // since the account may simply not hold that coin.
    let resp = reqwest::get(format!(
        "{}/v2/accounts/0x1/balance/0x1::fake_coin::FakeCoin",
        base_url(addr)
    ))
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["data"]["balance"], 0);
}

// ---- Request Timeout ----

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_request_timeout_fast_request_succeeds() {
    // Use default timeout (30s). A fast health check should succeed easily.
    let (addr, _ctx, _handle) = start_v2_server_with_config(|_| {}).await;

    let resp = reqwest::get(format!("{}/v2/health", base_url(addr)))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_request_timeout_disabled() {
    // Timeout = 0 means disabled; requests should still work.
    let (addr, _ctx, _handle) = start_v2_server_with_config(|cfg| {
        cfg.request_timeout_ms = 0;
    })
    .await;

    let resp = reqwest::get(format!("{}/v2/health", base_url(addr)))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

// ---- Graceful Shutdown ----

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_graceful_shutdown_stops_accepting() {
    let (addr, ctx, handle) = start_v2_server_with_config(|cfg| {
        cfg.graceful_shutdown_timeout_ms = 5_000;
    })
    .await;

    // Verify server is healthy before shutdown.
    let resp = reqwest::get(format!("{}/v2/health", base_url(addr)))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // Trigger shutdown.
    ctx.trigger_shutdown();

    // Wait for the server task to complete.
    let _ = tokio::time::timeout(Duration::from_secs(5), handle).await;

    // After shutdown, new connections should fail.
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();
    let result = client
        .get(format!("{}/v2/health", base_url(addr)))
        .send()
        .await;
    assert!(result.is_err(), "Requests should fail after shutdown");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_graceful_shutdown_drains_inflight() {
    let (addr, ctx, handle) = start_v2_server_with_config(|cfg| {
        cfg.graceful_shutdown_timeout_ms = 5_000;
    })
    .await;

    // Start a health request (which should be fast and complete before drain).
    let url = format!("{}/v2/health", base_url(addr));
    let resp = reqwest::get(&url).await.unwrap();
    assert_eq!(resp.status(), 200, "Pre-shutdown request should succeed");

    // Trigger shutdown and verify it completes cleanly.
    ctx.trigger_shutdown();
    let result = tokio::time::timeout(Duration::from_secs(5), handle).await;
    assert!(result.is_ok(), "Server should shut down within 5 seconds");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_graceful_shutdown_immediate() {
    // drain timeout = 0 means immediate shutdown.
    let (addr, ctx, handle) = start_v2_server_with_config(|cfg| {
        cfg.graceful_shutdown_timeout_ms = 0;
    })
    .await;

    // Verify server works first.
    let resp = reqwest::get(format!("{}/v2/health", base_url(addr)))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // Trigger immediate shutdown.
    ctx.trigger_shutdown();

    // Should complete very quickly.
    let result = tokio::time::timeout(Duration::from_secs(3), handle).await;
    assert!(result.is_ok(), "Immediate shutdown should be fast");
}

// ---- SSE (Server-Sent Events) ----

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_sse_blocks_returns_event_stream() {
    let (addr, _handle) = start_v2_server().await;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap();

    let resp = client
        .get(format!("{}/v2/sse/blocks", base_url(addr)))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let ct = resp
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(
        ct.starts_with("text/event-stream"),
        "Expected text/event-stream, got: {}",
        ct
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_sse_events_returns_event_stream() {
    let (addr, _handle) = start_v2_server().await;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap();

    let resp = client
        .get(format!("{}/v2/sse/events", base_url(addr)))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let ct = resp
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(
        ct.starts_with("text/event-stream"),
        "Expected text/event-stream, got: {}",
        ct
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_sse_events_with_filters() {
    let (addr, _handle) = start_v2_server().await;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap();

    // Events endpoint should accept filter query params.
    let resp = client
        .get(format!(
            "{}/v2/sse/events?event_types=0x1::coin::DepositEvent,0x1::coin::WithdrawEvent&sender=0x1&start_version=100",
            base_url(addr)
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let ct = resp
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(ct.starts_with("text/event-stream"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_sse_blocks_with_after_height() {
    let (addr, _handle) = start_v2_server().await;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap();

    // Blocks endpoint should accept after_height query param.
    let resp = client
        .get(format!(
            "{}/v2/sse/blocks?after_height=42",
            base_url(addr)
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let ct = resp
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(ct.starts_with("text/event-stream"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_sse_disabled_returns_error() {
    let (addr, _ctx, _handle) = start_v2_server_with_config(|cfg| {
        cfg.sse_enabled = false;
    })
    .await;

    let resp = reqwest::get(format!("{}/v2/sse/blocks", base_url(addr)))
        .await
        .unwrap();
    assert_eq!(resp.status(), 503);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["code"], "SERVICE_UNAVAILABLE");
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("SSE is disabled"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_v2_sse_spec_includes_sse_endpoints() {
    let (addr, _handle) = start_v2_server().await;

    let resp = reqwest::get(format!("{}/v2/spec.json", base_url(addr)))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let spec: serde_json::Value = resp.json().await.unwrap();
    let paths = spec["paths"].as_object().unwrap();

    assert!(
        paths.contains_key("/v2/sse/blocks"),
        "Spec should contain /v2/sse/blocks"
    );
    assert!(
        paths.contains_key("/v2/sse/events"),
        "Spec should contain /v2/sse/events"
    );

    // Verify SSE tag exists.
    let tags: Vec<&str> = spec["tags"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    assert!(tags.contains(&"SSE"), "Spec should have SSE tag");
}

// ======================================================================
// End-to-end integration tests
// ======================================================================

/// Helper: start a v2 server and return the TestContext for E2E testing.
///
/// Unlike `start_v2_server`, this returns the `TestContext` so tests can
/// create transactions, commit blocks, and advance the chain state.
/// An optional `config_fn` allows customizing the NodeConfig before the
/// server starts (e.g., to increase wait timeouts for the wait endpoint test).
async fn start_v2_e2e_server() -> (
    SocketAddr,
    TestContext,
    V2Context,
    tokio::task::JoinHandle<()>,
) {
    start_v2_e2e_server_ext(|_| {}).await
}

async fn start_v2_e2e_server_ext(
    config_fn: impl FnOnce(&mut NodeConfig),
) -> (
    SocketAddr,
    TestContext,
    V2Context,
    tokio::task::JoinHandle<()>,
) {
    let mut node_config = NodeConfig::default();
    node_config.storage.rocksdb_configs.enable_storage_sharding = false;
    config_fn(&mut node_config);

    // E2E tests need the full TestContext with a running Poem v1 server because
    // methods like create_user_account / root_account use the Poem API to query
    // sequence numbers. The v1 metrics AlreadyReg issue is handled by making
    // the v1 metrics registration resilient (see api/src/metrics.rs).
    let test_ctx = create_test_context("v2_e2e_test".to_string(), node_config.clone(), false);

    let context = Context::new(
        ChainId::test(),
        test_ctx.db.clone(),
        test_ctx.mempool.ac_client.clone(),
        node_config.clone(),
        None,
    );

    let v2_config = V2Config::from_configs(&node_config.api_v2, &node_config.api);
    let v2_ctx = V2Context::new(context, v2_config);
    let router = build_v2_router(v2_ctx.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind");
    let addr = listener.local_addr().unwrap();

    let handle = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    (addr, test_ctx, v2_ctx, handle)
}

/// Helper: BCS-encode a SignedTransaction wrapped in the Versioned envelope.
fn encode_versioned_bcs(txn: aptos_types::transaction::SignedTransaction) -> Vec<u8> {
    let versioned = super::extractors::Versioned::V1(txn);
    bcs::to_bytes(&versioned).expect("BCS serialization failed")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_e2e_submit_bcs_transaction() {
    let (addr, mut test_ctx, _v2_ctx, _handle) = start_v2_e2e_server().await;

    // Create a signed transaction using the test context.
    let account = test_ctx.gen_account();
    let txn = test_ctx.create_user_account(&account).await;
    let expected_hash = txn.committed_hash().to_hex_literal();

    // Wrap in Versioned envelope and BCS-encode.
    let body = encode_versioned_bcs(txn);

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/v2/transactions", base_url(addr)))
        .header("content-type", "application/x-bcs")
        .body(body)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "BCS submission should be accepted");

    let resp_body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(resp_body["data"]["status"], "accepted");
    assert_eq!(resp_body["data"]["hash"], expected_hash);
    assert!(
        resp_body["ledger"]["ledger_version"].is_number(),
        "Response should include ledger metadata"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_e2e_submit_commit_verify_by_hash() {
    let (addr, mut test_ctx, _v2_ctx, _handle) = start_v2_e2e_server().await;

    let account = test_ctx.gen_account();
    let txn = test_ctx.create_user_account(&account).await;
    let hash = txn.committed_hash().to_hex_literal();

    // Submit via v2 API.
    let body = encode_versioned_bcs(txn);

    let client = reqwest::Client::new();
    let submit_resp = client
        .post(format!("{}/v2/transactions", base_url(addr)))
        .header("content-type", "application/x-bcs")
        .body(body)
        .send()
        .await
        .unwrap();
    assert_eq!(submit_resp.status(), 200);

    // Commit the transaction from mempool to DB.
    test_ctx.commit_mempool_txns(1).await;

    // Verify it's committed via GET.
    let get_resp = client
        .get(format!("{}/v2/transactions/{}", base_url(addr), hash))
        .send()
        .await
        .unwrap();
    assert_eq!(
        get_resp.status(),
        200,
        "Committed transaction should be found"
    );

    let resp_body: serde_json::Value = get_resp.json().await.unwrap();
    assert_eq!(resp_body["data"]["type"], "user_transaction");
    assert!(resp_body["data"]["success"].as_bool().unwrap_or(false));
    assert!(resp_body["data"]["hash"].as_str().is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_e2e_submit_and_wait_for_commit() {
    // Use a longer wait timeout (10s) since the default is only 1s.
    let (addr, mut test_ctx, _v2_ctx, _handle) = start_v2_e2e_server_ext(|cfg| {
        cfg.api.wait_by_hash_timeout_ms = 10_000;
    })
    .await;

    let account = test_ctx.gen_account();
    let txn = test_ctx.create_user_account(&account).await;
    let hash = txn.committed_hash().to_hex_literal();

    // Submit via v2 API.
    let body = encode_versioned_bcs(txn);

    let client = reqwest::Client::new();
    let submit_resp = client
        .post(format!("{}/v2/transactions", base_url(addr)))
        .header("content-type", "application/x-bcs")
        .body(body)
        .send()
        .await
        .unwrap();
    assert_eq!(submit_resp.status(), 200);

    // Start wait in background, then commit after a short delay.
    let wait_client = client.clone();
    let wait_url = format!("{}/v2/transactions/{}/wait", base_url(addr), hash);
    let wait_handle = tokio::spawn(async move { wait_client.get(&wait_url).send().await.unwrap() });

    // Give the wait endpoint a moment to start polling, then commit.
    tokio::time::sleep(Duration::from_millis(500)).await;
    test_ctx.commit_mempool_txns(1).await;

    // The wait should return the committed transaction.
    let wait_resp = tokio::time::timeout(Duration::from_secs(15), wait_handle)
        .await
        .expect("Wait timed out")
        .expect("Wait task failed");
    assert_eq!(wait_resp.status(), 200, "Wait should return committed tx");

    let resp_body: serde_json::Value = wait_resp.json().await.unwrap();
    assert_eq!(resp_body["data"]["type"], "user_transaction");
    assert!(resp_body["data"]["success"].as_bool().unwrap_or(false));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_e2e_ws_new_block_on_commit() {
    let (addr, mut test_ctx, v2_ctx, _handle) = start_v2_e2e_server().await;

    // Start the block poller so WebSocket clients receive notifications.
    let ws_tx = v2_ctx.ws_broadcaster();
    tokio::spawn(super::websocket::broadcaster::run_block_poller(
        v2_ctx.clone(),
        ws_tx,
    ));

    // Give the poller time to process the genesis block.
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Connect WebSocket and subscribe to new_blocks.
    let mut ws = ws_connect(addr).await;
    let sub_resp = ws_send_recv(
        &mut ws,
        serde_json::json!({
            "action": "subscribe",
            "id": "e2e_blocks",
            "type": "new_blocks"
        }),
    )
    .await;
    assert_eq!(sub_resp["type"], "subscribed");

    // Create and commit a transaction (which produces a new block).
    let account = test_ctx.gen_account();
    let txn = test_ctx.create_user_account(&account).await;
    test_ctx.commit_block(&vec![txn]).await;

    // Wait for the WebSocket to deliver a new_block notification.
    use futures::StreamExt;
    let deadline = Duration::from_secs(5);
    let start = std::time::Instant::now();
    let mut got_new_block = false;

    while start.elapsed() < deadline {
        match tokio::time::timeout(Duration::from_secs(3), ws.next()).await {
            Ok(Some(Ok(tungstenite::Message::Text(text)))) => {
                let val: serde_json::Value = serde_json::from_str(&text).unwrap();
                if val["type"] == "new_block" {
                    // The committed block should have height > 0 (genesis is 0).
                    let height = val["data"]["height"].as_u64().unwrap_or(0);
                    if height > 0 {
                        got_new_block = true;
                        assert!(
                            val["data"]["num_transactions"].as_u64().unwrap_or(0) > 0,
                            "Block should have transactions"
                        );
                        break;
                    }
                }
            },
            _ => break,
        }
    }
    assert!(
        got_new_block,
        "Should have received a new_block notification for the committed block"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_e2e_full_flow_account_creation() {
    let (addr, mut test_ctx, _v2_ctx, _handle) = start_v2_e2e_server().await;

    let account = test_ctx.gen_account();
    let new_addr = format!("{}", account.address());

    // Submit CreateAccount transaction via v2 API.
    let client = reqwest::Client::new();
    let txn = test_ctx.create_user_account(&account).await;
    let hash = txn.committed_hash().to_hex_literal();
    let body = encode_versioned_bcs(txn);

    let submit_resp = client
        .post(format!("{}/v2/transactions", base_url(addr)))
        .header("content-type", "application/x-bcs")
        .body(body)
        .send()
        .await
        .unwrap();
    assert_eq!(submit_resp.status(), 200);

    let submit_body: serde_json::Value = submit_resp.json().await.unwrap();
    assert_eq!(submit_body["data"]["hash"], hash);

    // Commit the transaction.
    test_ctx.commit_mempool_txns(1).await;

    // Wait for the ledger info cache to expire (50ms TTL).
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Verify the committed transaction succeeded.
    let tx_resp = client
        .get(format!("{}/v2/transactions/{}", base_url(addr), hash))
        .send()
        .await
        .unwrap();
    assert_eq!(
        tx_resp.status(),
        200,
        "Committed CreateAccount transaction should be found"
    );
    let tx_body: serde_json::Value = tx_resp.json().await.unwrap();
    assert!(
        tx_body["data"]["success"].as_bool().unwrap_or(false),
        "CreateAccount transaction should succeed"
    );
    assert_eq!(tx_body["data"]["type"], "user_transaction");

    // Verify the new account is accessible via the accounts endpoint.
    let acct_resp = client
        .get(format!("{}/v2/accounts/{}", base_url(addr), new_addr))
        .send()
        .await
        .unwrap();
    assert_eq!(acct_resp.status(), 200, "Account info should be available");
    let acct_body: serde_json::Value = acct_resp.json().await.unwrap();
    assert!(
        acct_body["data"]["sequence_number"].is_string(),
        "Should have sequence_number"
    );
    assert_eq!(
        acct_body["data"]["sequence_number"], "0",
        "New account should have seq_num 0"
    );

    // Verify the ledger advanced from the committed transaction.
    assert!(
        acct_body["ledger"]["ledger_version"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "Ledger should have advanced past genesis"
    );
}
