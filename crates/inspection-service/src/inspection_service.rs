// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{gather_metrics, json_encoder::JsonEncoder, NUM_METRICS};
use aptos_build_info::build_information;
use aptos_config::config::NodeConfig;
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
    convert::Infallible,
    net::{SocketAddr, ToSocketAddrs},
    thread,
};
use tokio::runtime;

// The message displayed when the endpoint is disabled.
const DISABLED_ENDPOINT_MESSAGE: &str =
    "This endpoint is disabled! Enable it in the InspectionServiceConfig.";

pub fn encode_metrics(encoder: impl Encoder) -> Vec<u8> {
    let metric_families = gather_metrics();
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

async fn serve_requests(
    req: Request<Body>,
    node_config: NodeConfig,
) -> Result<Response<Body>, hyper::Error> {
    let mut resp = Response::new(Body::empty());
    match (req.method(), req.uri().path()) {
        // Expose the node configuration
        (&Method::GET, "/configuration") => {
            if node_config.inspection_service.expose_configuration {
                // We format the configuration using debug formatting. This is important to
                // prevent secret/private keys from being serialized and leaked (i.e.,
                // all secret keys are marked with SilentDisplay and SilentDebug).
                let encoded_configuration = format!("{:?}", node_config);
                *resp.body_mut() = Body::from(encoded_configuration);
            } else {
                *resp.body_mut() = Body::from(DISABLED_ENDPOINT_MESSAGE);
            }
        }
        // Exposes JSON encoded metrics
        (&Method::GET, "/json_metrics") => {
            let encoder = JsonEncoder;
            let buffer = encode_metrics(encoder);
            *resp.body_mut() = Body::from(buffer);
        }
        // Exposes text encoded metrics
        (&Method::GET, "/metrics") => {
            let encoder = TextEncoder::new();
            let buffer = encode_metrics(encoder);
            *resp.body_mut() = Body::from(buffer);
        }
        // Exposes forge encoded metrics (this is currently only used by forge).
        (&Method::GET, "/forge_metrics") => {
            let metrics = get_all_metrics();
            let encoded_metrics = serde_json::to_string(&metrics).unwrap();
            *resp.body_mut() = Body::from(encoded_metrics);
        }
        // Expose the system and build information
        (&Method::GET, "/system_information") => {
            if node_config.inspection_service.expose_system_information {
                let mut system_information =
                    aptos_telemetry::system_information::get_system_information();
                let build_info = build_information!();
                system_information.extend(build_info);
                let encoded_information = serde_json::to_string(&system_information).unwrap();
                *resp.body_mut() = Body::from(encoded_information);
            } else {
                *resp.body_mut() = Body::from(DISABLED_ENDPOINT_MESSAGE);
            }
        }
        _ => {
            *resp.status_mut() = StatusCode::NOT_FOUND;
        }
    };

    Ok(resp)
}

pub fn start_inspection_service(node_config: NodeConfig) {
    // Fetch the service port and address
    let service_port = node_config.inspection_service.port;
    let service_address = node_config.inspection_service.address.clone();

    // Only called from places that guarantee that host is parsable, but this must be assumed.
    let addr: SocketAddr = (service_address.as_str(), service_port)
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
        let make_service = make_service_fn(move |_conn| {
            let node_config = node_config.clone();
            async move {
                Ok::<_, Infallible>(service_fn(move |request| {
                    serve_requests(request, node_config.clone())
                }))
            }
        });

        let runtime = runtime::Builder::new_current_thread()
            .thread_name("inspection")
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
