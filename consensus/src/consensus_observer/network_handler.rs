// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::consensus_observer::{
    logging::{LogEntry, LogSchema},
    network_events::{ConsensusObserverNetworkEvents, NetworkMessage, ResponseSender},
    network_message::{
        ConsensusObserverDirectSend, ConsensusObserverMessage, ConsensusObserverRequest,
    },
};
use aptos_channels::{
    aptos_channel,
    aptos_channel::{Receiver, Sender},
    message_queues::QueueStyle,
};
use aptos_config::{config::ConsensusObserverConfig, network_id::PeerNetworkId};
use aptos_logger::{error, info, warn};
use futures::StreamExt;

/// A simple struct that holds a message to be sent to the consensus observer
pub struct ConsensusObserverNetworkMessage {
    peer_network_id: PeerNetworkId,
    message: ConsensusObserverDirectSend,
}

impl ConsensusObserverNetworkMessage {
    pub fn new(peer_network_id: PeerNetworkId, message: ConsensusObserverDirectSend) -> Self {
        Self {
            peer_network_id,
            message,
        }
    }

    /// Consumes and unpacks the message into its parts
    pub fn into_parts(self) -> (PeerNetworkId, ConsensusObserverDirectSend) {
        (self.peer_network_id, self.message)
    }
}

/// A simple struct that holds a message to be sent to the consensus publisher
pub struct ConsensusPublisherNetworkMessage {
    peer_network_id: PeerNetworkId,
    message: ConsensusObserverRequest,
    response_sender: ResponseSender,
}

impl ConsensusPublisherNetworkMessage {
    pub fn new(
        peer_network_id: PeerNetworkId,
        message: ConsensusObserverRequest,
        response_sender: ResponseSender,
    ) -> Self {
        Self {
            peer_network_id,
            message,
            response_sender,
        }
    }

    /// Consumes and unpacks the message into its parts
    pub fn into_parts(self) -> (PeerNetworkId, ConsensusObserverRequest, ResponseSender) {
        (self.peer_network_id, self.message, self.response_sender)
    }
}

/// The network message handler that forwards messages to the consensus
/// observer and publisher, depending on the destination.
pub struct ConsensusObserverNetworkHandler {
    // The consensus observer config
    consensus_observer_config: ConsensusObserverConfig,

    // The stream of network events
    network_service_events: ConsensusObserverNetworkEvents,

    // The sender for consensus observer messages
    observer_message_sender: Sender<(), ConsensusObserverNetworkMessage>,

    // The sender for consensus publisher messages
    publisher_message_sender: Sender<(), ConsensusPublisherNetworkMessage>,
}

impl ConsensusObserverNetworkHandler {
    pub fn new(
        consensus_observer_config: ConsensusObserverConfig,
        network_service_events: ConsensusObserverNetworkEvents,
    ) -> (
        Self,
        Receiver<(), ConsensusObserverNetworkMessage>,
        Receiver<(), ConsensusPublisherNetworkMessage>,
    ) {
        // Create a channel for sending consensus observer messages
        let (observer_message_sender, observer_message_receiver) = aptos_channel::new(
            QueueStyle::FIFO,
            consensus_observer_config.max_network_channel_size as usize,
            None,
        );

        // Create a channel for sending consensus publisher messages
        let (publisher_message_sender, publisher_message_receiver) = aptos_channel::new(
            QueueStyle::FIFO,
            consensus_observer_config.max_network_channel_size as usize,
            None,
        );

        // Create the network handler
        let network_handler = Self {
            consensus_observer_config,
            network_service_events,
            observer_message_sender,
            publisher_message_sender,
        };

        (
            network_handler,
            observer_message_receiver,
            publisher_message_receiver,
        )
    }

    /// Starts the network handler that forwards messages to the observer and publisher
    pub async fn start(mut self) {
        info!(LogSchema::new(LogEntry::ConsensusObserver)
            .message("Starting the consensus observer network handler!"));

        // Start the network message handler loop
        loop {
            tokio::select! {
                Some(network_message) = self.network_service_events.next() => {
                    // Unpack the network message
                    let NetworkMessage {
                        peer_network_id,
                        protocol_id: _,
                        consensus_observer_message,
                        response_sender,
                    } = network_message;

                    // Process the consensus observer message
                    match consensus_observer_message {
                        ConsensusObserverMessage::DirectSend(message) => {
                            self.handle_observer_message(peer_network_id, message);
                        },
                        ConsensusObserverMessage::Request(request) => {
                            self.handle_publisher_message(peer_network_id, request, response_sender);
                        },
                        ConsensusObserverMessage::Response(_) => {
                            warn!(LogSchema::new(LogEntry::ConsensusObserver)
                                .message(&format!("Received unexpected response from peer: {}", peer_network_id)));
                        },
                    }
                }
            }
        }
    }

    /// Handles an observer message by forwarding it to the consensus observer
    fn handle_observer_message(
        &mut self,
        peer_network_id: PeerNetworkId,
        message: ConsensusObserverDirectSend,
    ) {
        // Drop the message if the observer is not enabled
        if !self.consensus_observer_config.observer_enabled {
            return;
        }

        // Create the consensus observer message
        let network_message = ConsensusObserverNetworkMessage::new(peer_network_id, message);

        // Send the message to the consensus observer
        if let Err(error) = self.observer_message_sender.push((), network_message) {
            error!(
                LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                    "Failed to forward the observer message to the consensus observer! Error: {:?}",
                    error
                ))
            );
        }
    }

    /// Handles a publisher message by forwarding it to the consensus publisher
    fn handle_publisher_message(
        &mut self,
        peer_network_id: PeerNetworkId,
        request: ConsensusObserverRequest,
        response_sender: Option<ResponseSender>,
    ) {
        // Drop the message if the publisher is not enabled
        if !self.consensus_observer_config.publisher_enabled {
            return;
        }

        // Ensure that the response sender is present
        let response_sender = match response_sender {
            Some(response_sender) => response_sender,
            None => {
                error!(
                    LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                        "Missing response sender for RCP request: {:?}",
                        request
                    ))
                );
                return; // Something has gone wrong!
            },
        };

        // Create the consensus publisher message
        let network_message =
            ConsensusPublisherNetworkMessage::new(peer_network_id, request, response_sender);

        // Send the message to the consensus publisher
        if let Err(error) = self.publisher_message_sender.push((), network_message) {
            error!(
                LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                    "Failed to forward the publisher request to the consensus publisher! Error: {:?}",
                    error
                ))
            );
        }
    }
}
