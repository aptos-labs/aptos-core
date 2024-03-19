// Copyright © Aptos Foundation

use std::convert::Infallible;
use std::net::SocketAddr;
use hyper::{Body, Method, Server, StatusCode};
use hyper::header::CONTENT_TYPE;
use hyper::service::{make_service_fn, service_fn};
use once_cell::sync::Lazy;
use aptos_inspection_service::utils::get_encoded_metrics;
use aptos_metrics_core::{HistogramVec, register_histogram_vec, register_int_counter_vec, TextEncoder};

pub static REQUEST_HANDLING_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "keyless_pepper_request_handling_seconds",
        "How long it takes to process a pepper request",
        &["pepper_scheme", "result"],
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
        (&Method::GET, "/") => {
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
