// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{current_function_name, tests::new_test_context};
use serde_json::json;

#[tokio::test]
async fn test_get_ledger_info() {
    let mut context = new_test_context(current_function_name!());
    let resp = context.get("/").await;
    context.check_golden_output(resp);
}

#[tokio::test]
async fn test_returns_not_found_for_the_invalid_path() {
    let mut context = new_test_context(current_function_name!());
    let resp = context.expect_status_code(404).get("/invalid_path").await;
    context.check_golden_output(resp);
}

#[tokio::test]
async fn test_return_bad_request_if_method_not_allowed() {
    let mut context = new_test_context(current_function_name!());
    let resp = context
        .expect_status_code(405)
        .post("/accounts/0x1/resources", json!({}))
        .await;
    context.check_golden_output(resp);
}

#[tokio::test]
async fn test_health_check() {
    let context = new_test_context(current_function_name!());
    let resp = context
        .reply(warp::test::request().method("GET").path("/-/healthy"))
        .await;
    assert_eq!(resp.status(), 200)
}

#[tokio::test]
async fn test_openapi_spec() {
    let context = new_test_context(current_function_name!());
    let paths = ["/openapi.yaml", "/spec.html"];
    for path in paths {
        let req = warp::test::request().method("GET").path(path);
        let resp = context.reply(req).await;
        assert_eq!(resp.status(), 200);
    }
}

#[tokio::test]
async fn test_cors() {
    let context = new_test_context(current_function_name!());
    let paths = ["/openapi.yaml", "/spec.html", "/", "/transactions"];
    for path in paths {
        let req = warp::test::request()
            .header("origin", "test")
            .header("Access-Control-Request-Headers", "Content-Type")
            .header("Access-Control-Request-Method", "POST")
            .method("OPTIONS")
            .path(path);
        let resp = context.reply(req).await;
        assert_eq!(resp.status(), 200);
        let cors_header = resp.headers().get("access-control-allow-origin").unwrap();
        assert_eq!(cors_header, "test");
    }
}

#[tokio::test]
async fn test_cors_forbidden() {
    let mut context = new_test_context(current_function_name!());
    let paths = ["/openapi.yaml", "/spec.html", "/", "/transactions"];
    for path in paths {
        let req = warp::test::request()
            .header("origin", "test")
            .header("Access-Control-Request-Headers", "Content-Type")
            .header("Access-Control-Request-Method", "PUT")
            .method("OPTIONS")
            .path(path);
        let resp = context.reply(req).await;
        assert_eq!(resp.status(), 403);
        let err: serde_json::Value = serde_json::from_slice(resp.body()).unwrap();
        context.check_golden_output(err);
    }
}

#[tokio::test]
async fn test_cors_on_non_200_responses() {
    let context = new_test_context(current_function_name!());
    // Preflight must work no matter what
    let preflight_req = warp::test::request()
        .header("origin", "test")
        .header("Access-Control-Request-Headers", "Content-Type")
        .header("Access-Control-Request-Method", "GET")
        .method("OPTIONS")
        .path("/accounts/nope/resources");
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
        .path("/accounts/nope/resources");
    let resp = context.reply(req).await;
    assert_eq!(resp.status(), 400);
    let cors_header = resp.headers().get("access-control-allow-origin").unwrap();
    assert_eq!(cors_header, "*");
}
