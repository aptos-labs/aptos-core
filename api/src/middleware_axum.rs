// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::metrics::{HISTOGRAM, POST_BODY_BYTES, REQUEST_SOURCE_CLIENT, RESPONSE_STATUS};
use aptos_api_types::{AptosError, AptosErrorCode, TRACEPARENT, X_APTOS_CLIENT};
use aptos_logger::{
    debug, info,
    prelude::{sample, SampleRate},
    warn, Schema,
};
use axum::{
    extract::Request,
    http::{header, Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use once_cell::sync::Lazy;
use regex::Regex;
use std::time::Duration;

const REQUEST_SOURCE_CLIENT_UNKNOWN: &str = "unknown";
static REQUEST_SOURCE_CLIENT_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"aptos-[a-zA-Z\-]+/[0-9A-Za-z\.\-]+").unwrap());

fn extract_trace_context(req: &Request) -> (Option<String>, Option<String>) {
    if let Some(traceparent) = req.headers().get(TRACEPARENT) {
        if let Ok(traceparent_str) = traceparent.to_str() {
            let parts: Vec<&str> = traceparent_str.split('-').collect();
            if parts.len() == 4 && parts[1].len() == 32 && parts[2].len() == 16 {
                return (Some(parts[1].to_string()), Some(parts[2].to_string()));
            }
        }
    }

    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let trace_id = format!("{:032x}", nanos);
    let span_id = format!("{:016x}", nanos & 0xFFFFFFFFFFFFFFFF);
    (Some(trace_id), Some(span_id))
}

pub async fn logging_middleware(req: Request, next: Next) -> Response {
    let start = std::time::Instant::now();

    let (trace_id, span_id) = extract_trace_context(&req);

    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let route_pattern = normalize_path_for_metrics(&path);
    let referer = req
        .headers()
        .get(header::REFERER)
        .and_then(|v| v.to_str().ok().map(|v| v.to_string()));
    let user_agent = req
        .headers()
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok().map(|v| v.to_string()));
    let aptos_client = req
        .headers()
        .get(X_APTOS_CLIENT)
        .and_then(|v| v.to_str().ok().map(|v| v.to_string()));
    let forwarded = req
        .headers()
        .get(header::FORWARDED)
        .and_then(|v| v.to_str().ok().map(|v| v.to_string()));
    let content_length = req
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok().map(|v| v.to_string()));

    let response = next.run(req).await;

    let elapsed = start.elapsed();
    let status = response.status().as_u16();

    let log = HttpRequestLog {
        remote_addr: None,
        method: method.clone(),
        path,
        status,
        referer,
        user_agent,
        aptos_client: aptos_client.clone(),
        elapsed,
        forwarded,
        content_length: content_length.clone(),
        trace_id,
        span_id,
    };

    if status >= 500 {
        sample!(SampleRate::Duration(Duration::from_secs(1)), warn!(log));
    } else if status >= 400 {
        sample!(SampleRate::Duration(Duration::from_secs(60)), info!(log));
    } else {
        sample!(SampleRate::Duration(Duration::from_secs(1)), debug!(log));
    }

    RESPONSE_STATUS
        .with_label_values(&[status.to_string().as_str()])
        .observe(elapsed.as_secs_f64());

    let operation_id = response
        .extensions()
        .get::<OperationId>()
        .map(|op| op.0.as_str())
        .unwrap_or_else(|| route_pattern.as_str());

    HISTOGRAM
        .with_label_values(&[method.as_str(), operation_id, status.to_string().as_str()])
        .observe(elapsed.as_secs_f64());

    REQUEST_SOURCE_CLIENT
        .with_label_values(&[
            determine_request_source_client(&aptos_client),
            operation_id,
            status.to_string().as_str(),
        ])
        .inc();

    if method == Method::POST {
        if let Some(length) = content_length.and_then(|l| l.parse::<u32>().ok()) {
            POST_BODY_BYTES
                .with_label_values(&[operation_id, status.to_string().as_str()])
                .observe(length as f64);
        }
    }

    response
}

#[derive(Clone)]
pub struct OperationId(pub String);

pub async fn post_size_limit_middleware(
    max_size: u64,
    req: Request,
    next: Next,
) -> Result<Response, Response> {
    if req.method() != Method::POST {
        return Ok(next.run(req).await);
    }

    let content_length = req
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok());

    match content_length {
        None => {
            let error = AptosError::new_with_error_code(
                "missing `Content-Length` header",
                AptosErrorCode::WebFrameworkError,
            );
            let json = serde_json::to_vec(&error).unwrap_or_default();
            Err((
                StatusCode::LENGTH_REQUIRED,
                [(header::CONTENT_TYPE, "application/json")],
                json,
            )
                .into_response())
        },
        Some(size) if size > max_size => {
            let error = AptosError::new_with_error_code(
                "Payload too large",
                AptosErrorCode::WebFrameworkError,
            );
            let json = serde_json::to_vec(&error).unwrap_or_default();
            Err((
                StatusCode::PAYLOAD_TOO_LARGE,
                [(header::CONTENT_TYPE, "application/json")],
                json,
            )
                .into_response())
        },
        _ => Ok(next.run(req).await),
    }
}

fn determine_request_source_client(aptos_client: &Option<String>) -> &str {
    let aptos_client = match aptos_client {
        Some(aptos_client) => aptos_client,
        None => return REQUEST_SOURCE_CLIENT_UNKNOWN,
    };

    match REQUEST_SOURCE_CLIENT_REGEX.find_iter(aptos_client).last() {
        Some(capture) => capture.as_str(),
        None => REQUEST_SOURCE_CLIENT_UNKNOWN,
    }
}

/// Normalizes a raw request path into a stable route template for metrics labels.
/// Replaces variable path segments (hex addresses, numeric IDs, hashes) with
/// placeholders to prevent cardinality explosion in Prometheus metrics.
fn normalize_path_for_metrics(path: &str) -> String {
    static HEX_ADDR: Lazy<Regex> = Lazy::new(|| Regex::new(r"0x[0-9a-fA-F]+").unwrap());
    static NUMERIC: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[0-9]+$").unwrap());

    let segments: Vec<&str> = path.split('/').collect();
    let mut result = Vec::with_capacity(segments.len());

    for (i, seg) in segments.iter().enumerate() {
        if seg.is_empty() {
            result.push(*seg);
            continue;
        }
        if HEX_ADDR.is_match(seg) {
            result.push(":address");
        } else if NUMERIC.is_match(seg) {
            // Numeric segments after known path prefixes get named placeholders
            let prev = if i > 0 { segments[i - 1] } else { "" };
            match prev {
                "by_height" => result.push(":block_height"),
                "by_version" => result.push(":version"),
                "events" => result.push(":creation_number"),
                _ => result.push(":id"),
            }
        } else {
            result.push(seg);
        }
    }
    result.join("/")
}

#[derive(Schema)]
pub struct HttpRequestLog {
    #[schema(display)]
    remote_addr: Option<std::net::SocketAddr>,
    #[schema(display)]
    method: Method,
    path: String,
    pub status: u16,
    referer: Option<String>,
    user_agent: Option<String>,
    aptos_client: Option<String>,
    #[schema(debug)]
    pub elapsed: std::time::Duration,
    forwarded: Option<String>,
    content_length: Option<String>,
    trace_id: Option<String>,
    span_id: Option<String>,
}
