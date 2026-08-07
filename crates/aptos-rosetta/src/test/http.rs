// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! In-process HTTP tests for the (axum-based) Rosetta router.
//!
//! These exercise the router directly via `tower::ServiceExt::oneshot` — no
//! socket is bound and no full node is required. They validate response/CORS
//! parity for the offline-capable endpoints and the Rosetta "always-500" error
//! contract for endpoints that require a node connection.

use crate::{routes, test::test_rosetta_context, types::NetworkListResponse};
use axum::{
    body::Body,
    http::{header, Request, StatusCode},
};
use tower::ServiceExt;

/// Reads an entire axum response body into bytes.
async fn body_bytes(response: axum::response::Response) -> Vec<u8> {
    axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should be readable")
        .to_vec()
}

#[tokio::test]
async fn network_list_happy_path_returns_json() {
    let app = routes(test_rosetta_context().await);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/network/list")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
        Some("application/json")
    );

    let parsed: NetworkListResponse =
        serde_json::from_slice(&body_bytes(response).await).expect("valid NetworkListResponse");
    assert_eq!(parsed.network_identifiers.len(), 1);
    assert_eq!(parsed.network_identifiers[0].blockchain, "aptos");
}

#[tokio::test]
async fn node_offline_returns_500_with_error_body() {
    // The health check requires a REST client; the test context has none, so it
    // should surface `ApiError::NodeIsOffline` (code 14) as a Rosetta 500.
    let app = routes(test_rosetta_context().await);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/-/healthy")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let error: serde_json::Value =
        serde_json::from_slice(&body_bytes(response).await).expect("error body is JSON");
    assert_eq!(error["code"], 14);
    assert_eq!(error["retriable"], false);
}

#[tokio::test]
async fn cors_allows_any_origin() {
    let app = routes(test_rosetta_context().await);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/network/list")
                .header(header::ORIGIN, "https://example.com")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .and_then(|v| v.to_str().ok()),
        Some("*")
    );
}
