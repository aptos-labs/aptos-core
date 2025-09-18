// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::request_handler::is_known_path;
use aptos_inspection_service::utils::get_encoded_metrics;
use aptos_metrics_core::{exponential_buckets, register_histogram_vec, HistogramVec, TextEncoder};
use hyper::{header::CONTENT_TYPE, Body, Method, StatusCode};
use once_cell::sync::Lazy;
use std::{
    convert::Infallible,
    time::{Duration, Instant},
};

// Default port for the metrics service
pub const DEFAULT_METRICS_SERVER_PORT: u16 = 8080;

// Invalid request path label
const INVALID_PATH: &str = "invalid-path";

// Buckets for tracking latencies
static LATENCY_BUCKETS: Lazy<Vec<f64>> = Lazy::new(|| {
    exponential_buckets(
        /*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 24,
    )
    .unwrap()
});

// Histogram for tracking time taken to fetch external resources
static EXTERNAL_RESOURCE_FETCH_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "keyless_pepper_service_external_resource_fetch_seconds",
        "Time taken to fetch external resources",
        &["resource", "succeeded"],
        LATENCY_BUCKETS.clone()
    )
    .unwrap()
});

// Histogram for tracking time taken to fetch JWKs by issuer and result
static JWK_FETCH_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "keyless_pepper_service_jwk_fetch_seconds",
        "Time taken to fetch keyless pepper service jwks",
        &["issuer", "succeeded"],
        LATENCY_BUCKETS.clone()
    )
    .unwrap()
});

// Histogram for tracking time taken to handle pepper service requests
static REQUEST_HANDLING_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "keyless_pepper_service_request_handling_seconds",
        "Seconds taken to process pepper requests by scheme and result.",
        &["request_endpoint", "request_method", "response_code"],
        LATENCY_BUCKETS.clone()
    )
    .unwrap()
});

/// Handles incoming HTTP requests for the metrics server
pub async fn handle_metrics_request(
    request: hyper::Request<Body>,
) -> Result<hyper::Response<Body>, Infallible> {
    let response = match (request.method(), request.uri().path()) {
        (&Method::GET, "/metrics") => {
            let buffer = get_encoded_metrics(TextEncoder::new());
            hyper::Response::builder()
                .status(StatusCode::OK)
                .header(CONTENT_TYPE, "text/plain")
                .body(Body::from(buffer))
                .expect("Metric response should build")
        },
        _ => {
            let mut response = hyper::Response::new(Body::empty());
            *response.status_mut() = StatusCode::NOT_FOUND;
            response
        },
    };
    Ok(response)
}

/// Updates the external resource fetch metrics with the given data
pub fn update_external_resource_fetch_metrics(
    resource_name: &str,
    succeeded: bool,
    elapsed: Duration,
) {
    EXTERNAL_RESOURCE_FETCH_SECONDS
        .with_label_values(&[resource_name, &succeeded.to_string()])
        .observe(elapsed.as_secs_f64());
}

/// Updates the JWK fetch metrics with the given data
pub fn update_jwk_fetch_metrics(issuer: &str, succeeded: bool, elapsed: Duration) {
    JWK_FETCH_SECONDS
        .with_label_values(&[issuer, &succeeded.to_string()])
        .observe(elapsed.as_secs_f64());
}

/// Updates the request handling metrics with the given data
pub fn update_request_handling_metrics(
    request_endpoint: &str,
    request_method: Method,
    response_code: StatusCode,
    request_start_time: Instant,
) {
    // Calculate the elapsed time
    let elapsed = request_start_time.elapsed();

    // Determine the request endpoint to use in the metrics (i.e., replace
    // invalid paths with a fixed label to avoid high cardinality).
    let request_endpoint = if is_known_path(request_endpoint) {
        request_endpoint
    } else {
        INVALID_PATH
    };

    // Update the metrics
    REQUEST_HANDLING_SECONDS
        .with_label_values(&[
            request_endpoint,
            request_method.as_str(),
            &response_code.to_string(),
        ])
        .observe(elapsed.as_secs_f64());
}
