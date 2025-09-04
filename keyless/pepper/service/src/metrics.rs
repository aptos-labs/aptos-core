// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_inspection_service::utils::get_encoded_metrics;
use aptos_metrics_core::{exponential_buckets, register_histogram_vec, HistogramVec, TextEncoder};
use hyper::{header::CONTENT_TYPE, Body, Method, StatusCode};
use once_cell::sync::Lazy;
use std::convert::Infallible;

// Default port for the metrics service
pub const DEFAULT_METRICS_SERVER_PORT: u16 = 8080;

// Histogram for tracking time taken to fetch JWKs by issuer and result
pub static JWK_FETCH_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "keyless_pepper_service_jwk_fetch_seconds",
        "Time taken to fetch keyless pepper service jwks",
        &["issuer", "succeeded"],
        exponential_buckets(/*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 24).unwrap()
    )
    .unwrap()
});

pub static REQUEST_HANDLING_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "keyless_pepper_request_handling_seconds",
        "Seconds taken to process pepper requests by scheme and result.",
        &["pepper_scheme", "is_ok"],
        exponential_buckets(/*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 24).unwrap()
    )
    .unwrap()
});

pub async fn handle_request(
    req: hyper::Request<Body>,
) -> Result<hyper::Response<Body>, Infallible> {
    let response = match (req.method(), req.uri().path()) {
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
