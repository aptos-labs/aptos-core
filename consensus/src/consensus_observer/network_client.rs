// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::consensus_observer::{
    error::Error,
    logging::{LogEntry, LogEvent, LogSchema},
    metrics,
    network_message::{
        ConsensusObserverDirectSend, ConsensusObserverMessage, ConsensusObserverRequest,
        ConsensusObserverResponse,
    },
};
use aptos_config::network_id::PeerNetworkId;
use aptos_logger::{debug, warn};
use aptos_network::application::{
    interface::{NetworkClient, NetworkClientInterface},
    storage::PeersAndMetadata,
};
use std::{sync::Arc, time::Duration};

/// The interface for sending consensus publisher and observer messages
#[derive(Clone, Debug)]
pub struct ConsensusObserverClient<NetworkClient> {
    network_client: NetworkClient,
}

impl<NetworkClient: NetworkClientInterface<ConsensusObserverMessage>>
    ConsensusObserverClient<NetworkClient>
{
    pub fn new(network_client: NetworkClient) -> Self {
        Self { network_client }
    }

    /// Sends a direct send message to the specified peer
    pub fn send_message(
        &self,
        recipient: PeerNetworkId,
        message: ConsensusObserverDirectSend,
    ) -> Result<(), Error> {
        self.network_client
            .send_to_peer(ConsensusObserverMessage::DirectSend(message), recipient)
            .map_err(|error| error.into())
    }

    /// Sends an RPC request to the specified peer with the given timeout
    pub async fn send_rpc_request(
        &self,
        recipient: PeerNetworkId,
        request: ConsensusObserverRequest,
        timeout: Duration,
    ) -> Result<ConsensusObserverResponse, Error> {
        // Send the request and wait for the response
        let response = self
            .network_client
            .send_to_peer_rpc(
                ConsensusObserverMessage::Request(request),
                timeout,
                recipient,
            )
            .await
            .map_err(|error| Error::NetworkError(error.to_string()))?;

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

/// Sends a direct send message to all peers
pub fn send_message_to_peers(
    consensus_observer_client: &ConsensusObserverClient<NetworkClient<ConsensusObserverMessage>>,
    peer_network_ids: Vec<PeerNetworkId>,
    message: ConsensusObserverDirectSend,
) {
    // TODO: Identify if we need to use broadcast, instead of sending to each peer individually

    // Send the message to each peer (individually). If an error is encountered,
    // it will be logged, and we will continue sending to the remaining peers.
    for peer_network_id in peer_network_ids {
        let _ = send_message_to_peer(consensus_observer_client, &peer_network_id, message.clone());
    }
}

/// Sends a direct send message to a specific peer
pub fn send_message_to_peer(
    consensus_observer_client: &ConsensusObserverClient<NetworkClient<ConsensusObserverMessage>>,
    peer_network_id: &PeerNetworkId,
    message: ConsensusObserverDirectSend,
) -> Result<(), Error> {
    // Increment the message counter
    let message_label = message.get_label();
    metrics::increment_request_counter(
        &metrics::DIRECT_SEND_SENT_MESSAGES,
        message_label,
        peer_network_id,
    );

    // Log the message being sent
    debug!(
        (LogSchema::new(LogEntry::SendDirectSendMessage)
            .event(LogEvent::SendDirectSendMessage)
            .message_content(&message.get_content())
            .message_type(message.get_label())
            .peer(peer_network_id))
    );

    // Send the message and process the result
    if let Err(error) = consensus_observer_client.send_message(*peer_network_id, message) {
        warn!(
            (LogSchema::new(LogEntry::SendDirectSendMessage)
                .event(LogEvent::NetworkError)
                .message_type(message_label)
                .peer(peer_network_id)
                .error(&error))
        );
        metrics::increment_request_counter(
            &metrics::DIRECT_SEND_ERRORS,
            error.get_label(),
            peer_network_id,
        );
        return Err(Error::NetworkError(error.to_string()));
    }

    Ok(())
}

/// Sends a RPC request to a specific peer
pub async fn send_rpc_request_to_peer(
    consensus_observer_client: &ConsensusObserverClient<NetworkClient<ConsensusObserverMessage>>,
    peer_network_id: &PeerNetworkId,
    request_id: u64,
    request: ConsensusObserverRequest,
    request_timeout_ms: u64,
) -> Result<ConsensusObserverResponse, Error> {
    // Increment the request counter
    metrics::increment_request_counter(
        &metrics::RPC_SENT_REQUESTS,
        request.get_label(),
        peer_network_id,
    );

    // Log the request being sent
    debug!(
        (LogSchema::new(LogEntry::SendRpcRequest)
            .event(LogEvent::SendRpcRequest)
            .request_type(request.get_label())
            .request_id(request_id)
            .peer(peer_network_id))
    );

    // Send the request and process the result
    let result = consensus_observer_client
        .send_rpc_request(
            *peer_network_id,
            request.clone(),
            Duration::from_millis(request_timeout_ms),
        )
        .await;
    match result {
        Ok(response) => {
            metrics::increment_request_counter(
                &metrics::RPC_SUCCESS_RESPONSES,
                request.clone().get_label(),
                peer_network_id,
            );
            Ok(response)
        },
        Err(error) => {
            warn!(
                (LogSchema::new(LogEntry::SendRpcRequest)
                    .event(LogEvent::InvalidRpcResponse)
                    .request_type(request.get_label())
                    .request_id(request_id)
                    .peer(peer_network_id)
                    .error(&error))
            );
            metrics::increment_request_counter(
                &metrics::DIRECT_SEND_ERRORS,
                error.get_label(),
                peer_network_id,
            );
            Err(error)
        },
    }
}
