// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tower middleware stack for the v2 API.
//!
//! Provides logging/metrics, request ID, size limiting, CORS, and compression.
//! Phase 3: adds Prometheus metrics (request count, latency, in-flight gauge).

use super::{error::V2Error, metrics};
use axum::{
    extract::Request,
    http::{header, HeaderValue, Method},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::time::{Duration, Instant};
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

/// Middleware that logs request/response info and records Prometheus metrics.
pub async fn logging_layer(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let path = normalize_path(req.uri().path());
    let start = Instant::now();

    metrics::INFLIGHT_REQUESTS.inc();
    let resp = next.run(req).await;
    metrics::INFLIGHT_REQUESTS.dec();

    let elapsed = start.elapsed();
    let status = resp.status().as_u16();
    let status_str = status.to_string();

    // Record Prometheus metrics.
    metrics::REQUEST_DURATION
        .with_label_values(&[method.as_str(), &path, &status_str])
        .observe(elapsed.as_secs_f64());
    metrics::REQUEST_COUNT
        .with_label_values(&[method.as_str(), &path, &status_str])
        .inc();

    aptos_logger::info!(
        method = %method,
        path = %path,
        status = status,
        elapsed_ms = elapsed.as_millis() as u64,
        "[v2] request completed"
    );

    resp
}

/// Normalize a request path into a pattern suitable for metric labels.
/// Replaces dynamic segments (hex addresses, hashes, numbers) with
/// placeholders to avoid metric cardinality explosion.
///
/// Examples:
///   `/v2/accounts/0x1abc/resources` → `/v2/accounts/:address/resources`
///   `/v2/transactions/0xdeadbeef...` → `/v2/transactions/:hash`
///   `/v2/blocks/42` → `/v2/blocks/:height`
fn normalize_path(path: &str) -> String {
    let parts: Vec<&str> = path.split('/').collect();
    let mut normalized = Vec::with_capacity(parts.len());

    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            normalized.push(*part);
            continue;
        }

        // Check if previous segment gives context about what this is.
        let prev = if i > 0 { parts[i - 1] } else { "" };

        let replacement = match prev {
            "accounts" => Some(":address"),
            "blocks" if *part != "latest" && *part != "by_version" => Some(":height"),
            "by_version" => Some(":version"),
            "tables" => Some(":table_handle"),
            "transactions"
                if part.starts_with("0x")
                    || (part.len() == 64 && part.chars().all(|c| c.is_ascii_hexdigit())) =>
            {
                Some(":hash")
            },
            "balance" => Some(":asset_type"),
            "events" => Some(":creation_number"),
            "module" => Some(":module_name"),
            "resource" => Some(":resource_type"),
            _ => {
                // Also catch standalone hex addresses and numeric IDs.
                if (part.starts_with("0x") && part.len() > 6)
                    || (part.chars().all(|c| c.is_ascii_digit()) && part.len() > 2)
                {
                    Some(":param")
                } else {
                    None
                }
            },
        };

        normalized.push(replacement.unwrap_or(part));
    }

    normalized.join("/")
}

/// Middleware that enforces a per-request timeout.
///
/// If the downstream handler does not complete within `timeout_ms` milliseconds,
/// a 408 Request Timeout response is returned. A value of 0 disables the timeout.
///
/// This is applied inside the logging layer so that timeouts are always recorded
/// in metrics and logs.
pub async fn timeout_layer(timeout_ms: u64, req: Request, next: Next) -> Response {
    if timeout_ms == 0 {
        return next.run(req).await;
    }

    let method = req.method().clone();
    let path = normalize_path(req.uri().path());

    match tokio::time::timeout(Duration::from_millis(timeout_ms), next.run(req)).await {
        Ok(response) => response,
        Err(_) => {
            metrics::REQUEST_TIMEOUTS
                .with_label_values(&[method.as_str(), &path])
                .inc();
            V2Error::request_timeout(timeout_ms).into_response()
        },
    }
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
        .allow_headers([header::CONTENT_TYPE, header::ACCEPT, header::AUTHORIZATION])
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path_accounts() {
        assert_eq!(
            normalize_path("/v2/accounts/0x1/resources"),
            "/v2/accounts/:address/resources"
        );
        assert_eq!(
            normalize_path("/v2/accounts/0xabcdef1234567890/resource/0x1::coin::CoinStore"),
            "/v2/accounts/:address/resource/:resource_type"
        );
    }

    #[test]
    fn test_normalize_path_blocks() {
        assert_eq!(normalize_path("/v2/blocks/latest"), "/v2/blocks/latest");
        assert_eq!(normalize_path("/v2/blocks/12345"), "/v2/blocks/:height");
    }

    #[test]
    fn test_normalize_path_transactions() {
        assert_eq!(
            normalize_path("/v2/transactions/0xdeadbeef1234567890abcdef1234567890abcdef1234567890abcdef12345678"),
            "/v2/transactions/:hash"
        );
        assert_eq!(normalize_path("/v2/transactions"), "/v2/transactions");
    }

    #[test]
    fn test_normalize_path_static_routes() {
        assert_eq!(normalize_path("/v2/health"), "/v2/health");
        assert_eq!(normalize_path("/v2/info"), "/v2/info");
        assert_eq!(normalize_path("/v2/view"), "/v2/view");
        assert_eq!(normalize_path("/v2/batch"), "/v2/batch");
        assert_eq!(normalize_path("/v2/ws"), "/v2/ws");
        assert_eq!(normalize_path("/v2/spec.json"), "/v2/spec.json");
    }
}
