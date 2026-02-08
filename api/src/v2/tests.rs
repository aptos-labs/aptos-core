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
                    Some("transaction_status_update") => {
                        match val["data"]["status"].as_str() {
                            Some("pending") => got_pending = true,
                            Some("not_found") => {
                                got_not_found = true;
                                break;
                            },
                            _ => {},
                        }
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
    let paths = body["paths"].as_object().expect("paths should be an object");
    assert!(paths.len() >= 10, "Expected at least 10 paths, got {}", paths.len());

    // Verify some specific paths exist.
    assert!(paths.contains_key("/v2/health"), "Missing /v2/health");
    assert!(paths.contains_key("/v2/info"), "Missing /v2/info");
    assert!(paths.contains_key("/v2/transactions"), "Missing /v2/transactions");
    assert!(paths.contains_key("/v2/blocks/latest"), "Missing /v2/blocks/latest");
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
    assert!(body.contains("openapi:"), "YAML should contain openapi field");
    assert!(body.contains("Aptos Node API v2"), "YAML should contain API title");
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
    assert!(schemas.contains_key("LedgerMetadata"), "Missing LedgerMetadata schema");
    assert!(schemas.contains_key("V2Error"), "Missing V2Error schema");
    assert!(schemas.contains_key("ErrorCode"), "Missing ErrorCode schema");
    assert!(schemas.contains_key("HealthResponse"), "Missing HealthResponse schema");
    assert!(schemas.contains_key("NodeInfo"), "Missing NodeInfo schema");
    assert!(schemas.contains_key("SubmitResult"), "Missing SubmitResult schema");
    assert!(schemas.contains_key("TransactionSummary"), "Missing TransactionSummary schema");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_openapi_spec_tags() {
    let (addr, _handle) = start_v2_server().await;

    let resp = reqwest::get(format!("{}/v2/spec.json", base_url(addr)))
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();

    let tags = body["tags"].as_array().expect("tags should be an array");
    let tag_names: Vec<&str> = tags
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();

    assert!(tag_names.contains(&"Health"), "Missing Health tag");
    assert!(tag_names.contains(&"Accounts"), "Missing Accounts tag");
    assert!(tag_names.contains(&"Transactions"), "Missing Transactions tag");
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

    let test_ctx = create_test_context("v2_tls_test".to_string(), node_config.clone(), false);

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
        crate::v2::tls::serve_tls(listener, tls_acceptor, router, None).await;
    });

    // Give the server a moment to start.
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    (addr, handle)
}

/// Build a reqwest client that accepts the test self-signed certificate.
/// We use `danger_accept_invalid_certs` because the self-signed cert is
/// not in the system trust store. This is safe for test purposes only.
fn tls_test_client() -> reqwest::Client {
    reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .expect("Failed to build TLS client")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_tls_health_endpoint() {
    let (addr, _handle) = start_tls_v2_server().await;
    let client = tls_test_client();

    let resp = client
        .get(format!("https://localhost:{}/v2/health", addr.port()))
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
        .get(format!("https://localhost:{}/v2/info", addr.port()))
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
            "https://localhost:{}/v2/accounts/0x1/resources",
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
    let result = crate::v2::tls::build_tls_acceptor("/nonexistent/cert.pem", "/nonexistent/key.pem");
    assert!(result.is_err());
    let err_msg = format!("{}", result.err().unwrap());
    assert!(
        err_msg.contains("Failed to open TLS cert file"),
        "Error should mention cert file: {}",
        err_msg
    );
}
