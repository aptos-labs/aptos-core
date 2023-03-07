// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{
    logging::{LogEntry, LogSchema},
    metrics::{increment_counter, start_timer},
    network::PeerMonitoringServiceNetworkEvents,
};
use aptos_bounded_executor::BoundedExecutor;
use aptos_config::config::{BaseConfig, NodeConfig, RoleType};
use aptos_logger::prelude::*;
use aptos_network::{application::storage::PeersAndMetadata, ProtocolId};
use aptos_peer_monitoring_service_types::{
    LatencyPingRequest, LatencyPingResponse, NetworkInformationResponse,
    PeerMonitoringServiceError, PeerMonitoringServiceRequest, PeerMonitoringServiceResponse,
    Result, ServerProtocolVersionResponse, MAX_DISTANCE_FROM_VALIDATORS,
};
use error::Error;
use futures::stream::StreamExt;
use std::{cmp::min, sync::Arc};
use tokio::runtime::Handle;

mod error;
mod logging;
pub mod metrics;
pub mod network;

#[cfg(test)]
mod tests;

/// Peer monitoring server constants
pub const PEER_MONITORING_SERVER_VERSION: u64 = 1;

/// The server-side actor for the peer monitoring service
pub struct PeerMonitoringServiceServer {
    base_config: BaseConfig,
    bounded_executor: BoundedExecutor,
    network_requests: PeerMonitoringServiceNetworkEvents,
    peers_and_metadata: Arc<PeersAndMetadata>,
}

impl PeerMonitoringServiceServer {
    pub fn new(
        node_config: NodeConfig,
        executor: Handle,
        network_requests: PeerMonitoringServiceNetworkEvents,
        peers_and_metadata: Arc<PeersAndMetadata>,
    ) -> Self {
        let base_config = node_config.base;
        let bounded_executor = BoundedExecutor::new(
            node_config.peer_monitoring_service.max_concurrent_requests as usize,
            executor,
        );

        Self {
            base_config,
            bounded_executor,
            network_requests,
            peers_and_metadata,
        }
    }

    /// Starts the peer monitoring service server thread
    pub async fn start(mut self) {
        // Handle the service requests
        while let Some(network_request) = self.network_requests.next().await {
            // Log the request
            let peer_network_id = network_request.peer_network_id;
            let protocol_id = network_request.protocol_id;
            let peer_monitoring_service_request = network_request.peer_monitoring_service_request;
            let response_sender = network_request.response_sender;
            trace!(LogSchema::new(LogEntry::ReceivedPeerMonitoringRequest)
                .request(&peer_monitoring_service_request)
                .message(&format!(
                    "Received peer monitoring request. Peer: {:?}, protocol: {:?}.",
                    peer_network_id, protocol_id,
                )));

            // All handler methods are currently CPU-bound so we want
            // to spawn on the blocking thread pool.
            let base_config = self.base_config.clone();
            let peers_and_metadata = self.peers_and_metadata.clone();
            self.bounded_executor
                .spawn_blocking(move || {
                    let response = Handler::new(base_config, peers_and_metadata)
                        .call(protocol_id, peer_monitoring_service_request);
                    log_monitoring_service_response(&response);
                    response_sender.send(response);
                })
                .await;
        }
    }
}

/// The `Handler` is the "pure" inbound request handler. It contains all the
/// necessary context and state needed to construct a response to an inbound
/// request. We usually clone/create a new handler for every request.
#[derive(Clone)]
pub struct Handler {
    base_config: BaseConfig,
    peers_and_metadata: Arc<PeersAndMetadata>,
}

impl Handler {
    pub fn new(base_config: BaseConfig, peers_and_metadata: Arc<PeersAndMetadata>) -> Self {
        Self {
            base_config,
            peers_and_metadata,
        }
    }

    pub fn call(
        &self,
        protocol: ProtocolId,
        request: PeerMonitoringServiceRequest,
    ) -> Result<PeerMonitoringServiceResponse> {
        // Update the request count
        increment_counter(
            &metrics::PEER_MONITORING_REQUESTS_RECEIVED,
            protocol,
            request.get_label(),
        );

        // Time the request processing (the timer will stop when it's dropped)
        let _timer = start_timer(
            &metrics::PEER_MONITORING_REQUEST_PROCESSING_LATENCY,
            protocol,
            request.get_label(),
        );

        // Process the request
        let response = match &request {
            PeerMonitoringServiceRequest::GetNetworkInformation => self.get_network_information(),
            PeerMonitoringServiceRequest::GetServerProtocolVersion => {
                self.get_server_protocol_version()
            },
            PeerMonitoringServiceRequest::LatencyPing(request) => self.handle_latency_ping(request),
        };

        // Process the response and handle any errors
        match response {
            Err(error) => {
                // Log the error and update the counters
                increment_counter(
                    &metrics::PEER_MONITORING_ERRORS_ENCOUNTERED,
                    protocol,
                    error.get_label(),
                );
                error!(LogSchema::new(LogEntry::PeerMonitoringServiceError)
                    .error(&error)
                    .request(&request));

                // Return an appropriate response to the client
                match error {
                    Error::InvalidRequest(error) => {
                        Err(PeerMonitoringServiceError::InvalidRequest(error))
                    },
                    error => Err(PeerMonitoringServiceError::InternalError(error.to_string())),
                }
            },
            Ok(response) => {
                // The request was successful
                increment_counter(
                    &metrics::PEER_MONITORING_RESPONSES_SENT,
                    protocol,
                    response.get_label(),
                );
                Ok(response)
            },
        }
    }

    fn get_network_information(&self) -> Result<PeerMonitoringServiceResponse, Error> {
        // Get all required network information
        let connected_peers_and_metadata =
            self.peers_and_metadata.get_connected_peers_and_metadata()?;
        let distance_from_validators =
            get_distance_from_validators(&self.base_config, self.peers_and_metadata.clone());

        // Create and send the response
        let network_information_response = NetworkInformationResponse {
            connected_peers_and_metadata,
            distance_from_validators,
        };
        Ok(PeerMonitoringServiceResponse::NetworkInformation(
            network_information_response,
        ))
    }

    fn get_server_protocol_version(&self) -> Result<PeerMonitoringServiceResponse, Error> {
        let server_protocol_version_response = ServerProtocolVersionResponse {
            version: PEER_MONITORING_SERVER_VERSION,
        };
        Ok(PeerMonitoringServiceResponse::ServerProtocolVersion(
            server_protocol_version_response,
        ))
    }

    fn handle_latency_ping(
        &self,
        latency_ping_request: &LatencyPingRequest,
    ) -> Result<PeerMonitoringServiceResponse, Error> {
        let latency_ping_response = LatencyPingResponse {
            ping_counter: latency_ping_request.ping_counter,
        };
        Ok(PeerMonitoringServiceResponse::LatencyPing(
            latency_ping_response,
        ))
    }
}

/// Returns the distance from the validators using the given base config
/// and the peers and metadata information.
fn get_distance_from_validators(
    base_config: &BaseConfig,
    peers_and_metadata: Arc<PeersAndMetadata>,
) -> u64 {
    match base_config.role {
        RoleType::Validator => 0, // We're a validator!
        RoleType::FullNode => {
            match peers_and_metadata.get_connected_peers_and_metadata() {
                Ok(peers_and_metadata) => {
                    // Go through our peers, find the min, and return a distance relative to the min
                    let mut min_peer_distance_from_validators = MAX_DISTANCE_FROM_VALIDATORS;
                    for peer_metadata in peers_and_metadata.values() {
                        if let Some(distance_from_validators) = peer_metadata
                            .get_peer_monitoring_metadata()
                            .distance_from_validators
                        {
                            min_peer_distance_from_validators =
                                min(min_peer_distance_from_validators, distance_from_validators);
                        }
                    }

                    // We're one hop away from the peer
                    min(
                        MAX_DISTANCE_FROM_VALIDATORS,
                        min_peer_distance_from_validators + 1,
                    )
                },
                Err(error) => {
                    // Log the error and return the max distance
                    warn!(LogSchema::new(LogEntry::PeerMonitoringServiceError).error(&error.into()));
                    MAX_DISTANCE_FROM_VALIDATORS
                },
            }
        },
    }
}

/// Logs the response sent by the monitoring service for a request
fn log_monitoring_service_response(
    monitoring_service_response: &Result<PeerMonitoringServiceResponse, PeerMonitoringServiceError>,
) {
    let response = match monitoring_service_response {
        Ok(response) => {
            format!("{:?}", response)
        },
        Err(error) => {
            format!("{:?}", error)
        },
    };
    debug!(LogSchema::new(LogEntry::SentPeerMonitoringResponse).response(&response));
}
