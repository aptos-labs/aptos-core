// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::consensus_observer::{
    common::{
        error::Error,
        logging::{LogEntry, LogEvent, LogSchema},
        metrics,
    },
    network::observer_message::{
        ConsensusObserverDirectSend, ConsensusObserverMessage, ConsensusObserverRequest,
        ConsensusObserverResponse,
    },
};
use velor_config::network_id::PeerNetworkId;
use velor_logger::{debug, warn};
use velor_network::application::{interface::NetworkClientInterface, storage::PeersAndMetadata};
use velor_time_service::{TimeService, TimeServiceTrait};
use bytes::Bytes;
use rand::Rng;
use std::{sync::Arc, time::Duration};

/// The interface for sending consensus publisher and observer messages
#[derive(Clone, Debug)]
pub struct ConsensusObserverClient<NetworkClient> {
    network_client: NetworkClient,
    time_service: TimeService,
}

impl<NetworkClient: NetworkClientInterface<ConsensusObserverMessage>>
    ConsensusObserverClient<NetworkClient>
{
    pub fn new(network_client: NetworkClient) -> Self {
        let time_service = TimeService::real();
        Self {
            network_client,
            time_service,
        }
    }

    /// Sends an already serialized (direct send) message to a specific peer
    pub fn send_serialized_message_to_peer(
        &self,
        peer_network_id: &PeerNetworkId,
        message: Bytes,
        message_label: &str,
    ) -> Result<(), Error> {
        // Increment the message counter
        metrics::increment_counter(
            &metrics::PUBLISHER_SENT_MESSAGES,
            message_label,
            peer_network_id,
        );

        // Log the message being sent
        debug!(LogSchema::new(LogEntry::SendDirectSendMessage)
            .event(LogEvent::SendDirectSendMessage)
            .message_type(message_label)
            .peer(peer_network_id));

        // Send the message
        let result = self
            .network_client
            .send_to_peer_raw(message, *peer_network_id)
            .map_err(|error| Error::NetworkError(error.to_string()));

        // Process any error results
        if let Err(error) = result {
            // Log the failed send
            warn!(LogSchema::new(LogEntry::SendDirectSendMessage)
                .event(LogEvent::NetworkError)
                .message_type(message_label)
                .peer(peer_network_id)
                .message(&format!("Failed to send message: {:?}", error)));

            // Update the direct send error metrics
            metrics::increment_counter(
                &metrics::PUBLISHER_SENT_MESSAGE_ERRORS,
                error.get_label(),
                peer_network_id,
            );

            Err(Error::NetworkError(error.to_string()))
        } else {
            Ok(())
        }
    }

    /// Serializes the given message into bytes for the specified peer
    pub fn serialize_message_for_peer(
        &self,
        peer_network_id: &PeerNetworkId,
        message: ConsensusObserverDirectSend,
    ) -> Result<Bytes, Error> {
        // Serialize the message into bytes
        let message_label = message.get_label();
        let message = ConsensusObserverMessage::DirectSend(message);
        let result = self
            .network_client
            .to_bytes_by_protocol(vec![*peer_network_id], message)
            .map_err(|error| Error::NetworkError(error.to_string()));

        // Process the serialization result
        match result {
            Ok(peer_to_serialized_bytes) => {
                // Get the serialized bytes for the peer
                let serialized_bytes =
                    peer_to_serialized_bytes
                        .get(peer_network_id)
                        .ok_or_else(|| {
                            Error::NetworkError(format!(
                                "Failed to get serialized bytes for peer: {:?}!",
                                peer_network_id
                            ))
                        })?;

                Ok(serialized_bytes.clone())
            },
            Err(error) => {
                // Log the serialization error
                warn!(LogSchema::new(LogEntry::SendDirectSendMessage)
                    .event(LogEvent::NetworkError)
                    .message_type(message_label)
                    .peer(peer_network_id)
                    .message(&format!("Failed to serialize message: {:?}", error)));

                // Update the direct send error metrics
                metrics::increment_counter(
                    &metrics::PUBLISHER_SENT_MESSAGE_ERRORS,
                    error.get_label(),
                    peer_network_id,
                );

                Err(Error::NetworkError(error.to_string()))
            },
        }
    }

    /// Sends a RPC request to a specific peer and returns the response
    pub async fn send_rpc_request_to_peer(
        &self,
        peer_network_id: &PeerNetworkId,
        request: ConsensusObserverRequest,
        request_timeout_ms: u64,
    ) -> Result<ConsensusObserverResponse, Error> {
        // Generate a random request ID
        let request_id = rand::thread_rng().gen();

        // Increment the request counter
        metrics::increment_counter(
            &metrics::OBSERVER_SENT_REQUESTS,
            request.get_label(),
            peer_network_id,
        );

        // Log the request being sent
        debug!(LogSchema::new(LogEntry::SendRpcRequest)
            .event(LogEvent::SendRpcRequest)
            .request_type(request.get_label())
            .request_id(request_id)
            .peer(peer_network_id));

        // Send the request and wait for the response
        let request_label = request.get_label();
        let result = self
            .send_rpc_request(
                *peer_network_id,
                request,
                Duration::from_millis(request_timeout_ms),
            )
            .await;

        // Process the response
        match result {
            Ok(consensus_observer_response) => {
                // Update the RPC success metrics
                metrics::increment_counter(
                    &metrics::OBSERVER_RECEIVED_MESSAGE_RESPONSES,
                    request_label,
                    peer_network_id,
                );

                Ok(consensus_observer_response)
            },
            Err(error) => {
                // Log the failed RPC request
                warn!(LogSchema::new(LogEntry::SendRpcRequest)
                    .event(LogEvent::InvalidRpcResponse)
                    .request_type(request_label)
                    .request_id(request_id)
                    .peer(peer_network_id)
                    .error(&error));

                // Update the RPC error metrics
                metrics::increment_counter(
                    &metrics::OBSERVER_SENT_MESSAGE_ERRORS,
                    error.get_label(),
                    peer_network_id,
                );

                Err(error)
            },
        }
    }

    /// Sends an RPC request to the specified peer with the given timeout
    async fn send_rpc_request(
        &self,
        peer_network_id: PeerNetworkId,
        request: ConsensusObserverRequest,
        timeout: Duration,
    ) -> Result<ConsensusObserverResponse, Error> {
        // Start the request timer
        let start_time = self.time_service.now();

        // Send the request and wait for the response
        let request_label = request.get_label();
        let response = self
            .network_client
            .send_to_peer_rpc(
                ConsensusObserverMessage::Request(request),
                timeout,
                peer_network_id,
            )
            .await
            .map_err(|error| Error::NetworkError(error.to_string()))?;

        // Stop the timer and calculate the duration
        let request_duration_secs = start_time.elapsed().as_secs_f64();

        // Update the RPC request metrics
        metrics::observe_value_with_label(
            &metrics::OBSERVER_REQUEST_LATENCIES,
            request_label,
            &peer_network_id,
            request_duration_secs,
        );

        // Process the response
        match response {
            ConsensusObserverMessage::Response(response) => Ok(response),
            ConsensusObserverMessage::Request(request) => Err(Error::NetworkError(format!(
                "Got consensus observer request instead of response! Request: {:?}",
                request
            ))),
            ConsensusObserverMessage::DirectSend(message) => Err(Error::NetworkError(format!(
                "Got consensus observer direct send message instead of response! Message: {:?}",
                message
            ))),
        }
    }

    /// Returns the peers and metadata struct
    pub fn get_peers_and_metadata(&self) -> Arc<PeersAndMetadata> {
        self.network_client.get_peers_and_metadata()
    }
}
