// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_inspection_service::utils::get_encoded_metrics;
use aptos_metrics_core::{exponential_buckets, register_histogram_vec, HistogramVec, TextEncoder};
use hyper::{
    header::CONTENT_TYPE,
    service::{make_service_fn, service_fn},
    Body, Method, Server, StatusCode,
};
use once_cell::sync::Lazy;
use std::{convert::Infallible, net::SocketAddr};

pub static JWK_FETCH_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "keyless_pepper_service_jwk_fetch_seconds",
        "Seconds taken to process pepper requests by scheme and result.",
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

pub fn start_metric_server() {
    let _handle = tokio::spawn(async move {
        let addr = SocketAddr::from(([0, 0, 0, 0], 8080));

        let make_svc =
            make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle_request)) });

        let server = Server::bind(&addr).serve(make_svc);

        if let Err(e) = server.await {
            eprintln!("server error: {}", e);
        }
    });
}

async fn handle_request(req: hyper::Request<Body>) -> Result<hyper::Response<Body>, Infallible> {
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
