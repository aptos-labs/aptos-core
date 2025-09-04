// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::server::utils::CONTENT_TYPE_TEXT;
use velor_config::config::NodeConfig;
use velor_data_client::client::VelorDataClient;
use velor_logger::debug;
use velor_network::application::storage::PeersAndMetadata;
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, Server, StatusCode,
};
use std::{
    convert::Infallible,
    net::{SocketAddr, ToSocketAddrs},
    sync::Arc,
    thread,
};

mod configuration;
mod identity_information;
mod index;
mod json_encoder;
mod metrics;
mod peer_information;
mod system_information;
pub mod utils;

#[cfg(test)]
mod tests;

// The list of endpoints offered by the inspection service
pub const CONFIGURATION_PATH: &str = "/configuration";
pub const CONSENSUS_HEALTH_CHECK_PATH: &str = "/consensus_health_check";
pub const FORGE_METRICS_PATH: &str = "/forge_metrics";
pub const IDENTITY_INFORMATION_PATH: &str = "/identity_information";
pub const INDEX_PATH: &str = "/";
pub const JSON_METRICS_PATH: &str = "/json_metrics";
pub const METRICS_PATH: &str = "/metrics";
pub const PEER_INFORMATION_PATH: &str = "/peer_information";
pub const SYSTEM_INFORMATION_PATH: &str = "/system_information";

// Useful string constants
pub const HEADER_CONTENT_TYPE: &str = "Content-Type";
pub const INVALID_ENDPOINT_MESSAGE: &str = "The requested endpoint is invalid!";
pub const UNEXPECTED_ERROR_MESSAGE: &str = "An unexpected error was encountered!";

/// Starts the inspection service that listens on the configured
/// address and handles various endpoint requests.
pub fn start_inspection_service(
    node_config: NodeConfig,
    velor_data_client: VelorDataClient,
    peers_and_metadata: Arc<PeersAndMetadata>,
) {
    // Fetch the service port and address
    let service_port = node_config.inspection_service.port;
    let service_address = node_config.inspection_service.address.clone();

    // Create the inspection service socket address
    let address: SocketAddr = (service_address.as_str(), service_port)
        .to_socket_addrs()
        .unwrap_or_else(|_| {
            panic!(
                "Failed to parse {}:{} as address",
                service_address, service_port
            )
        })
        .next()
        .unwrap();

    // Create a runtime for the inspection service
    let runtime = velor_runtimes::spawn_named_runtime("inspection".into(), None);

    // Spawn the inspection service
    thread::spawn(move || {
        // Create the service function that handles the endpoint requests
        let make_service = make_service_fn(move |_conn| {
            let node_config = node_config.clone();
            let velor_data_client = velor_data_client.clone();
            let peers_and_metadata = peers_and_metadata.clone();
            async move {
                Ok::<_, Infallible>(service_fn(move |request| {
                    serve_requests(
                        request,
                        node_config.clone(),
                        velor_data_client.clone(),
                        peers_and_metadata.clone(),
                    )
                }))
            }
        });

        // Start and block on the server
        runtime
            .block_on(async {
                let server = Server::bind(&address).serve(make_service);
                server.await
            })
            .unwrap();
    });
}

/// A simple helper function that handles each endpoint request
async fn serve_requests(
    req: Request<Body>,
    node_config: NodeConfig,
    velor_data_client: VelorDataClient,
    peers_and_metadata: Arc<PeersAndMetadata>,
) -> Result<Response<Body>, hyper::Error> {
    // Process the request and get the response components
    let (status_code, body, content_type) = match req.uri().path() {
        CONFIGURATION_PATH => {
            // /configuration
            // Exposes the node configuration
            configuration::handle_configuration_request(&node_config)
        },
        CONSENSUS_HEALTH_CHECK_PATH => {
            // /consensus_health_check
            // Exposes the consensus health check
            metrics::handle_consensus_health_check(&node_config).await
        },
        FORGE_METRICS_PATH => {
            // /forge_metrics
            // Exposes forge encoded metrics
            metrics::handle_forge_metrics()
        },
        IDENTITY_INFORMATION_PATH => {
            // /identity_information
            // Exposes the identity information of the node
            identity_information::handle_identity_information_request(&node_config)
        },
        INDEX_PATH => {
            // /
            // Exposes the index and list of available endpoints
            index::handle_index_request()
        },
        JSON_METRICS_PATH => {
            // /json_metrics
            // Exposes JSON encoded metrics
            metrics::handle_json_metrics_request()
        },
        METRICS_PATH => {
            // /metrics
            // Exposes text encoded metrics
            metrics::handle_metrics_request()
        },
        PEER_INFORMATION_PATH => {
            // /peer_information
            // Exposes the peer information
            peer_information::handle_peer_information_request(
                &node_config,
                velor_data_client,
                peers_and_metadata,
            )
        },
        SYSTEM_INFORMATION_PATH => {
            // /system_information
            // Exposes the system and build information
            system_information::handle_system_information_request(node_config)
        },
        _ => {
            // Handle the invalid path
            (
                StatusCode::NOT_FOUND,
                Body::from(INVALID_ENDPOINT_MESSAGE),
                CONTENT_TYPE_TEXT.into(),
            )
        },
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
