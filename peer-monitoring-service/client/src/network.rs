// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    logging::{LogEntry, LogEvent, LogSchema},
    metrics, Error,
};
use velor_config::network_id::PeerNetworkId;
use velor_logger::{trace, warn};
use velor_network::application::{
    interface::{NetworkClient, NetworkClientInterface},
    storage::PeersAndMetadata,
};
use velor_peer_monitoring_service_types::{
    request::PeerMonitoringServiceRequest, response::PeerMonitoringServiceResponse,
    PeerMonitoringServiceMessage,
};
use std::{sync::Arc, time::Duration};

/// The interface for sending peer monitoring service requests
/// and querying peer information.
#[derive(Clone, Debug)]
pub struct PeerMonitoringServiceClient<NetworkClient> {
    network_client: NetworkClient,
}

impl<NetworkClient: NetworkClientInterface<PeerMonitoringServiceMessage>>
    PeerMonitoringServiceClient<NetworkClient>
{
    pub fn new(network_client: NetworkClient) -> Self {
        Self { network_client }
    }

    /// Sends an RPC request to the specified peer with the given timeout
    pub async fn send_request(
        &self,
        recipient: PeerNetworkId,
        request: PeerMonitoringServiceRequest,
        timeout: Duration,
    ) -> Result<PeerMonitoringServiceResponse, Error> {
        let response = self
            .network_client
            .send_to_peer_rpc(
                PeerMonitoringServiceMessage::Request(request),
                timeout,
                recipient,
            )
            .await
            .map_err(|error| Error::NetworkError(error.to_string()))?;
        match response {
            PeerMonitoringServiceMessage::Response(Ok(response)) => Ok(response),
            PeerMonitoringServiceMessage::Response(Err(err)) => {
                Err(Error::PeerMonitoringServiceError(err))
            },
            PeerMonitoringServiceMessage::Request(request) => Err(Error::NetworkError(format!(
                "Got peer monitoring request instead of response! Request: {:?}",
                request
            ))),
        }
    }

    /// Returns the peers and metadata struct
    pub fn get_peers_and_metadata(&self) -> Arc<PeersAndMetadata> {
        self.network_client.get_peers_and_metadata()
    }
}

/// Sends a request to a specific peer
pub async fn send_request_to_peer(
    peer_monitoring_client: PeerMonitoringServiceClient<
        NetworkClient<PeerMonitoringServiceMessage>,
    >,
    peer_network_id: &PeerNetworkId,
    request_id: u64,
    request: PeerMonitoringServiceRequest,
    request_timeout_ms: u64,
) -> Result<PeerMonitoringServiceResponse, Error> {
    trace!(
        (LogSchema::new(LogEntry::SendRequest)
            .event(LogEvent::SendRequest)
            .request_type(request.get_label())
            .request_id(request_id)
            .peer(peer_network_id)
            .request(&request))
    );
    metrics::increment_request_counter(
        &metrics::SENT_REQUESTS,
        request.get_label(),
        peer_network_id,
    );

    // Send the request and process the result
    let result = peer_monitoring_client
        .send_request(
            *peer_network_id,
            request.clone(),
            Duration::from_millis(request_timeout_ms),
        )
        .await;
    match result {
        Ok(response) => {
            trace!(
                (LogSchema::new(LogEntry::SendRequest)
                    .event(LogEvent::ResponseSuccess)
                    .request_type(request.get_label())
                    .request_id(request_id)
                    .peer(peer_network_id))
            );
            metrics::increment_request_counter(
                &metrics::SUCCESS_RESPONSES,
                request.clone().get_label(),
                peer_network_id,
            );
            Ok(response)
        },
        Err(error) => {
            warn!(
                (LogSchema::new(LogEntry::SendRequest)
                    .event(LogEvent::ResponseError)
                    .request_type(request.get_label())
                    .request_id(request_id)
                    .peer(peer_network_id)
                    .error(&error))
            );
            metrics::increment_request_counter(
                &metrics::ERROR_RESPONSES,
                error.get_label(),
                peer_network_id,
            );
            Err(error)
        },
    }
}
