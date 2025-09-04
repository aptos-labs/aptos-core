// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    gather_metrics::{gather_metrics, NUM_METRICS},
    MetricsServerConfig,
};
use anyhow::Result;
use poem::{
    handler, http::Method, listener::TcpListener, middleware::Cors, EndpointExt, Route, Server,
};
use prometheus::{Encoder, TextEncoder};
use std::future::Future;

pub fn encode_metrics(encoder: impl Encoder) -> Vec<u8> {
    let metric_families = gather_metrics();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();

    NUM_METRICS
        .with_label_values(&["total_bytes"])
        .inc_by(buffer.len() as u64);
    buffer
}

#[handler]
fn metrics() -> Vec<u8> {
    encode_metrics(TextEncoder)
}

pub fn run_metrics_server(
    config: MetricsServerConfig,
) -> impl Future<Output = Result<(), std::io::Error>> {
    let cors = Cors::new().allow_methods(vec![Method::GET]);
    Server::new(TcpListener::bind((
        config.listen_address.clone(),
        config.listen_port,
    )))
    .run(Route::new().at("/metrics", metrics).with(cors))
}
