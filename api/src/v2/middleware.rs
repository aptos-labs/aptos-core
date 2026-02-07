// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-core/blob/main/LICENSE

//! Tower middleware stack for the v2 API.
//!
//! Provides logging/metrics, request ID, size limiting, CORS, and compression.

use axum::{
    extract::Request,
    http::{header, HeaderValue, Method},
    middleware::Next,
    response::Response,
};
use std::time::Instant;
use uuid::Uuid;

/// Middleware that attaches a unique request ID to every response.
pub async fn request_id_layer(mut req: Request, next: Next) -> Response {
    let request_id = Uuid::new_v4().to_string();
    req.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(&request_id).unwrap_or_else(|_| HeaderValue::from_static("")),
    );
    let mut resp = next.run(req).await;
    resp.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(&request_id).unwrap_or_else(|_| HeaderValue::from_static("")),
    );
    resp
}

/// Middleware that logs request/response metrics.
pub async fn logging_layer(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let start = Instant::now();

    let resp = next.run(req).await;

    let elapsed = start.elapsed();
    let status = resp.status().as_u16();

    aptos_logger::info!(
        method = %method,
        path = %path,
        status = status,
        elapsed_ms = elapsed.as_millis() as u64,
        "[v2] request completed"
    );

    resp
}

/// Build the CORS layer for v2.
pub fn cors_layer() -> tower_http::cors::CorsLayer {
    tower_http::cors::CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::CONTENT_TYPE,
            header::ACCEPT,
            header::AUTHORIZATION,
        ])
        .expose_headers([header::HeaderName::from_static("x-request-id")])
        .max_age(std::time::Duration::from_secs(3600))
}

/// Build the compression layer for v2.
pub fn compression_layer() -> tower_http::compression::CompressionLayer {
    tower_http::compression::CompressionLayer::new()
}

/// Build the request body size limit layer.
pub fn size_limit_layer(max_bytes: usize) -> tower_http::limit::RequestBodyLimitLayer {
    tower_http::limit::RequestBodyLimitLayer::new(max_bytes)
}
