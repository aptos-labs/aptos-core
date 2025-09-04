// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::new_test_context;
use velor_api_test_context::current_function_name;
use serde_json::json;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_get_index() {
    let mut context = new_test_context(current_function_name!());
    let resp = context.get("/").await;
    context.check_golden_output(resp);
}

// TODO: Un-ignore this pending https://github.com/poem-web/poem/issues/343.
#[ignore]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_returns_not_found_for_the_invalid_path() {
    let mut context = new_test_context(current_function_name!());
    let resp = context.expect_status_code(404).get("/invalid_path").await;
    context.check_golden_output(resp);
}

// TODO: Un-ignore this pending https://github.com/poem-web/poem/issues/343.
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
    let resp = context
        .reply(warp::test::request().method("GET").path("/v1/-/healthy"))
        .await;
    assert_eq!(resp.status(), 200)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_openapi_spec() {
    let context = new_test_context(current_function_name!());
    let paths = ["/spec.yaml", "/spec.json", "/spec"];
    for path in paths {
        let req = warp::test::request()
            .method("GET")
            .path(&format!("/v1{}", path));
        let resp = context.reply(req).await;
        assert_eq!(resp.status(), 200);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_cors() {
    let context = new_test_context(current_function_name!());
    let paths = ["/spec.yaml", "/spec", "/", "/transactions"];
    for path in paths {
        let req = warp::test::request()
            .header("origin", "test")
            .header("Access-Control-Request-Headers", "Content-Type")
            .header("Access-Control-Request-Method", "POST")
            .method("OPTIONS")
            .path(&format!("/v1{}", path));
        let resp = context.reply(req).await;
        assert_eq!(resp.status(), 200);
        let cors_header = resp.headers().get("access-control-allow-origin").unwrap();
        assert_eq!(cors_header, "test");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_cors_forbidden() {
    let mut context = new_test_context(current_function_name!());
    let paths = ["/spec.yaml", "/spec", "/", "/transactions"];
    for path in paths {
        let req = warp::test::request()
            .header("origin", "test")
            .header("Access-Control-Request-Headers", "Content-Type")
            .header("Access-Control-Request-Method", "PUT")
            .method("OPTIONS")
            .path(&format!("/v1{}", path));
        let resp = context.reply(req).await;
        assert_eq!(resp.status(), 403);
        let err: serde_json::Value = serde_json::from_slice(resp.body()).unwrap();
        context.check_golden_output(err);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_cors_on_non_200_responses() {
    let context = new_test_context(current_function_name!());
    // Preflight must work no matter what
    let preflight_req = warp::test::request()
        .header("origin", "test")
        .header("Access-Control-Request-Headers", "Content-Type")
        .header("Access-Control-Request-Method", "GET")
        .method("OPTIONS")
        .path("/v1/accounts/nope/resources");
    let preflight_resp = context.reply(preflight_req).await;
    assert_eq!(preflight_resp.status(), 200);
    let cors_header = preflight_resp
        .headers()
        .get("access-control-allow-origin")
        .unwrap();
    assert_eq!(cors_header, "test");

    // Actual request should also have correct CORS headers set
    let req = warp::test::request()
        .header("origin", "test")
        .header("Access-Control-Request-Headers", "Content-Type")
        .header("Access-Control-Request-Method", "GET")
        .method("GET")
        .path("/v1/accounts/nope/resources");
    let resp = context.reply(req).await;
    assert_eq!(resp.status(), 400);
    let cors_header = resp.headers().get("access-control-allow-origin").unwrap();
    assert_eq!(cors_header, "test");
}

/// Verifies gzip compression is applied when accept-encoding header is present
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_compression_middleware() {
    let context = new_test_context(current_function_name!());

    let req = warp::test::request()
        .header("accept-encoding", "gzip")
        .method("GET")
        .path("/v1/info");

    let resp = context.reply(req).await;
    assert_eq!(resp.status(), 200);
    let content_encoding = resp.headers().get("content-encoding").unwrap();
    assert_eq!(content_encoding, "gzip");

    let req = warp::test::request().method("GET").path("/v1/info");
    let resp = context.reply(req).await;
    assert_eq!(resp.status(), 200);
    assert!(resp.headers().get("content-encoding").is_none());
}
