// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{
    register_int_counter, register_int_counter_vec, register_int_gauge_vec, IntCounter,
    IntCounterVec, IntGaugeVec, TextEncoder,
};
use http::StatusCode;
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, Server,
};
use inspection_service::inspection_service::encode_metrics;
use once_cell::sync::Lazy;
use std::{
    convert::Infallible,
    net::{SocketAddr, ToSocketAddrs},
    thread,
};
use tokio::runtime;

/// Number of times a given processor has been invoked
pub static PROCESSOR_INVOCATIONS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "indexer_processor_invocation_count",
        "Number of times a given processor has been invoked",
        &["processor_name"]
    )
    .unwrap()
});

/// Number of times any given processor has raised an error
pub static PROCESSOR_ERRORS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "indexer_processor_error_count",
        "Number of times a given processor has raised an error",
        &["processor_name"]
    )
    .unwrap()
});

/// Number of times any given processor has completed successfully
pub static PROCESSOR_SUCCESSES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "indexer_processor_success_count",
        "Number of times a given processor has completed successfully",
        &["processor_name"]
    )
    .unwrap()
});

/// Number of times the connection pool has timed out when trying to get a connection
pub static UNABLE_TO_GET_CONNECTION: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "indexer_connection_pool_err",
        "Number of times the connection pool has timed out when trying to get a connection"
    )
    .unwrap()
});

/// Number of times the connection pool got a connection
pub static GOT_CONNECTION: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "indexer_connection_pool_ok",
        "Number of times the connection pool got a connection"
    )
    .unwrap()
});

/// Max block processed
pub static LATEST_PROCESSED_BLOCK: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "indexer_processor_latest_block",
        "Latest block a processor has fully consumed",
        &["processor_name"]
    )
    .unwrap()
});

/// Max version processed
pub static LATEST_PROCESSED_VERSION: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "indexer_processor_latest_version",
        "Latest version a processor has fully consumed",
        &["processor_name"]
    )
    .unwrap()
});

pub fn start_inspection_service(service_address: &str, service_port: u16) {
    // Only called from places that guarantee that host is parsable, but this must be assumed.
    let addr: SocketAddr = (service_address, service_port)
        .to_socket_addrs()
        .unwrap_or_else(|_| {
            unreachable!(
                "Failed to parse {}:{} as address",
                service_address, service_port
            )
        })
        .next()
        .unwrap();

    // Spawn the server
    thread::spawn(move || {
        let make_service =
            make_service_fn(
                move |_conn| async move { Ok::<_, Infallible>(service_fn(serve_requests)) },
            );

        let runtime = runtime::Builder::new_current_thread()
            .enable_io()
            .disable_lifo_slot()
            .build()
            .unwrap();
        runtime
            .block_on(async {
                let server = Server::bind(&addr).serve(make_service);
                server.await
            })
            .unwrap();
    });
}

async fn serve_requests(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let mut resp = Response::new(Body::empty());
    match (req.method(), req.uri().path()) {
        // Exposes text encoded metrics
        (&Method::GET, "/metrics") => {
            let encoder = TextEncoder::new();
            let buffer = encode_metrics(encoder);
            *resp.body_mut() = Body::from(buffer);
        }
        _ => {
            *resp.status_mut() = StatusCode::NOT_FOUND;
        }
    };
    Ok(resp)
}

// pub fn encode_metrics(encoder: impl Encoder) -> Vec<u8> {
//     let metric_families = gather_metrics();
//     let mut buffer = vec![];
//     encoder.encode(&metric_families, &mut buffer).unwrap();
//     buffer
// }

// pub fn gather_metrics() -> Vec<prometheus::proto::MetricFamily> {
//     let metric_families = aptos_metrics_core::gather();
//     let mut total: u64 = 0;
//     let mut families_over_1000: u64 = 0;

//     // Take metrics of metric gathering so we know possible overhead of this process
//     for metric_family in &metric_families {
//         let family_count = metric_family.get_metric().len();
//         if family_count > 1000 {
//             families_over_1000 = families_over_1000.saturating_add(1);
//             let name = metric_family.get_name();
//             aptos_logger::warn!(
//                 count = family_count,
//                 metric_family = name,
//                 "Metric Family '{}' over 1000 dimensions '{}'",
//                 name,
//                 family_count
//             );
//         }
//         total = total.saturating_add(family_count as u64);
//     }
//     metric_families
// }
