// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{
    logging::{LogEntry, LogSchema},
    metrics::{increment_counter, start_timer},
    // network::PeerMonitoringServiceNetworkEvents,
    storage::StorageReaderInterface,
};
use aptos_bounded_executor::BoundedExecutor;
use aptos_config::{
    config::{BaseConfig, NodeConfig},
    network_id::NetworkId,
};
use aptos_logger::prelude::*;
use aptos_network::application::storage::PeersAndMetadata;
use aptos_peer_monitoring_service_types::{request::{LatencyPingRequest, PeerMonitoringServiceRequest}, response::{
    ConnectionMetadata, LatencyPingResponse, NetworkInformationResponse,
    NodeInformationResponse, PeerMonitoringServiceResponse, ServerProtocolVersionResponse,
}, PeerMonitoringServiceError, Result, MAX_DISTANCE_FROM_VALIDATORS, PeerMonitoringSharedState, PeerMonitoringServiceMessage, DirectNetPerformanceMessage};
use aptos_time_service::{TimeService, TimeServiceTrait};
use error::Error;
use futures::stream::StreamExt;
use std::{cmp::min, sync::Arc, time::Instant};
use std::sync::RwLock;
use bytes::Bytes;
use futures::channel::oneshot;
use tokio::runtime::Handle;
use aptos_config::network_id::PeerNetworkId;
use aptos_network::application::interface::{NetworkClient, NetworkClientInterface, NetworkServiceEvents};
use aptos_network::ProtocolId;
use aptos_network::protocols::network::{Event, RpcError};
use aptos_types::account_address::AccountAddress;

mod error;
mod logging;
pub mod metrics;
pub mod network;
pub mod storage;

#[cfg(test)]
mod tests;

/// Peer monitoring server constants
pub const PEER_MONITORING_SERVER_VERSION: u64 = 1;

/// The server-side actor for the peer monitoring service
pub struct PeerMonitoringServiceServer<T> {
    base_config: BaseConfig,
    bounded_executor: BoundedExecutor,
    //network_requests: NetworkServiceEvents<PeerMonitoringServiceMessage>,//PeerMonitoringServiceNetworkEvents,
    peers_and_metadata: Arc<PeersAndMetadata>,
    start_time: Instant,
    storage: T,
    time_service: TimeService,
    shared: Arc<RwLock<PeerMonitoringSharedState>>,
}

impl<T: StorageReaderInterface> PeerMonitoringServiceServer<T> {
    pub fn new(
        node_config: NodeConfig,
        executor: Handle,
        //network_requests: NetworkServiceEvents<PeerMonitoringServiceMessage>, //PeerMonitoringServiceNetworkEvents,
        peers_and_metadata: Arc<PeersAndMetadata>,
        storage: T,
        time_service: TimeService,
        shared: Arc<RwLock<PeerMonitoringSharedState>>,
    ) -> Self {
        let base_config = node_config.base;
        let bounded_executor = BoundedExecutor::new(
            node_config.peer_monitoring_service.max_concurrent_requests as usize,
            executor,
        );
        let start_time = time_service.now();

        Self {
            base_config,
            bounded_executor,
            //network_requests,
            peers_and_metadata,
            start_time,
            storage,
            time_service,
            shared,
        }
    }

    /// Starts the peer monitoring service server thread
    pub async fn start(
        mut self,
        network_requests: NetworkServiceEvents<PeerMonitoringServiceMessage>,
        network_client: NetworkClient<PeerMonitoringServiceMessage>,
    ) {
        let network_events: Vec<_> = network_requests
            .into_network_and_events()
            .into_iter()
            .map(|(network_id, events)| events.map(move |event| (network_id, event)))
            .collect();
        let mut network_events = futures::stream::select_all(network_events).fuse();
        // Handle the service requests
        // while let Some(network_request) = self.network_requests.next().await {
        loop {
            let (network_id, event) = match network_events.next().await {
                None => { return; }  // fused stream will never return more
                Some(x) => { x }
            };
            match event {
                Event::Message(peer_id, msg_wrapper) => {
                    // TODO: counters, note blob size and increment message counter
                    match msg_wrapper {
                        PeerMonitoringServiceMessage::Request(_) => {}
                        PeerMonitoringServiceMessage::Response(_) => {}
                        PeerMonitoringServiceMessage::DirectNetPerformance(msg) => {
                            if msg.data.is_empty() {
                                // this is a reply, note the request_counter got back and round trip time
                                let receive_time = self.time_service.now_unix_time().as_micros() as i64;
                                    let rec = {
                                        self.shared.read().unwrap().find(msg.request_counter)
                                    };
                                    if rec.request_counter == msg.request_counter {
                                        info!("pmd[{}] {} bytes in {} micros", rec.request_counter, rec.bytes_sent, receive_time - rec.send_micros);
                                    } else {
                                        info!("pmd[{}] unk bytes in > {} micros", msg.request_counter, receive_time - rec.send_micros)
                                    }
                            } else {
                                // make a reply, empty data, local time, request counter from source
                                let reply = DirectNetPerformanceMessage{
                                    request_counter: msg.request_counter,
                                    send_micros: self.time_service.now_unix_time().as_micros() as i64,
                                    data: vec![],
                                };
                                let reply = PeerMonitoringServiceMessage::DirectNetPerformance(reply);
                                let send_ok = network_client.send_to_peer(reply, PeerNetworkId::new(network_id, peer_id)); // TODO: log&count error
                                if let Err(err) = send_ok {
                                    info!("pmd[{}] reply send failed: {}", msg.request_counter, err);
                                }
                            }
                        }
                    }
                }
                Event::RpcRequest(peer_id, msg_wrapper, protocol_id, sender) => {
                    match msg_wrapper {
                        PeerMonitoringServiceMessage::Request(request) => {
                            self.handle_rpc(network_id, peer_id, request, protocol_id, sender).await;
                        }
                        PeerMonitoringServiceMessage::Response(_) => {}
                        PeerMonitoringServiceMessage::DirectNetPerformance(_) => {}
                    };
                }
                Event::NewPeer(_) => {}  // don't care
                Event::LostPeer(_) => {}  // don't care
            }
        }
    }

    pub async fn handle_rpc(&mut self,
                            network_id: NetworkId,
                            peer_id: AccountAddress,
                            peer_monitoring_service_request: PeerMonitoringServiceRequest,
                            _protocol_id: ProtocolId,
                            sender: oneshot::Sender<Result<Bytes, RpcError>>,
    ) {
            // Log the request
            //let peer_network_id = network_request.peer_network_id;
            //let peer_monitoring_service_request = network_request.peer_monitoring_service_request;
            //let response_sender = network_request.response_sender;
            //let response_sender = sender;
            trace!(LogSchema::new(LogEntry::ReceivedPeerMonitoringRequest)
                .request(&peer_monitoring_service_request)
                .message(&format!(
                    "Received peer monitoring request. Peer: {:?} {:?}",
                    peer_id, network_id,
                )));

            // All handler methods are currently CPU-bound so we want
            // to spawn on the blocking thread pool.
            let base_config = self.base_config.clone();
            let peers_and_metadata = self.peers_and_metadata.clone();
            let start_time = self.start_time;
            let storage = self.storage.clone();
            let time_service = self.time_service.clone();
            self.bounded_executor
                .spawn_blocking(move || {
                    let response = Handler::new(
                        base_config,
                        peers_and_metadata,
                        start_time,
                        storage,
                        time_service,
                    )
                    .call(
                        network_id,
                        peer_monitoring_service_request,
                    );
                    log_monitoring_service_response(&response);
                    let msg = PeerMonitoringServiceMessage::Response(response);
                    let result = bcs::to_bytes(&msg)
                        .map(Bytes::from)
                        .map_err(RpcError::BcsError);
                    if let Err(_) = sender.send(result) {
                        info!("PM rpc reply failed")
                    }
                    //response_sender.send(response);
                })
                .await;

    }
}

/// The `Handler` is the "pure" inbound request handler. It contains all the
/// necessary context and state needed to construct a response to an inbound
/// request. We usually clone/create a new handler for every request.
#[derive(Clone)]
pub struct Handler<T> {
    base_config: BaseConfig,
    peers_and_metadata: Arc<PeersAndMetadata>,
    start_time: Instant,
    storage: T,
    time_service: TimeService,
}

impl<T: StorageReaderInterface> Handler<T> {
    pub fn new(
        base_config: BaseConfig,
        peers_and_metadata: Arc<PeersAndMetadata>,
        start_time: Instant,
        storage: T,
        time_service: TimeService,
    ) -> Self {
        Self {
            base_config,
            peers_and_metadata,
            start_time,
            storage,
            time_service,
        }
    }

    pub fn call(
        &self,
        network_id: NetworkId,
        request: PeerMonitoringServiceRequest,
    ) -> Result<PeerMonitoringServiceResponse> {
        // Update the request count
        increment_counter(
            &metrics::PEER_MONITORING_REQUESTS_RECEIVED,
            network_id,
            request.get_label(),
        );

        // Time the request processing (the timer will stop when it's dropped)
        let _timer = start_timer(
            &metrics::PEER_MONITORING_REQUEST_PROCESSING_LATENCY,
            network_id,
            request.get_label(),
        );

        // Process the request
        let response = match &request {
            PeerMonitoringServiceRequest::GetNetworkInformation => self.get_network_information(),
            PeerMonitoringServiceRequest::GetServerProtocolVersion => {
                self.get_server_protocol_version()
            },
            PeerMonitoringServiceRequest::GetNodeInformation => self.get_node_information(),
            PeerMonitoringServiceRequest::LatencyPing(request) => self.handle_latency_ping(request),

            #[cfg(feature = "network-perf-test")] // Disabled by default
            PeerMonitoringServiceRequest::PerformanceMonitoringRequest(request) => {
                self.handle_performance_monitoring_request(request)
            },
        };

        // Process the response and handle any errors
        match response {
            Err(error) => {
                // Log the error and update the counters
                increment_counter(
                    &metrics::PEER_MONITORING_ERRORS_ENCOUNTERED,
                    network_id,
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
                    network_id,
                    response.get_label(),
                );
                Ok(response)
            },
        }
    }

    fn get_network_information(&self) -> Result<PeerMonitoringServiceResponse, Error> {
        // Get the connected peers
        let connected_peers_and_metadata =
            self.peers_and_metadata.get_connected_peers_and_metadata()?;
        let connected_peers = connected_peers_and_metadata
            .into_iter()
            .map(|(peer, metadata)| {
                let connection_metadata = metadata.get_connection_metadata();
                (
                    peer,
                    ConnectionMetadata::new(
                        connection_metadata.addr,
                        connection_metadata.remote_peer_id,
                        connection_metadata.role,
                    ),
                )
            })
            .collect();

        // Get the distance from the validators
        let distance_from_validators =
            get_distance_from_validators(&self.base_config, self.peers_and_metadata.clone());

        // Create and return the response
        let network_information_response = NetworkInformationResponse {
            connected_peers,
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

    fn get_node_information(&self) -> Result<PeerMonitoringServiceResponse, Error> {
        // Get the node information
        let build_information = aptos_build_info::get_build_information();
        let current_time: Instant = self.time_service.now();
        let uptime = current_time.duration_since(self.start_time);
        let (highest_synced_epoch, highest_synced_version) =
            self.storage.get_highest_synced_epoch_and_version()?;
        let ledger_timestamp_usecs = self.storage.get_ledger_timestamp_usecs()?;
        let lowest_available_version = self.storage.get_lowest_available_version()?;

        // Create and return the response
        let node_information_response = NodeInformationResponse {
            build_information,
            highest_synced_epoch,
            highest_synced_version,
            ledger_timestamp_usecs,
            lowest_available_version,
            uptime,
        };
        Ok(PeerMonitoringServiceResponse::NodeInformation(
            node_information_response,
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

    #[cfg(feature = "network-perf-test")] // Disabled by default
    fn handle_performance_monitoring_request(
        &self,
        performance_monitoring_request: &aptos_peer_monitoring_service_types::request::PerformanceMonitoringRequest,
    ) -> Result<PeerMonitoringServiceResponse, Error> {
        let performance_monitoring_response =
            aptos_peer_monitoring_service_types::response::PerformanceMonitoringResponse {
                response_counter: performance_monitoring_request.request_counter,
            };
        Ok(PeerMonitoringServiceResponse::PerformanceMonitoring(
            performance_monitoring_response,
        ))
    }
}

/// Returns the distance from the validators using the given base config
/// and the peers and metadata information.
fn get_distance_from_validators(
    base_config: &BaseConfig,
    peers_and_metadata: Arc<PeersAndMetadata>,
) -> u64 {
    // Get the connected peers and metadata
    let connected_peers_and_metadata = match peers_and_metadata.get_connected_peers_and_metadata() {
        Ok(connected_peers_and_metadata) => connected_peers_and_metadata,
        Err(error) => {
            warn!(LogSchema::new(LogEntry::PeerMonitoringServiceError).error(&error.into()));
            return MAX_DISTANCE_FROM_VALIDATORS;
        },
    };

    // If we're a validator and we have active validator peers, we're in the validator set.
    // TODO: figure out if we need to deal with validator set forks here.
    if base_config.role.is_validator() {
        for peer_metadata in connected_peers_and_metadata.values() {
            if peer_metadata.get_connection_metadata().role.is_validator() {
                return 0;
            }
        }
    }

    // Otherwise, go through our peers, find the min, and return a distance relative to the min
    let mut min_peer_distance_from_validators = MAX_DISTANCE_FROM_VALIDATORS;
    for peer_metadata in connected_peers_and_metadata.values() {
        if let Some(latest_network_info_response) = peer_metadata
            .get_peer_monitoring_metadata()
            .latest_network_info_response
        {
            min_peer_distance_from_validators = min(
                min_peer_distance_from_validators,
                latest_network_info_response.distance_from_validators,
            );
        }
    }

    // We're one hop away from the peer
    min(
        MAX_DISTANCE_FROM_VALIDATORS,
        min_peer_distance_from_validators + 1,
    )
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
