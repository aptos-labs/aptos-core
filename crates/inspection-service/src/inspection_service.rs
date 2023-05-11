// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{gather_metrics, json_encoder::JsonEncoder, NUM_METRICS};
use aptos_build_info::build_information;
use aptos_config::{config::NodeConfig, network_id::NetworkId};
use aptos_logger::debug;
use aptos_network::application::storage::PeersAndMetadata;
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
    string::String,
    sync::Arc,
    thread,
};

// TODO: refactor this crate and split up logic into different files

// Useful string constants
const CONTENT_TYPE_JSON: &str = "application/json";
const CONTENT_TYPE_TEXT: &str = "text/plain";
const CONFIGURATION_DISABLED_MESSAGE: &str =
    "This endpoint is disabled! Enable it in the node config at inspection_service.expose_configuration: true";
const PEER_INFO_DISABLED_MESSAGE: &str =
    "This endpoint is disabled! Enable it in the node config at inspection_service.expose_peer_information: true";
const SYS_INFO_DISABLED_MESSAGE: &str =
    "This endpoint is disabled! Enable it in the node config at inspection_service.expose_system_information: true";
const HEADER_CONTENT_TYPE: &str = "Content-Type";
const INVALID_ENDPOINT_MESSAGE: &str = "The requested endpoint is invalid!";
const UNEXPECTED_ERROR_MESSAGE: &str = "An unexpected error was encountered!";

// The list of endpoints offered by the inspection service
const CONFIGURATION_PATH: &str = "/configuration";
const FORGE_METRICS_PATH: &str = "/forge_metrics";
const INDEX_PATH: &str = "/";
const JSON_METRICS_PATH: &str = "/json_metrics";
const METRICS_PATH: &str = "/metrics";
const PEER_INFORMATION_PATH: &str = "/peer_information";
const SYSTEM_INFORMATION_PATH: &str = "/system_information";

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
    peers_and_metadata: Arc<PeersAndMetadata>,
) -> Result<Response<Body>, hyper::Error> {
    // Process the request and get the response components
    let (status_code, body, content_type) = match req.uri().path() {
        CONFIGURATION_PATH => {
            // /configuration
            // Exposes the node configuration
            if node_config.inspection_service.expose_configuration {
                // We format the configuration using debug formatting. This is important to
                // prevent secret/private keys from being serialized and leaked (i.e.,
                // all secret keys are marked with SilentDisplay and SilentDebug).
                let encoded_configuration = format!("{:?}", node_config);
                (
                    StatusCode::OK,
                    Body::from(encoded_configuration),
                    CONTENT_TYPE_TEXT,
                )
            } else {
                (
                    StatusCode::FORBIDDEN,
                    Body::from(CONFIGURATION_DISABLED_MESSAGE),
                    CONTENT_TYPE_TEXT,
                )
            }
        },
        FORGE_METRICS_PATH => {
            // /forge_metrics
            // Exposes forge encoded metrics
            let metrics = get_all_metrics();
            let encoded_metrics = serde_json::to_string(&metrics).unwrap();
            (
                StatusCode::OK,
                Body::from(encoded_metrics),
                CONTENT_TYPE_JSON,
            )
        },
        INDEX_PATH => {
            // /
            // Exposes the index and list of available endpoints
            let index_response = get_index_response();
            (
                StatusCode::OK,
                Body::from(index_response),
                CONTENT_TYPE_TEXT,
            )
        },
        JSON_METRICS_PATH => {
            // /json_metrics
            // Exposes JSON encoded metrics
            let encoder = JsonEncoder;
            let buffer = encode_metrics(encoder);
            (StatusCode::OK, Body::from(buffer), CONTENT_TYPE_JSON)
        },
        METRICS_PATH => {
            // /metrics
            // Exposes text encoded metrics
            let encoder = TextEncoder::new();
            let buffer = encode_metrics(encoder);
            (StatusCode::OK, Body::from(buffer), CONTENT_TYPE_TEXT)
        },
        PEER_INFORMATION_PATH => {
            // /peer_information
            // Exposes the peer information
            if node_config.inspection_service.expose_peer_information {
                let peer_information = get_peer_information(peers_and_metadata);
                (
                    StatusCode::OK,
                    Body::from(peer_information),
                    CONTENT_TYPE_TEXT,
                )
            } else {
                (
                    StatusCode::FORBIDDEN,
                    Body::from(PEER_INFO_DISABLED_MESSAGE),
                    CONTENT_TYPE_TEXT,
                )
            }
        },
        SYSTEM_INFORMATION_PATH => {
            // /system_information
            // Exposes the system and build information
            if node_config.inspection_service.expose_system_information {
                let mut system_information =
                    aptos_telemetry::system_information::get_system_information();
                let build_info = build_information!();
                system_information.extend(build_info);
                let encoded_information = serde_json::to_string(&system_information).unwrap();
                (
                    StatusCode::OK,
                    Body::from(encoded_information),
                    CONTENT_TYPE_JSON,
                )
            } else {
                (
                    StatusCode::FORBIDDEN,
                    Body::from(SYS_INFO_DISABLED_MESSAGE),
                    CONTENT_TYPE_TEXT,
                )
            }
        },
        _ => (
            StatusCode::NOT_FOUND,
            Body::from(INVALID_ENDPOINT_MESSAGE),
            CONTENT_TYPE_TEXT,
        ),
    };

    // Create a response builder
    let response_builder = Response::builder()
        .header(HEADER_CONTENT_TYPE, content_type)
        .status(status_code);

    // Build the response based on the request methods
    let response = match *req.method() {
        Method::HEAD => response_builder.body(Body::empty()), // Return only the headers
        Method::GET => response_builder.body(body),           // Include the response body
        _ => {
            // Invalid method found
            Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .body(Body::empty())
        },
    };

    // Return the processed response
    Ok(response.unwrap_or_else(|error| {
        // Log the internal error
        debug!("Error encountered when generating response: {:?}", error);

        // Return a failure response
        let mut response = Response::new(Body::from(UNEXPECTED_ERROR_MESSAGE));
        *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
        response
    }))
}

pub fn start_inspection_service(
    node_config: NodeConfig,
    peers_and_metadata: Arc<PeersAndMetadata>,
) {
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
            let peers_and_metadata = peers_and_metadata.clone();
            async move {
                Ok::<_, Infallible>(service_fn(move |request| {
                    serve_requests(request, node_config.clone(), peers_and_metadata.clone())
                }))
            }
        });

        let runtime = aptos_runtimes::spawn_named_runtime("inspection".into(), None);
        runtime
            .block_on(async {
                let server = Server::bind(&addr).serve(make_service);
                server.await
            })
            .unwrap();
    });
}

/// Returns the response for the index page. The response
/// simply lists all available endpoints.
fn get_index_response() -> String {
    let mut index_response: Vec<String> = Vec::new();

    // Add the list of available endpoints
    index_response.push("Welcome to the Aptos Inspection Service!".into());
    index_response.push("The following endpoints are available:".into());
    index_response.push(format!("\t- {}", CONFIGURATION_PATH));
    index_response.push(format!("\t- {}", FORGE_METRICS_PATH));
    index_response.push(format!("\t- {}", JSON_METRICS_PATH));
    index_response.push(format!("\t- {}", METRICS_PATH));
    index_response.push(format!("\t- {}", PEER_INFORMATION_PATH));
    index_response.push(format!("\t- {}", SYSTEM_INFORMATION_PATH));

    index_response.join("\n") // Separate each entry with a newline
}

/// Returns a simple text formatted string with peer and network information
fn get_peer_information(peers_and_metadata: Arc<PeersAndMetadata>) -> String {
    let mut peer_information = Vec::<String>::new();

    // Display a summary of all peers and networks
    let all_peers = peers_and_metadata.get_all_peers().unwrap_or_default();
    let registered_networks: Vec<NetworkId> =
        peers_and_metadata.get_registered_networks().collect();
    peer_information.push("Peer information summary:".into());
    peer_information.push(format!("\t- Number of peers: {}", all_peers.len()));
    peer_information.push(format!(
        "\t- Registered networks: {:?}",
        registered_networks
    ));
    peer_information.push(format!("\t- Peers and network IDs: {:?}", all_peers));
    peer_information.push("\n".into());

    // Display connection metadata for each peer
    peer_information.push("Connection metadata for each peer:".into());
    for peer in &all_peers {
        if let Ok(peer_metadata) = peers_and_metadata.get_metadata_for_peer(*peer) {
            let connection_metadata = peer_metadata.get_connection_metadata();
            peer_information.push(format!(
                "\t- Peer: {}, connection state: {:?}, connection metadata: {}",
                peer,
                peer_metadata.get_connection_state(),
                serde_json::to_string(&connection_metadata).unwrap_or_default()
            ));
        }
    }
    peer_information.push("\n".into());

    // Display the entire set of trusted peers
    peer_information.push("Trusted peers:".into());
    for network in registered_networks {
        peer_information.push(format!("\t- Network: {}", network));
        if let Ok(trusted_peers) = peers_and_metadata.get_trusted_peers(&network) {
            let trusted_peers = trusted_peers.read().clone();
            for trusted_peer in trusted_peers {
                peer_information.push(format!("\t\t- Peer: {:?}", trusted_peer));
            }
        }
    }
    peer_information.push("\n".into());

    // Display basic peer metadata for each peer
    peer_information.push("Basic monitoring metadata for each peer:".into());
    for peer in &all_peers {
        if let Ok(peer_metadata) = peers_and_metadata.get_metadata_for_peer(*peer) {
            let peer_monitoring_metadata = peer_metadata.get_peer_monitoring_metadata();
            peer_information.push(format!(
                "\t- Peer: {}, basic metadata: {}", // Display formatting for basic metadata
                peer, peer_monitoring_metadata
            ));
        }
    }
    peer_information.push("\n".into());

    // Display detailed peer metadata for each peer
    peer_information.push("Detailed monitoring metadata for each peer:".into());
    for peer in &all_peers {
        if let Ok(peer_metadata) = peers_and_metadata.get_metadata_for_peer(*peer) {
            let peer_monitoring_metadata = peer_metadata.get_peer_monitoring_metadata();
            peer_information.push(format!(
                "\t- Peer: {}, detailed metadata: {:?}", // Debug formatting for detailed metadata
                peer, peer_monitoring_metadata
            ));
        }
    }
    peer_information.push("\n".into());

    // Display the internal client state for each peer
    peer_information.push("Internal client state for each peer:".into());
    for peer in &all_peers {
        if let Ok(peer_metadata) = peers_and_metadata.get_metadata_for_peer(*peer) {
            let peer_monitoring_metadata = peer_metadata.get_peer_monitoring_metadata();
            peer_information.push(format!(
                "\t- Peer: {}, internal client state: {:?}",
                peer, peer_monitoring_metadata.internal_client_state
            ));
        }
    }

    peer_information.join("\n") // Separate each entry with a newline
}

#[cfg(test)]
mod test {
    use super::*;
    use futures::executor::block_on;
    use hyper::body;
    use once_cell::sync::Lazy;
    use prometheus::{register_int_counter, IntCounter};
    use std::io::read_to_string;

    // This metrics counter only exists in this test context; and the rest of
    // the system's metrics counters _don't_ exist, so we need to add one
    // here for test_inspect_metrics() below.
    const INT_COUNTER_NAME: &str = "INT_COUNTER";
    pub static INT_COUNTER: Lazy<IntCounter> =
        Lazy::new(|| register_int_counter!(INT_COUNTER_NAME, "An integer counter").unwrap());

    // exercise the serve_requests() handler kinda like how the HTTP framework would
    fn do_test_get(config: &NodeConfig, path: &str) -> Response<Body> {
        let mut uri = String::from("http://127.0.0.1:9201");
        uri += path;
        block_on(serve_requests(
            Request::builder()
                .uri(uri)
                .method(Method::GET)
                .body(Body::from(""))
                .unwrap(),
            config.clone(),
            PeersAndMetadata::new(&[]),
        ))
        .unwrap()
    }

    #[test]
    fn test_inspect_configuration() {
        let mut config = NodeConfig::get_default_validator_config();

        config.inspection_service.expose_configuration = false;
        let mut response_1 = do_test_get(&config, CONFIGURATION_PATH);
        assert_eq!(response_1.status(), StatusCode::FORBIDDEN);
        let response_1_body = block_on(body::to_bytes(response_1.body_mut())).unwrap();
        assert_eq!(response_1_body, CONFIGURATION_DISABLED_MESSAGE);

        config.inspection_service.expose_configuration = true;
        let mut response_2 = do_test_get(&config, CONFIGURATION_PATH);
        assert_eq!(response_2.status(), StatusCode::OK);
        let response_2_body = block_on(body::to_bytes(response_2.body_mut())).unwrap();
        let response_2_body_string = read_to_string(response_2_body.as_ref()).unwrap();
        assert!(response_2_body_string.contains("NodeConfig")); // debug format prints a field type name
        assert!(response_2_body_string.contains("InspectionServiceConfig")); // debug format prints a field type name
        assert!(response_2_body_string.contains("expose_configuration: true")); // configuration is on as set above
    }

    #[test]
    fn test_inspect_index() {
        // Create a PFN node config
        let config = NodeConfig::get_default_pfn_config();

        // Ping the index
        let mut response = do_test_get(&config, INDEX_PATH);
        let response_body = block_on(body::to_bytes(response.body_mut())).unwrap();
        let response_body_string: String = read_to_string(response_body.as_ref()).unwrap();

        // Verify that the response contains all the endpoints
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response_body_string.contains(CONFIGURATION_PATH));
        assert!(response_body_string.contains(FORGE_METRICS_PATH));
        assert!(response_body_string.contains(JSON_METRICS_PATH));
        assert!(response_body_string.contains(METRICS_PATH));
        assert!(response_body_string.contains(PEER_INFORMATION_PATH));
        assert!(response_body_string.contains(SYSTEM_INFORMATION_PATH));
    }

    #[test]
    fn test_inspect_system() {
        let mut config = NodeConfig::get_default_validator_config();

        config.inspection_service.expose_system_information = false;
        let mut response_1 = do_test_get(&config, SYSTEM_INFORMATION_PATH);
        assert_eq!(response_1.status(), StatusCode::FORBIDDEN);
        let response_1_body = block_on(body::to_bytes(response_1.body_mut())).unwrap();
        assert_eq!(response_1_body, SYS_INFO_DISABLED_MESSAGE);

        config.inspection_service.expose_system_information = true;
        let mut response_2 = do_test_get(&config, SYSTEM_INFORMATION_PATH);
        assert_eq!(response_2.status(), StatusCode::OK);
        let response_2_body = block_on(body::to_bytes(response_2.body_mut())).unwrap();
        let response_2_body_string = read_to_string(response_2_body.as_ref()).unwrap();
        // Assert some field names we expect to see in the system information:
        assert!(response_2_body_string.contains("build_commit_hash"));
        assert!(response_2_body_string.contains("cpu_count"));
        assert!(response_2_body_string.contains("memory_available"));
    }

    #[test]
    fn test_inspect_peer_information() {
        // Create a validator node config
        let mut config = NodeConfig::get_default_validator_config();

        // Disable the peer information endpoint and ping it
        config.inspection_service.expose_peer_information = false;
        let mut response = do_test_get(&config, PEER_INFORMATION_PATH);
        let response_body = block_on(body::to_bytes(response.body_mut())).unwrap();

        // Verify that the response contains an error
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(response_body, PEER_INFO_DISABLED_MESSAGE);

        // Enable the peer information endpoint and ping it
        config.inspection_service.expose_peer_information = true;
        let mut response = do_test_get(&config, PEER_INFORMATION_PATH);
        let response_body = block_on(body::to_bytes(response.body_mut())).unwrap();
        let response_body_string = read_to_string(response_body.as_ref()).unwrap();

        // Verify that the response contains the expected information
        assert_eq!(response.status(), StatusCode::OK);
        assert!(response_body_string.contains("Number of peers"));
        assert!(response_body_string.contains("Registered networks"));
        assert!(response_body_string.contains("Peers and network IDs"));
    }

    #[test]
    fn test_inspect_metrics() {
        INT_COUNTER.inc(); // make sure we have a count to show

        let config = NodeConfig::get_default_validator_config();
        let mut response_1 = do_test_get(&config, METRICS_PATH);
        assert_eq!(response_1.status(), StatusCode::OK);
        let response_1_body = block_on(body::to_bytes(response_1.body_mut())).unwrap();
        let response_1_body_string = read_to_string(response_1_body.as_ref()).unwrap();
        // Ensure that the text has at least the one counter we set.
        assert!(response_1_body_string.contains(INT_COUNTER_NAME));
    }

    #[test]
    fn test_inspect_json_metrics() {
        INT_COUNTER.inc(); // make sure we have a count to show

        let config = NodeConfig::get_default_validator_config();
        let mut response_1 = do_test_get(&config, JSON_METRICS_PATH);
        assert_eq!(response_1.status(), StatusCode::OK);
        let response_1_body = block_on(body::to_bytes(response_1.body_mut())).unwrap();
        let response_1_body_string = read_to_string(response_1_body.as_ref()).unwrap();
        // Ensure that the text has at least the one counter we set.
        assert!(response_1_body_string.contains(INT_COUNTER_NAME));
    }

    #[test]
    fn test_inspect_forge_metrics() {
        INT_COUNTER.inc(); // make sure we have a count to show

        let config = NodeConfig::get_default_validator_config();
        let mut response_1 = do_test_get(&config, FORGE_METRICS_PATH);
        assert_eq!(response_1.status(), StatusCode::OK);
        let response_1_body = block_on(body::to_bytes(response_1.body_mut())).unwrap();
        let response_1_body_string = read_to_string(response_1_body.as_ref()).unwrap();
        // Ensure that the text has at least the one counter we set.
        assert!(response_1_body_string.contains(INT_COUNTER_NAME));
    }
}
