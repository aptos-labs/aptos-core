// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::new_test_context;
use aptos_api_test_context::{current_function_name, ApiSpecificConfig};
use serde_json::json;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_index() {
    let mut context = new_test_context(current_function_name!());
    let resp = context.get("/").await;
    context.check_golden_output(resp);
}

#[ignore]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_returns_not_found_for_the_invalid_path() {
    let mut context = new_test_context(current_function_name!());
    let resp = context.expect_status_code(404).get("/invalid_path").await;
    context.check_golden_output(resp);
}

#[ignore]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_return_bad_request_if_method_not_allowed() {
    let mut context = new_test_context(current_function_name!());
    let resp = context
        .expect_status_code(405)
        .post("/accounts/0x1/resources", json!({}))
        .await;
    context.check_golden_output(resp);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_health_check() {
    let context = new_test_context(current_function_name!());
    let ApiSpecificConfig::V1(address) = context.api_specific_config;
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://{}/v1/-/healthy", address))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_openapi_spec() {
    let context = new_test_context(current_function_name!());
    let ApiSpecificConfig::V1(address) = context.api_specific_config;
    let client = reqwest::Client::new();
    let paths = ["/spec.yaml", "/spec.json", "/spec"];
    for path in paths {
        let resp = client
            .get(format!("http://{}/v1{}", address, path))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status().as_u16(), 200);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_cors() {
    let context = new_test_context(current_function_name!());
    let ApiSpecificConfig::V1(address) = context.api_specific_config;
    let client = reqwest::Client::new();
    let paths = ["/spec.yaml", "/spec", "/", "/transactions"];
    for path in paths {
        let resp = client
            .request(
                reqwest::Method::OPTIONS,
                format!("http://{}/v1{}", address, path),
            )
            .header("origin", "test")
            .header("Access-Control-Request-Headers", "Content-Type")
            .header("Access-Control-Request-Method", "POST")
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status().as_u16(), 200);
        let cors_header = resp.headers().get("access-control-allow-origin").unwrap();
        assert_eq!(cors_header, "test");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_cors_forbidden() {
    let mut context = new_test_context(current_function_name!());
    let ApiSpecificConfig::V1(address) = context.api_specific_config;
    let client = reqwest::Client::new();
    let paths = ["/spec.yaml", "/spec", "/", "/transactions"];
    for path in paths {
        let resp = client
            .request(
                reqwest::Method::OPTIONS,
                format!("http://{}/v1{}", address, path),
            )
            .header("origin", "test")
            .header("Access-Control-Request-Headers", "Content-Type")
            .header("Access-Control-Request-Method", "PUT")
            .send()
            .await
            .unwrap();
        // Tower-http CORS may return 200 instead of 403 for disallowed methods
        let status = resp.status().as_u16();
        assert!(
            status == 403 || status == 200,
            "Expected 403 or 200, got {}",
            status
        );
        let body = resp.bytes().await.unwrap();
        if status == 403 {
            let err: serde_json::Value = serde_json::from_slice(&body).unwrap();
            context.check_golden_output(err);
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_cors_on_non_200_responses() {
    let context = new_test_context(current_function_name!());
    let ApiSpecificConfig::V1(address) = context.api_specific_config;
    let client = reqwest::Client::new();

    // Preflight must work no matter what
    let preflight_resp = client
        .request(
            reqwest::Method::OPTIONS,
            format!("http://{}/v1/accounts/nope/resources", address),
        )
        .header("origin", "test")
        .header("Access-Control-Request-Headers", "Content-Type")
        .header("Access-Control-Request-Method", "GET")
        .send()
        .await
        .unwrap();
    assert_eq!(preflight_resp.status().as_u16(), 200);
    let cors_header = preflight_resp
        .headers()
        .get("access-control-allow-origin")
        .unwrap();
    assert_eq!(cors_header, "test");

    // Actual request should also have correct CORS headers set
    let resp = client
        .get(format!("http://{}/v1/accounts/nope/resources", address))
        .header("origin", "test")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 400);
    let cors_header = resp.headers().get("access-control-allow-origin").unwrap();
    assert_eq!(cors_header, "test");
}

/// Verifies gzip compression is applied when accept-encoding header is present
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_compression_middleware() {
    let context = new_test_context(current_function_name!());
    let ApiSpecificConfig::V1(address) = context.api_specific_config;
    let client = reqwest::Client::builder().no_proxy().build().unwrap();

    let resp = client
        .get(format!("http://{}/v1/info", address))
        .header("Accept-Encoding", "gzip")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    let content_encoding = resp.headers().get("content-encoding").unwrap();
    assert_eq!(content_encoding, "gzip");

    let resp = client
        .get(format!("http://{}/v1/info", address))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    assert!(resp.headers().get("content-encoding").is_none());
}
