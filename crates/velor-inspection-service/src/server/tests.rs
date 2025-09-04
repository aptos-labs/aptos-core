// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    server::{
        configuration::CONFIGURATION_DISABLED_MESSAGE,
        identity_information::IDENTITY_INFO_DISABLED_MESSAGE,
        peer_information::PEER_INFO_DISABLED_MESSAGE, serve_requests,
        system_information::SYS_INFO_DISABLED_MESSAGE, utils::get_all_metrics,
    },
    CONFIGURATION_PATH, FORGE_METRICS_PATH, IDENTITY_INFORMATION_PATH, INDEX_PATH,
    JSON_METRICS_PATH, METRICS_PATH, PEER_INFORMATION_PATH, SYSTEM_INFORMATION_PATH,
};
use velor_config::config::{VelorDataClientConfig, BaseConfig, Identity, NodeConfig};
use velor_data_client::client::VelorDataClient;
use velor_network::application::{interface::NetworkClient, storage::PeersAndMetadata};
use velor_storage_interface::DbReader;
use velor_storage_service_client::StorageServiceClient;
use velor_time_service::TimeService;
use assert_approx_eq::assert_approx_eq;
use futures::executor::block_on;
use hyper::{body, Body, Method, Request, Response, StatusCode};
use once_cell::sync::Lazy;
use prometheus::{proto::MetricFamily, register_int_counter, Counter, IntCounter, Opts, Registry};
use rusty_fork::rusty_fork_test;
use std::{collections::HashMap, io::read_to_string, string::String, sync::Arc};

// This metrics counter only exists in this test context; the rest of the
// system's metrics counters don't exist, so we need to add this for tests.
const INT_COUNTER_NAME: &str = "INT_COUNTER";
static INT_COUNTER: Lazy<IntCounter> =
    Lazy::new(|| register_int_counter!(INT_COUNTER_NAME, "An integer counter").unwrap());

#[tokio::test]
async fn test_inspect_configuration() {
    // Create a validator config
    let mut node_config = NodeConfig::get_default_validator_config();

    // Disable the configuration endpoint and ping it
    node_config.inspection_service.expose_configuration = false;
    let mut response = send_get_request_to_path(&node_config, CONFIGURATION_PATH).await;
    let response_body = body::to_bytes(response.body_mut()).await.unwrap();

    // Verify that the response contains an error
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(response_body, CONFIGURATION_DISABLED_MESSAGE);

    // Enable the configuration endpoint and ping it
    node_config.inspection_service.expose_configuration = true;
    let mut response = send_get_request_to_path(&node_config, CONFIGURATION_PATH).await;
    let response_body = body::to_bytes(response.body_mut()).await.unwrap();
    let response_body_string = read_to_string(response_body.as_ref()).unwrap();

    // Verify that the response contains the expected information
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response_body_string.contains("NodeConfig"));
    assert!(response_body_string.contains("InspectionServiceConfig"));
    assert!(response_body_string.contains("expose_configuration: true"));
}

#[tokio::test]
async fn test_inspect_forge_metrics() {
    // Create a VFN config
    let config = NodeConfig::get_default_vfn_config();

    // Increment a counter and get the forge metrics
    INT_COUNTER.inc();
    let mut response = send_get_request_to_path(&config, FORGE_METRICS_PATH).await;
    let response_body = body::to_bytes(response.body_mut()).await.unwrap();
    let response_body_string = read_to_string(response_body.as_ref()).unwrap();

    // Verify that the response contains the expected information
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response_body_string.contains(INT_COUNTER_NAME));
}

#[tokio::test]
async fn test_inspect_index() {
    // Create a PFN config
    let config = NodeConfig::get_default_pfn_config();

    // Ping the index
    let mut response = send_get_request_to_path(&config, INDEX_PATH).await;
    let response_body = body::to_bytes(response.body_mut()).await.unwrap();
    let response_body_string: String = read_to_string(response_body.as_ref()).unwrap();

    // Verify that the response contains all the endpoints
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response_body_string.contains(CONFIGURATION_PATH));
    assert!(response_body_string.contains(FORGE_METRICS_PATH));
    assert!(response_body_string.contains(IDENTITY_INFORMATION_PATH));
    assert!(response_body_string.contains(JSON_METRICS_PATH));
    assert!(response_body_string.contains(METRICS_PATH));
    assert!(response_body_string.contains(PEER_INFORMATION_PATH));
    assert!(response_body_string.contains(SYSTEM_INFORMATION_PATH));
}

#[tokio::test]
async fn test_inspect_json_metrics() {
    // Create a validator config
    let config = NodeConfig::get_default_validator_config();

    // Increment a counter and get the JSON metrics
    INT_COUNTER.inc();
    let mut response = send_get_request_to_path(&config, JSON_METRICS_PATH).await;
    let response_body = body::to_bytes(response.body_mut()).await.unwrap();
    let response_body_string = read_to_string(response_body.as_ref()).unwrap();

    // Verify that the response contains the expected information
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response_body_string.contains(INT_COUNTER_NAME));
}

#[tokio::test]
async fn test_inspect_identity_information() {
    // Create a validator config (with a single validator identity)
    let mut config = NodeConfig::get_default_validator_config();
    if let Some(network_config) = config.validator_network.as_mut() {
        network_config.identity = Identity::None; // Reset the identity
        network_config
            .set_listen_address_and_prepare_identity()
            .unwrap(); // Generates a random identity
    }
    config.full_node_networks = vec![];

    // Disable the identity information endpoint and ping it
    config.inspection_service.expose_identity_information = false;
    let mut response = send_get_request_to_path(&config, IDENTITY_INFORMATION_PATH).await;
    let response_body = body::to_bytes(response.body_mut()).await.unwrap();

    // Verify that the response contains an error
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(response_body, IDENTITY_INFO_DISABLED_MESSAGE);

    // Enable the identity information endpoint and ping it
    config.inspection_service.expose_identity_information = true;
    let mut response = send_get_request_to_path(&config, IDENTITY_INFORMATION_PATH).await;
    let response_body = body::to_bytes(response.body_mut()).await.unwrap();
    let response_body_string = read_to_string(response_body.as_ref()).unwrap();

    // Verify that the response contains the expected information
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response_body_string.contains("Identity Information:"));
}

#[tokio::test]
async fn test_inspect_metrics() {
    // Create a validator config
    let config = NodeConfig::get_default_validator_config();

    // Increment a counter and get the metrics
    INT_COUNTER.inc();
    let mut response = send_get_request_to_path(&config, METRICS_PATH).await;
    let response_body = body::to_bytes(response.body_mut()).await.unwrap();
    let response_body_string = read_to_string(response_body.as_ref()).unwrap();

    // Verify that the response contains the expected information
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response_body_string.contains(INT_COUNTER_NAME));
}

#[tokio::test]
async fn test_inspect_system_information() {
    // Create a validator node config
    let mut config = NodeConfig::get_default_validator_config();

    // Disable the system information endpoint and ping it
    config.inspection_service.expose_system_information = false;
    let mut response = send_get_request_to_path(&config, SYSTEM_INFORMATION_PATH).await;
    let response_body = body::to_bytes(response.body_mut()).await.unwrap();

    // Verify that the response contains an error
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(response_body, SYS_INFO_DISABLED_MESSAGE);

    // Enable the system information endpoint and ping it
    config.inspection_service.expose_system_information = true;
    let mut response = send_get_request_to_path(&config, SYSTEM_INFORMATION_PATH).await;
    let response_body = body::to_bytes(response.body_mut()).await.unwrap();
    let response_body_string = read_to_string(response_body.as_ref()).unwrap();

    // Verify that the response contains the expected information
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response_body_string.contains("build_commit_hash"));
    assert!(response_body_string.contains("cpu_count"));
    assert!(response_body_string.contains("memory_available"));
}

#[tokio::test]
async fn test_inspect_peer_information() {
    // Create a validator node config
    let mut config = NodeConfig::get_default_validator_config();

    // Disable the peer information endpoint and ping it
    config.inspection_service.expose_peer_information = false;
    let mut response = send_get_request_to_path(&config, PEER_INFORMATION_PATH).await;
    let response_body = block_on(body::to_bytes(response.body_mut())).unwrap();

    // Verify that the response contains an error
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(response_body, PEER_INFO_DISABLED_MESSAGE);

    // Enable the peer information endpoint and ping it
    config.inspection_service.expose_peer_information = true;
    let mut response = send_get_request_to_path(&config, PEER_INFORMATION_PATH).await;
    let response_body = block_on(body::to_bytes(response.body_mut())).unwrap();
    let response_body_string = read_to_string(response_body.as_ref()).unwrap();

    // Verify that the response contains the expected information
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response_body_string.contains("Number of peers"));
    assert!(response_body_string.contains("Registered networks"));
    assert!(response_body_string.contains("Peers and network IDs"));
    assert!(response_body_string.contains("State sync metadata"));
}

rusty_fork_test! {
#[test]
fn test_gather_metrics() {
    // Increment the counter
    let iterations = 12;
    for _ in 0..iterations {
        INT_COUNTER.inc();
    }

    // Fetch the metrics and verify that a new entry was added
    let all_metrics = get_all_metrics();
    assert_eq!(all_metrics.len(), 1);

    // Verify that the counter has the expected value
    for (metric, value) in get_all_metrics() {
        if metric.starts_with(INT_COUNTER_NAME) {
            assert_eq!(value, iterations.to_string());
            return;
        }
    }
    panic!("Metric {} not found", INT_COUNTER_NAME);
}
}

rusty_fork_test! {
#[test]
fn test_get_all_metrics() {
    // Increment the counter
    INT_COUNTER.inc();

    // Verify that the metrics map only has one entry
    let metrics = get_all_metrics();
    assert_eq!(metrics.len(), 1);

    // Verify that the counter has the expected value
    let counter_value = metrics.values().next().unwrap().parse::<i32>().unwrap();
    assert_eq!(counter_value, 1);
}
}

#[test]
fn test_publish_metrics() {
    // Create a counter metric
    let counter_opts = Opts::new("test_counter", "test counter help");
    let counter = Counter::with_opts(counter_opts).unwrap();

    // Register the counter metric
    let register = Registry::new();
    register.register(Box::new(counter.clone())).unwrap();

    // Increment the counter and verify that the metric families are updated
    counter.inc();
    let metric_families = register.gather();
    assert_eq!(metric_families.len(), 1);

    // Verify that the metric family has the expected values
    let metric_family: &MetricFamily = metric_families.first().unwrap();
    assert_eq!("test counter help", metric_family.get_help());
    assert_eq!("test_counter", metric_family.get_name());

    // Verify that the metric has the expected value
    let metrics = metric_family.get_metric();
    assert_eq!(metrics.len(), 1);
    assert_approx_eq!(1.0, metrics.first().unwrap().get_counter().get_value());
}

// Exercise the serve_requests() handler with a GET request to the given path
async fn send_get_request_to_path(config: &NodeConfig, endpoint: &str) -> Response<Body> {
    // Build the URI
    let uri = format!("http://127.0.0.1:9201{}", endpoint);

    // Create the peers and metadata
    let peers_and_metadata = PeersAndMetadata::new(&[]);

    // Create the data client
    let network_client =
        NetworkClient::new(vec![], vec![], HashMap::new(), peers_and_metadata.clone());
    let (velor_data_client, _) = VelorDataClient::new(
        VelorDataClientConfig::default(),
        BaseConfig::default(),
        TimeService::mock(),
        Arc::new(MockDatabaseReader {}),
        StorageServiceClient::new(network_client),
        None,
    );

    // Serve the request
    serve_requests(
        Request::builder()
            .uri(uri)
            .method(Method::GET)
            .body(Body::from(""))
            .unwrap(),
        config.clone(),
        velor_data_client,
        peers_and_metadata,
    )
    .await
    .unwrap()
}

/// A simple mock database reader
pub struct MockDatabaseReader {}
impl DbReader for MockDatabaseReader {}
