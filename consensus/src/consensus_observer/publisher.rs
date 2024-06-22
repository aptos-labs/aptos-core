// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::consensus_observer::{
    logging::{LogEntry, LogEvent, LogSchema},
    metrics,
    network_client::ConsensusObserverClient,
    network_events::ResponseSender,
    network_message::{
        ConsensusObserverDirectSend, ConsensusObserverMessage, ConsensusObserverRequest,
        ConsensusObserverResponse,
    },
};
use aptos_config::{config::ConsensusObserverConfig, network_id::PeerNetworkId};
use aptos_infallible::RwLock;
use aptos_logger::{info, warn};
use aptos_network::application::interface::NetworkClient;
use futures::{SinkExt, StreamExt};
use futures_channel::mpsc;
use std::{collections::HashSet, sync::Arc, time::Duration};
use tokio::time::interval;
use tokio_stream::wrappers::IntervalStream;

/// The consensus publisher sends consensus updates to downstream observers
#[derive(Clone)]
pub struct ConsensusPublisher {
    // The consensus observer client to send network messages
    consensus_observer_client:
        Arc<ConsensusObserverClient<NetworkClient<ConsensusObserverMessage>>>,

    // The configuration for the consensus observer
    consensus_observer_config: ConsensusObserverConfig,

    // The set of active subscribers that have subscribed to consensus updates
    active_subscribers: Arc<RwLock<HashSet<PeerNetworkId>>>,

    // The sender for outbound network messages
    outbound_message_sender: mpsc::Sender<(PeerNetworkId, ConsensusObserverDirectSend)>,
}

impl ConsensusPublisher {
    pub fn new(
        network_client: NetworkClient<ConsensusObserverMessage>,
        consensus_observer_config: ConsensusObserverConfig,
    ) -> (
        Self,
        mpsc::Receiver<(PeerNetworkId, ConsensusObserverDirectSend)>,
    ) {
        // Create the outbound message sender and receiver
        let max_network_channel_size = consensus_observer_config.max_network_channel_size as usize;
        let (outbound_message_sender, outbound_message_receiver) =
            mpsc::channel(max_network_channel_size);

        // Create the consensus publisher
        let consensus_publisher = Self {
            consensus_observer_client: Arc::new(ConsensusObserverClient::new(network_client)),
            consensus_observer_config,
            active_subscribers: Arc::new(RwLock::new(HashSet::new())),
            outbound_message_sender,
        };

        // Return the publisher and the outbound message receiver
        (consensus_publisher, outbound_message_receiver)
    }

    /// Garbage collect inactive subscriptions by removing peers that are no longer connected
    fn garbage_collect_subscriptions(&self) {
        // Get the set of active subscribers
        let active_subscribers = self.active_subscribers.read().clone();

        // Get the connected peers and metadata
        let peers_and_metadata = self.consensus_observer_client.get_peers_and_metadata();
        let connected_peers_and_metadata =
            match peers_and_metadata.get_connected_peers_and_metadata() {
                Ok(connected_peers_and_metadata) => connected_peers_and_metadata,
                Err(error) => {
                    // We failed to get the connected peers and metadata
                    warn!(LogSchema::new(LogEntry::ConsensusPublisher)
                        .event(LogEvent::UnexpectedError)
                        .message(&format!(
                            "Failed to get connected peers and metadata! Error: {:?}",
                            error
                        )));
                    return;
                },
            };

        // Identify the active subscribers that are no longer connected
        let connected_peers: HashSet<PeerNetworkId> =
            connected_peers_and_metadata.keys().cloned().collect();
        let disconnected_subscribers: HashSet<PeerNetworkId> = active_subscribers
            .difference(&connected_peers)
            .cloned()
            .collect();

        // Remove any subscriptions from peers that are no longer connected
        for peer_network_id in &disconnected_subscribers {
            self.active_subscribers.write().remove(peer_network_id);
            info!(LogSchema::new(LogEntry::ConsensusPublisher)
                .event(LogEvent::Subscription)
                .message(&format!(
                    "Removed peer subscription due to disconnection! Peer: {:?}",
                    peer_network_id
                )));
        }

        // Update the number of active subscribers for each network
        let active_subscribers = self.active_subscribers.read().clone();
        for network_id in peers_and_metadata.get_registered_networks() {
            // Calculate the number of active subscribers for the network
            let num_active_subscribers = active_subscribers
                .iter()
                .filter(|peer_network_id| peer_network_id.network_id() == network_id)
                .count() as i64;

            // Update the active subscriber metric
            metrics::set_gauge(
                &metrics::PUBLISHER_NUM_ACTIVE_SUBSCRIBERS,
                &network_id,
                num_active_subscribers,
            );
        }
    }

    /// Returns a copy of the consensus observer client
    pub fn get_consensus_observer_client(
        &self,
    ) -> Arc<ConsensusObserverClient<NetworkClient<ConsensusObserverMessage>>> {
        self.consensus_observer_client.clone()
    }

    /// Handles a subscription message from a peer
    pub fn handle_subscription_request(
        &self,
        peer_network_id: &PeerNetworkId,
        request: ConsensusObserverRequest,
        response_sender: ResponseSender,
    ) {
        // Update the RPC request counter
        metrics::increment_request_counter(
            &metrics::PUBLISHER_RECEIVED_REQUESTS,
            request.get_label(),
            peer_network_id,
        );

        // Handle the request
        match request {
            ConsensusObserverRequest::Subscribe => {
                // Add the peer to the set of active subscribers
                self.active_subscribers.write().insert(*peer_network_id);
                info!(LogSchema::new(LogEntry::ConsensusPublisher)
                    .event(LogEvent::Subscription)
                    .message(&format!(
                        "New peer subscribed to consensus updates! Peer: {:?}",
                        peer_network_id
                    )));

                // Send a simple subscription ACK
                response_sender.send(ConsensusObserverResponse::SubscribeAck);
            },
            ConsensusObserverRequest::Unsubscribe => {
                // Remove the peer from the set of active subscribers
                self.active_subscribers.write().remove(peer_network_id);
                info!(LogSchema::new(LogEntry::ConsensusPublisher)
                    .event(LogEvent::Subscription)
                    .message(&format!(
                        "Peer unsubscribed from consensus updates! Peer: {:?}",
                        peer_network_id
                    )));

                // Send a simple unsubscription ACK
                response_sender.send(ConsensusObserverResponse::UnsubscribeAck);
            },
        }
    }

    /// Publishes a direct send message to all active subscribers
    pub async fn publish_message(&self, message: ConsensusObserverDirectSend) {
        // Get the set of active subscribers
        let active_subscribers = self.active_subscribers.read().clone();

        // Send the message to all active subscribers
        for peer_network_id in &active_subscribers {
            // Send the message to the outbound receiver for publishing
            let mut outbound_message_sender = self.outbound_message_sender.clone();
            if let Err(error) = outbound_message_sender
                .send((*peer_network_id, message.clone()))
                .await
            {
                // The message send failed
                warn!(LogSchema::new(LogEntry::ConsensusPublisher)
                    .event(LogEvent::SendDirectSendMessage)
                    .message(&format!(
                        "Failed to send outbound message to the receiver for peer {:?}! Error: {:?}",
                        peer_network_id, error
                    )));
            }
        }
    }

    /// Starts the consensus publisher
    pub async fn start(
        self,
        outbound_message_receiver: mpsc::Receiver<(PeerNetworkId, ConsensusObserverDirectSend)>,
    ) {
        // Spawn the message serializer and sender
        spawn_message_serializer_and_sender(
            self.consensus_observer_client.clone(),
            self.consensus_observer_config,
            outbound_message_receiver,
        );

        // Create a garbage collection ticker
        let mut garbage_collection_interval = IntervalStream::new(interval(Duration::from_millis(
            self.consensus_observer_config
                .garbage_collection_interval_ms,
        )))
        .fuse();

        // Start the publisher garbage collection loop
        info!(LogSchema::new(LogEntry::ConsensusPublisher)
            .message("Starting the consensus publisher garbage collection loop!"));
        loop {
            tokio::select! {
                _ = garbage_collection_interval.select_next_some() => {
                    // Perform garbage collection
                    self.garbage_collect_subscriptions();
                },
            }
        }
    }
}

/// Spawns a message serialization task that serializes outbound publisher
/// messages in parallel but guarantees in order sends to the receiver.
fn spawn_message_serializer_and_sender(
    consensus_observer_client: Arc<
        ConsensusObserverClient<NetworkClient<ConsensusObserverMessage>>,
    >,
    consensus_observer_config: ConsensusObserverConfig,
    outbound_message_receiver: mpsc::Receiver<(PeerNetworkId, ConsensusObserverDirectSend)>,
) {
    tokio::spawn(async move {
        // Create the message serialization task
        let consensus_observer_client_clone = consensus_observer_client.clone();
        let serialization_task =
            outbound_message_receiver.map(move |(peer_network_id, message)| {
                // Spawn a new blocking task to serialize the message
                let consensus_observer_client_clone = consensus_observer_client_clone.clone();
                tokio::task::spawn_blocking(move || {
                    let message_label = message.get_label();
                    let serialized_message = consensus_observer_client_clone
                        .serialize_message_for_peer(&peer_network_id, message);
                    (peer_network_id, serialized_message, message_label)
                })
            });

        // Execute the serialization task with in-order buffering
        let consensus_observer_client_clone = consensus_observer_client.clone();
        serialization_task
            .buffered(consensus_observer_config.max_parallel_serialization_tasks)
            .map(|serialization_result| {
                // Attempt to send the serialized message to the peer
                match serialization_result {
                    Ok((peer_network_id, serialized_message, message_label)) => {
                        match serialized_message {
                            Ok(serialized_message) => {
                                // Send the serialized message to the peer
                                if let Err(error) = consensus_observer_client_clone
                                    .send_serialized_message_to_peer(
                                        &peer_network_id,
                                        serialized_message,
                                        message_label,
                                    )
                                {
                                    // We failed to send the message
                                    warn!(LogSchema::new(LogEntry::ConsensusPublisher)
                                        .event(LogEvent::SendDirectSendMessage)
                                        .message(&format!(
                                            "Failed to send message to peer: {:?}. Error: {:?}",
                                            peer_network_id, error
                                        )));
                                }
                            },
                            Err(error) => {
                                // We failed to serialize the message
                                warn!(LogSchema::new(LogEntry::ConsensusPublisher)
                                    .event(LogEvent::SendDirectSendMessage)
                                    .message(&format!(
                                        "Failed to serialize message for peer: {:?}. Error: {:?}",
                                        peer_network_id, error
                                    )));
                            },
                        }
                    },
                    Err(error) => {
                        // We failed to spawn the serialization task
                        warn!(LogSchema::new(LogEntry::ConsensusPublisher)
                            .event(LogEvent::SendDirectSendMessage)
                            .message(&format!("Failed to spawn the serializer task: {:?}", error)));
                    },
                }
            })
            .collect::<()>()
            .await;
    });
}
