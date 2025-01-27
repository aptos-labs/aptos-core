// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::consensus_observer::{
    common::{
        logging::{LogEntry, LogEvent, LogSchema},
        metrics,
    },
    network::{
        network_handler::ConsensusPublisherNetworkMessage,
        observer_client::ConsensusObserverClient,
        observer_message::{
            ConsensusObserverDirectSend, ConsensusObserverMessage, ConsensusObserverRequest,
            ConsensusObserverResponse,
        },
    },
};
use aptos_channels::aptos_channel::Receiver;
use aptos_config::{config::ConsensusObserverConfig, network_id::PeerNetworkId};
use aptos_infallible::RwLock;
use aptos_logger::{error, info, warn};
use aptos_network::application::interface::NetworkClient;
use futures::StreamExt;
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
        consensus_observer_config: ConsensusObserverConfig,
        consensus_observer_client: Arc<
            ConsensusObserverClient<NetworkClient<ConsensusObserverMessage>>,
        >,
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
            consensus_observer_client,
            consensus_observer_config,
            active_subscribers: Arc::new(RwLock::new(HashSet::new())),
            outbound_message_sender,
        };

        // Return the publisher and the outbound message receiver
        (consensus_publisher, outbound_message_receiver)
    }

    #[cfg(test)]
    /// Creates a new consensus publisher with the given active subscribers
    pub fn new_with_active_subscribers(
        consensus_observer_config: ConsensusObserverConfig,
        consensus_observer_client: Arc<
            ConsensusObserverClient<NetworkClient<ConsensusObserverMessage>>,
        >,
        active_subscribers: HashSet<PeerNetworkId>,
    ) -> Self {
        // Create the consensus publisher
        let (consensus_publisher, _) =
            ConsensusPublisher::new(consensus_observer_config, consensus_observer_client);

        // Update the active subscribers
        *consensus_publisher.active_subscribers.write() = active_subscribers;

        // Return the publisher
        consensus_publisher
    }

    /// Adds the given subscriber to the set of active subscribers
    fn add_active_subscriber(&self, peer_network_id: PeerNetworkId) {
        self.active_subscribers.write().insert(peer_network_id);
    }

    /// Garbage collect inactive subscriptions by removing peers that are no longer connected
    fn garbage_collect_subscriptions(&self) {
        // Get the set of active subscribers
        let active_subscribers = self.get_active_subscribers();

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
            self.remove_active_subscriber(peer_network_id);
            info!(LogSchema::new(LogEntry::ConsensusPublisher)
                .event(LogEvent::Subscription)
                .message(&format!(
                    "Removed peer subscription due to disconnection! Peer: {:?}",
                    peer_network_id
                )));
        }

        // Update the number of active subscribers for each network
        let active_subscribers = self.get_active_subscribers();
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

    /// Returns a clone of the currently active subscribers
    pub fn get_active_subscribers(&self) -> HashSet<PeerNetworkId> {
        self.active_subscribers.read().clone()
    }

    /// Removes the given subscriber from the set of active subscribers
    fn remove_active_subscriber(&self, peer_network_id: &PeerNetworkId) {
        self.active_subscribers.write().remove(peer_network_id);
    }

    /// Processes a network message received by the consensus publisher
    fn process_network_message(&self, network_message: ConsensusPublisherNetworkMessage) {
        // Unpack the network message
        let (peer_network_id, message, response_sender) = network_message.into_parts();

        // Update the RPC request counter
        metrics::increment_counter(
            &metrics::PUBLISHER_RECEIVED_REQUESTS,
            message.get_label(),
            &peer_network_id,
        );

        // Handle the message
        match message {
            ConsensusObserverRequest::Subscribe => {
                // Add the peer to the set of active subscribers
                self.add_active_subscriber(peer_network_id);
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
                self.remove_active_subscriber(&peer_network_id);
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

    /// Publishes a direct send message to all active subscribers. Note: this method
    /// is non-blocking (to avoid blocking callers during publishing, e.g., consensus).
    pub fn publish_message(&self, message: ConsensusObserverDirectSend) {
        // Get the active subscribers
        let active_subscribers = self.get_active_subscribers();

        // Send the message to all active subscribers
        for peer_network_id in &active_subscribers {
            // Send the message to the outbound receiver for publishing
            let mut outbound_message_sender = self.outbound_message_sender.clone();
            if let Err(error) =
                outbound_message_sender.try_send((*peer_network_id, message.clone()))
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
        mut publisher_message_receiver: Receiver<(), ConsensusPublisherNetworkMessage>,
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
                Some(network_message) = publisher_message_receiver.next() => {
                    self.process_network_message(network_message);
                },
                _ = garbage_collection_interval.select_next_some() => {
                    self.garbage_collect_subscriptions();
                },
                else => {
                    break; // Exit the consensus publisher loop
                }
            }
        }

        // Log the exit of the consensus publisher loop
        error!(LogSchema::new(LogEntry::ConsensusPublisher)
            .message("The consensus publisher loop exited unexpectedly!"));
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::consensus_observer::network::{
        network_events::ResponseSender, observer_message::BlockTransactionPayload,
    };
    use aptos_config::network_id::NetworkId;
    use aptos_crypto::HashValue;
    use aptos_network::{
        application::{metadata::ConnectionState, storage::PeersAndMetadata},
        transport::ConnectionMetadata,
    };
    use aptos_types::{
        aggregate_signature::AggregateSignature,
        block_info::BlockInfo,
        ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
        PeerId,
    };
    use futures::FutureExt;
    use maplit::hashmap;
    use tokio_stream::StreamExt;

    #[test]
    pub fn test_garbage_collect_subscriptions() {
        // Create a network client
        let network_id = NetworkId::Public;
        let peers_and_metadata = PeersAndMetadata::new(&[network_id]);
        let network_client =
            NetworkClient::new(vec![], vec![], hashmap![], peers_and_metadata.clone());
        let consensus_observer_client = Arc::new(ConsensusObserverClient::new(network_client));

        // Create a consensus publisher
        let (consensus_publisher, _) = ConsensusPublisher::new(
            ConsensusObserverConfig::default(),
            consensus_observer_client,
        );

        // Add a peer to the peers and metadata
        let peer_network_id_1 = PeerNetworkId::new(network_id, PeerId::random());
        let connection_metadata = ConnectionMetadata::mock(peer_network_id_1.peer_id());
        peers_and_metadata
            .insert_connection_metadata(peer_network_id_1, connection_metadata)
            .unwrap();

        // Add the peer to the active subscribers
        process_subscription_for_peer(&consensus_publisher, &peer_network_id_1);

        // Garbage collect the subscriptions and verify that the peer is still an active subscriber
        consensus_publisher.garbage_collect_subscriptions();
        verify_active_subscribers(&consensus_publisher, 1, vec![&peer_network_id_1], vec![]);

        // Add another peer to the active subscribers
        let peer_network_id_2 = PeerNetworkId::new(network_id, PeerId::random());
        process_subscription_for_peer(&consensus_publisher, &peer_network_id_2);

        // Garbage collect the subscriptions and verify that the second peer
        // is removed but not the first (we have no metadata for the second peer).
        consensus_publisher.garbage_collect_subscriptions();
        verify_active_subscribers(&consensus_publisher, 1, vec![&peer_network_id_1], vec![
            &peer_network_id_2,
        ]);

        // Add another peer to the peers and metadata
        let peer_network_id_3 = PeerNetworkId::new(network_id, PeerId::random());
        let connection_metadata = ConnectionMetadata::mock(peer_network_id_3.peer_id());
        peers_and_metadata
            .insert_connection_metadata(peer_network_id_3, connection_metadata)
            .unwrap();

        // Add the peer to the active subscribers
        process_subscription_for_peer(&consensus_publisher, &peer_network_id_3);

        // Garbage collect the subscriptions and verify that both peers are active
        consensus_publisher.garbage_collect_subscriptions();
        verify_active_subscribers(
            &consensus_publisher,
            2,
            vec![&peer_network_id_1, &peer_network_id_3],
            vec![],
        );

        // Update the connection state for the first peer (to disconnected)
        peers_and_metadata
            .update_connection_state(peer_network_id_1, ConnectionState::Disconnected)
            .unwrap();

        // Garbage collect the subscriptions and verify that the first peer is removed
        consensus_publisher.garbage_collect_subscriptions();
        verify_active_subscribers(&consensus_publisher, 1, vec![&peer_network_id_3], vec![
            &peer_network_id_1,
        ]);
    }

    #[test]
    fn test_handle_subscription_request() {
        // Create a network client
        let network_id = NetworkId::Public;
        let peers_and_metadata = PeersAndMetadata::new(&[network_id]);
        let network_client =
            NetworkClient::new(vec![], vec![], hashmap![], peers_and_metadata.clone());
        let consensus_observer_client = Arc::new(ConsensusObserverClient::new(network_client));

        // Create a consensus publisher
        let (consensus_publisher, _) = ConsensusPublisher::new(
            ConsensusObserverConfig::default(),
            consensus_observer_client,
        );

        // Subscribe a new peer to consensus updates and verify the subscription
        let peer_network_id_1 = PeerNetworkId::new(network_id, PeerId::random());
        process_subscription_for_peer(&consensus_publisher, &peer_network_id_1);
        verify_active_subscribers(&consensus_publisher, 1, vec![&peer_network_id_1], vec![]);

        // Subscribe the same peer again and verify that the subscription is still active
        process_subscription_for_peer(&consensus_publisher, &peer_network_id_1);
        verify_active_subscribers(&consensus_publisher, 1, vec![&peer_network_id_1], vec![]);

        // Subscribe another peer to consensus updates and verify the subscription
        let peer_network_id_2 = PeerNetworkId::new(network_id, PeerId::random());
        process_subscription_for_peer(&consensus_publisher, &peer_network_id_2);
        verify_active_subscribers(
            &consensus_publisher,
            2,
            vec![&peer_network_id_1, &peer_network_id_2],
            vec![],
        );

        // Unsubscribe the first peer from consensus updates and verify the unsubscription
        process_unsubscription_for_peer(&consensus_publisher, &peer_network_id_1);
        verify_active_subscribers(&consensus_publisher, 1, vec![&peer_network_id_2], vec![
            &peer_network_id_1,
        ]);

        // Unsubscribe the first peer again and verify that the subscription is removed
        process_unsubscription_for_peer(&consensus_publisher, &peer_network_id_1);
        verify_active_subscribers(&consensus_publisher, 1, vec![&peer_network_id_2], vec![
            &peer_network_id_1,
        ]);

        // Unsubscribe the second peer and verify that the subscription is removed
        process_unsubscription_for_peer(&consensus_publisher, &peer_network_id_2);
        verify_active_subscribers(&consensus_publisher, 0, vec![], vec![
            &peer_network_id_1,
            &peer_network_id_2,
        ]);
    }

    #[tokio::test]
    async fn test_publish_message() {
        // Create a network client
        let network_id = NetworkId::Public;
        let peers_and_metadata = PeersAndMetadata::new(&[network_id]);
        let network_client =
            NetworkClient::new(vec![], vec![], hashmap![], peers_and_metadata.clone());
        let consensus_observer_client = Arc::new(ConsensusObserverClient::new(network_client));

        // Create a consensus publisher
        let (consensus_publisher, mut outbound_message_receiver) = ConsensusPublisher::new(
            ConsensusObserverConfig::default(),
            consensus_observer_client,
        );

        // Subscribe a new peer to consensus updates
        let peer_network_id_1 = PeerNetworkId::new(network_id, PeerId::random());
        process_subscription_for_peer(&consensus_publisher, &peer_network_id_1);

        // Publish a message to the active subscribers
        let ordered_block_message = ConsensusObserverMessage::new_ordered_block_message(
            vec![],
            LedgerInfoWithSignatures::new(
                LedgerInfo::new(BlockInfo::empty(), HashValue::zero()),
                AggregateSignature::empty(),
            ),
        );
        consensus_publisher.publish_message(ordered_block_message.clone());

        // Verify that the message was sent to the outbound message receiver
        let (peer_network_id, message) = outbound_message_receiver.next().await.unwrap();
        assert_eq!(peer_network_id, peer_network_id_1);
        assert_eq!(message, ordered_block_message);

        // Add several peers to the active subscribers
        let mut additional_peer_network_ids = vec![];
        for _ in 0..10 {
            let peer_network_id = PeerNetworkId::new(network_id, PeerId::random());
            process_subscription_for_peer(&consensus_publisher, &peer_network_id);
            additional_peer_network_ids.push(peer_network_id);
        }

        // Publish a message to the active subscribers
        let transaction_payload = BlockTransactionPayload::new_quorum_store_inline_hybrid(
            vec![],
            vec![],
            Some(10),
            None,
            vec![],
        );
        let block_payload_message = ConsensusObserverMessage::new_block_payload_message(
            BlockInfo::empty(),
            transaction_payload,
        );
        consensus_publisher.publish_message(block_payload_message.clone());

        // Verify that the message was sent to all active subscribers
        let num_expected_messages = additional_peer_network_ids.len() + 1;
        for _ in 0..num_expected_messages {
            let (peer_network_id, message) = outbound_message_receiver.next().await.unwrap();
            assert!(
                additional_peer_network_ids.contains(&peer_network_id)
                    || peer_network_id == peer_network_id_1
            );
            assert_eq!(message, block_payload_message);
        }

        // Unsubscribe the first peer from consensus updates
        process_unsubscription_for_peer(&consensus_publisher, &peer_network_id_1);

        // Publish another message to the active subscribers
        let commit_decision_message =
            ConsensusObserverMessage::new_commit_decision_message(LedgerInfoWithSignatures::new(
                LedgerInfo::new(BlockInfo::empty(), HashValue::zero()),
                AggregateSignature::empty(),
            ));
        consensus_publisher.publish_message(commit_decision_message.clone());

        // Verify that the message was sent to all active subscribers except the first peer
        for _ in 0..additional_peer_network_ids.len() {
            let (peer_network_id, message) = outbound_message_receiver.next().await.unwrap();
            assert!(additional_peer_network_ids.contains(&peer_network_id));
            assert_eq!(message, commit_decision_message);
        }

        // Unsubscribe the remaining peers from consensus updates
        for peer_network_id in additional_peer_network_ids {
            process_unsubscription_for_peer(&consensus_publisher, &peer_network_id);
        }

        // Publish another message to the active subscribers
        let block_payload_message = ConsensusObserverMessage::new_block_payload_message(
            BlockInfo::empty(),
            BlockTransactionPayload::empty(),
        );
        consensus_publisher.publish_message(block_payload_message.clone());

        // Verify that no messages were sent to the outbound message receiver
        assert!(outbound_message_receiver.next().now_or_never().is_none());
    }

    /// Processes a subscription request for the given peer
    fn process_subscription_for_peer(
        consensus_publisher: &ConsensusPublisher,
        peer_network_id: &PeerNetworkId,
    ) {
        // Create the subscribe message
        let network_message = ConsensusPublisherNetworkMessage::new(
            *peer_network_id,
            ConsensusObserverRequest::Subscribe,
            ResponseSender::new_for_test(),
        );

        // Process the subscription request
        consensus_publisher.process_network_message(network_message);
    }

    /// Processes an unsubscription request for the given peer
    fn process_unsubscription_for_peer(
        consensus_publisher: &ConsensusPublisher,
        peer_network_id: &PeerNetworkId,
    ) {
        // Create the unsubscribe message
        let network_message = ConsensusPublisherNetworkMessage::new(
            *peer_network_id,
            ConsensusObserverRequest::Unsubscribe,
            ResponseSender::new_for_test(),
        );

        // Process the unsubscription request
        consensus_publisher.process_network_message(network_message);
    }

    /// Verifies the active subscribers has the expected size and contains the expected peers
    fn verify_active_subscribers(
        consensus_publisher: &ConsensusPublisher,
        expected_size: usize,
        expected_peers: Vec<&PeerNetworkId>,
        unexpected_peers: Vec<&PeerNetworkId>,
    ) {
        // Verify the number of active subscribers
        let active_subscribers = consensus_publisher.get_active_subscribers();
        assert_eq!(active_subscribers.len(), expected_size);

        // Verify that the active subscribers contains the expected peers
        for peer_network_id in expected_peers {
            assert!(active_subscribers.contains(peer_network_id));
        }

        // Verify that the active subscribers does not contain the unexpected peers
        for peer_network_id in unexpected_peers {
            assert!(!active_subscribers.contains(peer_network_id));
        }
    }
}
