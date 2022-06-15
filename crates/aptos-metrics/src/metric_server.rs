// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    gather_metrics, json_encoder::JsonEncoder, system_information::get_system_information,
    system_metrics::update_system_metrics, NUM_METRICS,
};
use futures::future;
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, Server, StatusCode,
};
use prometheus::{
    proto::{MetricFamily, MetricType},
    Encoder, TextEncoder,
};
use std::{
    collections::HashMap,
    net::{SocketAddr, ToSocketAddrs},
    thread,
};
use tokio::runtime;

fn encode_metrics(encoder: impl Encoder, whitelist: &'static [&'static str]) -> Vec<u8> {
    let mut metric_families = gather_metrics();
    if !whitelist.is_empty() {
        metric_families = whitelist_metrics(metric_families, whitelist);
    }
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();

    NUM_METRICS
        .with_label_values(&["total_bytes"])
        .inc_by(buffer.len() as u64);
    buffer
}

fn get_metrics(fams: Vec<MetricFamily>) -> HashMap<String, String> {
    // TODO: use an existing metric encoder (same as used by
    // prometheus/metric-server)
    let mut all_metrics = HashMap::new();
    for metric_family in fams {
        let values: Vec<_> = match metric_family.get_field_type() {
            MetricType::COUNTER => metric_family
                .get_metric()
                .iter()
                .map(|m| m.get_counter().get_value().to_string())
                .collect(),
            MetricType::GAUGE => metric_family
                .get_metric()
                .iter()
                .map(|m| m.get_gauge().get_value().to_string())
                .collect(),
            MetricType::SUMMARY => panic!("Unsupported Metric 'SUMMARY'"),
            MetricType::UNTYPED => panic!("Unsupported Metric 'UNTYPED'"),
            MetricType::HISTOGRAM => metric_family
                .get_metric()
                .iter()
                .map(|m| m.get_histogram().get_sample_count().to_string())
                .collect(),
        };
        let metric_names = metric_family.get_metric().iter().map(|m| {
            let label_strings: Vec<String> = m
                .get_label()
                .iter()
                .map(|l| format!("{}={}", l.get_name(), l.get_value()))
                .collect();
            let labels_string = format!("{{{}}}", label_strings.join(","));
            format!("{}{}", metric_family.get_name(), labels_string)
        });

        for (name, value) in metric_names.zip(values.into_iter()) {
            all_metrics.insert(name, value);
        }
    }

    all_metrics
}

pub fn get_all_metrics() -> HashMap<String, String> {
    let all_metric_families = gather_metrics();
    get_metrics(all_metric_families)
}

// filtering metrics from the prometheus collections
// only return the whitelisted metrics
fn whitelist_metrics(
    metric_families: Vec<MetricFamily>,
    whitelist: &'static [&'static str],
) -> Vec<MetricFamily> {
    let mut whitelist_metrics = Vec::new();
    for mf in metric_families {
        let name = mf.get_name();
        if whitelist.contains(&name) {
            whitelist_metrics.push(mf.clone());
        }
    }
    whitelist_metrics
}

async fn serve_metrics(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let mut resp = Response::new(Body::empty());
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/metrics") => {
            // Prometheus server expects metrics to be on host:port/metrics
            let encoder = TextEncoder::new();
            let buffer = encode_metrics(encoder, &[]);
            *resp.body_mut() = Body::from(buffer);
        }
        // Expose system information to host:port/system_information
        (&Method::GET, "/system_information") => {
            let system_information = get_system_information();
            let encoded_information = serde_json::to_string(&system_information).unwrap();
            *resp.body_mut() = Body::from(encoded_information);
        }
        (&Method::GET, "/counters") => {
            // Json encoded aptos_metrics;
            let encoder = JsonEncoder;
            let buffer = encode_metrics(encoder, &[]);
            *resp.body_mut() = Body::from(buffer);
        }
        _ => {
            *resp.status_mut() = StatusCode::NOT_FOUND;
        }
    };

    Ok(resp)
}

pub fn start_server(host: String, port: u16) {
    // Only called from places that guarantee that host is parsable, but this must be assumed.
    let addr: SocketAddr = (host.as_str(), port)
        .to_socket_addrs()
        .unwrap_or_else(|_| unreachable!("Failed to parse {}:{} as address", host, port))
        .next()
        .unwrap();

    // Spawn the metric server
    thread::spawn(move || {
        let make_service =
            make_service_fn(|_| future::ok::<_, hyper::Error>(service_fn(serve_metrics)));

        let rt = runtime::Builder::new_current_thread()
            .enable_io()
            .build()
            .unwrap();
        rt.block_on(async {
            let server = Server::bind(&addr).serve(make_service);
            server.await
        })
        .unwrap();
    });

    // Update system metrics on startup
    thread::spawn(move || {
        // TODO(joshlind): this doesn't need to be done here.
        update_system_metrics();
    });
}
