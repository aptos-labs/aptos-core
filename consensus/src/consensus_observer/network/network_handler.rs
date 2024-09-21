// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::consensus_observer::{
    common::logging::{LogEntry, LogSchema},
    network::{
        network_events::{ConsensusObserverNetworkEvents, NetworkMessage, ResponseSender},
        observer_message::{
            ConsensusObserverDirectSend, ConsensusObserverMessage, ConsensusObserverRequest,
        },
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
                            warn!(
                                LogSchema::new(LogEntry::ConsensusObserver).message(&format!(
                                    "Received unexpected response from peer: {}",
                                    peer_network_id
                                ))
                            );
                        },
                    }
                }
                else => {
                    break; // Exit the network handler loop
                }
            }
        }

        // Log an error that the network handler has stopped
        error!(LogSchema::new(LogEntry::ConsensusObserver)
            .message("Consensus observer network handler has stopped!"));
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
                        "Missing response sender for the RPC request: {:?}",
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

#[cfg(test)]
mod test {
    use crate::consensus_observer::network::{
        network_events::ConsensusObserverNetworkEvents,
        network_handler::{
            ConsensusObserverNetworkHandler, ConsensusObserverNetworkMessage,
            ConsensusPublisherNetworkMessage,
        },
        observer_client::ConsensusObserverClient,
        observer_message::{
            ConsensusObserverDirectSend, ConsensusObserverMessage, ConsensusObserverRequest,
        },
    };
    use aptos_channels::{aptos_channel, aptos_channel::Receiver, message_queues::QueueStyle};
    use aptos_config::{
        config::ConsensusObserverConfig,
        network_id::{NetworkId, PeerNetworkId},
    };
    use aptos_crypto::HashValue;
    use aptos_network::{
        application::{
            interface::{NetworkClient, NetworkServiceEvents},
            storage::PeersAndMetadata,
        },
        peer_manager::{ConnectionRequestSender, PeerManagerRequest, PeerManagerRequestSender},
        protocols::{
            network::{
                NetworkEvents, NetworkSender, NewNetworkEvents, NewNetworkSender, ReceivedMessage,
            },
            wire::{
                handshake::v1::{ProtocolId, ProtocolIdSet},
                messaging::v1::{DirectSendMsg, NetworkMessage, RpcRequest},
            },
        },
        transport::ConnectionMetadata,
    };
    use aptos_types::{
        aggregate_signature::AggregateSignature,
        block_info::BlockInfo,
        ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
        PeerId,
    };
    use futures::{FutureExt, StreamExt};
    use std::{collections::HashMap, hash::Hash, sync::Arc, time::Duration};
    use tokio::time::timeout;

    // Useful test constants for timeouts
    const MAX_CHANNEL_TIMEOUT_SECS: u64 = 5;
    const MAX_MESSAGE_WAIT_TIME_SECS: u64 = 5;
    const RPC_REQUEST_TIMEOUT_MS: u64 = 10_000;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_handle_observer_message() {
        // Create a consensus observer config with the observer enabled
        let consensus_observer_config = ConsensusObserverConfig {
            observer_enabled: true,
            ..Default::default()
        };

        // Create the peers and metadata container
        let network_ids = vec![NetworkId::Vfn, NetworkId::Public];
        let peers_and_metadata = PeersAndMetadata::new(&network_ids);

        // Create a single peer and initialize the connection metadata
        let peer_network_id =
            create_peer_and_connection(NetworkId::Public, peers_and_metadata.clone());

        // Create the consensus observer client
        let (
            network_senders,
            network_events,
            mut outbound_request_receivers,
            mut inbound_request_senders,
        ) = create_network_sender_and_events(&network_ids);
        let consensus_observer_client =
            create_observer_network_client(peers_and_metadata, network_senders);

        // Create the consensus observer network events
        let observer_network_events = ConsensusObserverNetworkEvents::new(network_events);

        // Create the consensus observer network handler
        let (network_handler, mut observer_message_receiver, mut publisher_message_receiver) =
            ConsensusObserverNetworkHandler::new(
                consensus_observer_config,
                observer_network_events,
            );

        // Start the consensus observer network handler
        tokio::spawn(network_handler.start());

        // Create a consensus observer message
        let consensus_observer_message = ConsensusObserverMessage::new_ordered_block_message(
            vec![],
            LedgerInfoWithSignatures::new(
                LedgerInfo::new(BlockInfo::empty(), HashValue::zero()),
                AggregateSignature::empty(),
            ),
        );

        // Send the message to the network handler
        send_observer_message(
            &peer_network_id,
            consensus_observer_client,
            &consensus_observer_message,
        );

        // Wait for the message to be processed by the outbound handler
        wait_for_outbound_processing(
            peer_network_id,
            &mut outbound_request_receivers,
            &mut inbound_request_senders,
            Some(ProtocolId::ConsensusObserver),
            None,
            false,
        )
        .await;

        // Wait for the handler to process and forward the observer message
        wait_for_handler_processing(
            peer_network_id,
            &mut observer_message_receiver,
            &mut publisher_message_receiver,
            Some(consensus_observer_message),
            None,
        )
        .await;

        // Verify no further message is received
        wait_and_verify_no_message(&mut observer_message_receiver).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_handle_observer_message_disabled() {
        // Create a consensus observer config with the observer disabled
        let consensus_observer_config = ConsensusObserverConfig {
            observer_enabled: false,
            ..Default::default()
        };

        // Create the peers and metadata container
        let network_ids = vec![NetworkId::Vfn, NetworkId::Public];
        let peers_and_metadata = PeersAndMetadata::new(&network_ids);

        // Create a single peer and initialize the connection metadata
        let peer_network_id =
            create_peer_and_connection(NetworkId::Public, peers_and_metadata.clone());

        // Create the consensus observer client
        let (
            network_senders,
            network_events,
            mut outbound_request_receivers,
            mut inbound_request_senders,
        ) = create_network_sender_and_events(&network_ids);
        let consensus_observer_client =
            create_observer_network_client(peers_and_metadata, network_senders);

        // Create the consensus observer network events
        let observer_network_events = ConsensusObserverNetworkEvents::new(network_events);

        // Create the consensus observer network handler
        let (network_handler, mut observer_message_receiver, _) =
            ConsensusObserverNetworkHandler::new(
                consensus_observer_config,
                observer_network_events,
            );

        // Start the consensus observer network handler
        tokio::spawn(network_handler.start());

        // Create a consensus observer message
        let consensus_observer_message = ConsensusObserverMessage::new_ordered_block_message(
            vec![],
            LedgerInfoWithSignatures::new(
                LedgerInfo::new(BlockInfo::empty(), HashValue::zero()),
                AggregateSignature::empty(),
            ),
        );

        // Send the message to the network handler
        send_observer_message(
            &peer_network_id,
            consensus_observer_client,
            &consensus_observer_message,
        );

        // Wait for the message to be processed by the outbound handler
        wait_for_outbound_processing(
            peer_network_id,
            &mut outbound_request_receivers,
            &mut inbound_request_senders,
            Some(ProtocolId::ConsensusObserver),
            None,
            false,
        )
        .await;

        // Verify no message is received
        wait_and_verify_no_message(&mut observer_message_receiver).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_handle_publisher_message() {
        // Create a consensus observer config with the publisher enabled
        let consensus_observer_config = ConsensusObserverConfig {
            publisher_enabled: true,
            ..Default::default()
        };

        // Create the peers and metadata container
        let network_ids = vec![NetworkId::Vfn, NetworkId::Public];
        let peers_and_metadata = PeersAndMetadata::new(&network_ids);

        // Create a single peer and initialize the connection metadata
        let peer_network_id =
            create_peer_and_connection(NetworkId::Vfn, peers_and_metadata.clone());

        // Create the consensus observer client
        let (
            network_senders,
            network_events,
            mut outbound_request_receivers,
            mut inbound_request_senders,
        ) = create_network_sender_and_events(&network_ids);
        let consensus_observer_client =
            create_observer_network_client(peers_and_metadata, network_senders);

        // Create the consensus observer network events
        let observer_network_events = ConsensusObserverNetworkEvents::new(network_events);

        // Create the consensus observer network handler
        let (network_handler, mut observer_message_receiver, mut publisher_message_receiver) =
            ConsensusObserverNetworkHandler::new(
                consensus_observer_config,
                observer_network_events,
            );

        // Start the consensus observer network handler
        tokio::spawn(network_handler.start());

        // Create a consensus publisher message
        let consensus_publisher_message = ConsensusObserverRequest::Subscribe;

        // Send the message to the network handler
        send_publisher_message(
            peer_network_id,
            consensus_observer_client,
            consensus_publisher_message.clone(),
        );

        // Wait for the message to be processed by the outbound handler
        wait_for_outbound_processing(
            peer_network_id,
            &mut outbound_request_receivers,
            &mut inbound_request_senders,
            None,
            Some(ProtocolId::ConsensusObserverRpc),
            true,
        )
        .await;

        // Wait for the handler to process and forward the publisher message
        wait_for_handler_processing(
            peer_network_id,
            &mut observer_message_receiver,
            &mut publisher_message_receiver,
            None,
            Some(consensus_publisher_message),
        )
        .await;

        // Verify no further message is received
        wait_and_verify_no_message(&mut publisher_message_receiver).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_handle_publisher_message_disabled() {
        // Create a consensus observer config with the publisher disabled
        let consensus_observer_config = ConsensusObserverConfig {
            publisher_enabled: false,
            ..Default::default()
        };

        // Create the peers and metadata container
        let network_ids = vec![NetworkId::Vfn, NetworkId::Public];
        let peers_and_metadata = PeersAndMetadata::new(&network_ids);

        // Create a single peer and initialize the connection metadata
        let peer_network_id =
            create_peer_and_connection(NetworkId::Public, peers_and_metadata.clone());

        // Create the consensus observer client
        let (
            network_senders,
            network_events,
            mut outbound_request_receivers,
            mut inbound_request_senders,
        ) = create_network_sender_and_events(&network_ids);
        let consensus_observer_client =
            create_observer_network_client(peers_and_metadata, network_senders);

        // Create the consensus observer network events
        let observer_network_events = ConsensusObserverNetworkEvents::new(network_events);

        // Create the consensus observer network handler
        let (network_handler, _, mut publisher_message_receiver) =
            ConsensusObserverNetworkHandler::new(
                consensus_observer_config,
                observer_network_events,
            );

        // Start the consensus observer network handler
        tokio::spawn(network_handler.start());

        // Create a consensus publisher message
        let consensus_publisher_message = ConsensusObserverRequest::Subscribe;

        // Send the message to the network handler
        send_publisher_message(
            peer_network_id,
            consensus_observer_client,
            consensus_publisher_message.clone(),
        );

        // Wait for the message to be processed by the outbound handler
        wait_for_outbound_processing(
            peer_network_id,
            &mut outbound_request_receivers,
            &mut inbound_request_senders,
            None,
            Some(ProtocolId::ConsensusObserverRpc),
            true,
        )
        .await;

        // Verify no message is received
        wait_and_verify_no_message(&mut publisher_message_receiver).await;
    }

    /// Creates and returns a single Aptos channel
    fn create_aptos_channel<K: Eq + Hash + Clone, T>(
    ) -> (aptos_channel::Sender<K, T>, aptos_channel::Receiver<K, T>) {
        aptos_channel::new(QueueStyle::FIFO, 10, None)
    }

    /// Creates a network sender and events for testing (using the specified network IDs)
    fn create_network_sender_and_events(
        network_ids: &[NetworkId],
    ) -> (
        HashMap<NetworkId, NetworkSender<ConsensusObserverMessage>>,
        NetworkServiceEvents<ConsensusObserverMessage>,
        HashMap<NetworkId, aptos_channel::Receiver<(PeerId, ProtocolId), PeerManagerRequest>>,
        HashMap<NetworkId, aptos_channel::Sender<(PeerId, ProtocolId), ReceivedMessage>>,
    ) {
        let mut network_senders = HashMap::new();
        let mut network_and_events = HashMap::new();
        let mut outbound_request_receivers = HashMap::new();
        let mut inbound_request_senders = HashMap::new();

        for network_id in network_ids {
            // Create the peer manager and connection channels
            let (inbound_request_sender, inbound_request_receiver) = create_aptos_channel();
            let (outbound_request_sender, outbound_request_receiver) = create_aptos_channel();
            let (connection_outbound_sender, _connection_outbound_receiver) =
                create_aptos_channel();

            // Create the network sender and events
            let network_sender = NetworkSender::new(
                PeerManagerRequestSender::new(outbound_request_sender),
                ConnectionRequestSender::new(connection_outbound_sender),
            );
            let network_events = NetworkEvents::new(inbound_request_receiver, None, true);

            // Save the sender, events and receivers
            network_senders.insert(*network_id, network_sender);
            network_and_events.insert(*network_id, network_events);
            outbound_request_receivers.insert(*network_id, outbound_request_receiver);
            inbound_request_senders.insert(*network_id, inbound_request_sender);
        }

        // Create the network service events
        let network_service_events = NetworkServiceEvents::new(network_and_events);

        (
            network_senders,
            network_service_events,
            outbound_request_receivers,
            inbound_request_senders,
        )
    }

    /// Creates and returns a consensus observer network client
    fn create_observer_network_client(
        peers_and_metadata: Arc<PeersAndMetadata>,
        network_senders: HashMap<NetworkId, NetworkSender<ConsensusObserverMessage>>,
    ) -> ConsensusObserverClient<NetworkClient<ConsensusObserverMessage>> {
        let network_client: NetworkClient<ConsensusObserverMessage> = NetworkClient::new(
            vec![ProtocolId::ConsensusObserver],
            vec![ProtocolId::ConsensusObserverRpc],
            network_senders,
            peers_and_metadata.clone(),
        );
        ConsensusObserverClient::new(network_client)
    }

    /// Creates a new peer with the specified connection metadata
    fn create_peer_and_connection(
        network_id: NetworkId,
        peers_and_metadata: Arc<PeersAndMetadata>,
    ) -> PeerNetworkId {
        // Create the peer and connection metadata
        let peer_network_id = PeerNetworkId::new(network_id, PeerId::random());
        let mut connection_metadata = ConnectionMetadata::mock(peer_network_id.peer_id());

        // Update the application protocols
        let protocol_ids = vec![
            ProtocolId::ConsensusObserver,
            ProtocolId::ConsensusObserverRpc,
        ];
        connection_metadata.application_protocols = ProtocolIdSet::from_iter(protocol_ids);

        // Insert the connection into peers and metadata
        peers_and_metadata
            .insert_connection_metadata(peer_network_id, connection_metadata.clone())
            .unwrap();

        peer_network_id
    }

    /// Sends a consensus observer message to the network handler
    fn send_observer_message(
        peer_network_id: &PeerNetworkId,
        consensus_observer_client: ConsensusObserverClient<NetworkClient<ConsensusObserverMessage>>,
        consensus_observer_message: &ConsensusObserverDirectSend,
    ) {
        // Serialize the message
        let serialized_message = consensus_observer_client
            .serialize_message_for_peer(peer_network_id, consensus_observer_message.clone())
            .unwrap();

        // Send the message via the observer client
        consensus_observer_client
            .send_serialized_message_to_peer(peer_network_id, serialized_message, "")
            .unwrap();
    }

    /// Sends a consensus publisher message to the network handler
    fn send_publisher_message(
        peer_network_id: PeerNetworkId,
        consensus_observer_client: ConsensusObserverClient<NetworkClient<ConsensusObserverMessage>>,
        consensus_publisher_message: ConsensusObserverRequest,
    ) {
        tokio::spawn(async move {
            consensus_observer_client
                .send_rpc_request_to_peer(
                    &peer_network_id,
                    consensus_publisher_message,
                    RPC_REQUEST_TIMEOUT_MS,
                )
                .await
                .unwrap()
        });
    }

    /// Waits for a while to ensure that the handler has processed any
    /// messages it receives and verifies that no message is received.
    async fn wait_and_verify_no_message<Message>(message_receiver: &mut Receiver<(), Message>) {
        // Wait for a while to ensure the handler has processed any message
        tokio::time::sleep(Duration::from_secs(MAX_MESSAGE_WAIT_TIME_SECS)).await;

        // Verify that no message is received
        assert!(message_receiver.select_next_some().now_or_never().is_none());
    }

    /// Waits for the network handler to process a message and forward
    /// it to the appropriate receiver (observer or publisher).
    async fn wait_for_handler_processing(
        expected_peer_network_id: PeerNetworkId,
        observer_message_receiver: &mut aptos_channel::Receiver<
            (),
            ConsensusObserverNetworkMessage,
        >,
        publisher_message_receiver: &mut aptos_channel::Receiver<
            (),
            ConsensusPublisherNetworkMessage,
        >,
        expected_observer_message: Option<ConsensusObserverDirectSend>,
        expected_publisher_message: Option<ConsensusObserverRequest>,
    ) {
        // If we expect an observer message, wait for it and verify the contents
        if let Some(expected_observer_message) = expected_observer_message {
            match timeout(Duration::from_secs(MAX_CHANNEL_TIMEOUT_SECS), observer_message_receiver.select_next_some()).await {
                Ok(observer_network_message) => {
                    let (peer_network_id, observer_message) = observer_network_message.into_parts();
                    assert_eq!(peer_network_id, expected_peer_network_id);
                    assert_eq!(observer_message, expected_observer_message);
                },
                Err(elapsed) => panic!(
                    "Timed out while waiting to receive a consensus observer message. Elapsed: {:?}",
                    elapsed
                ),
            }
        }

        // If we expect a publisher message, wait for it and verify the contents
        if let Some(expected_publisher_message) = expected_publisher_message {
            match timeout(Duration::from_secs(MAX_CHANNEL_TIMEOUT_SECS), publisher_message_receiver.select_next_some()).await {
                Ok(publisher_network_message) => {
                    let (peer_network_id, publisher_message, _) = publisher_network_message.into_parts();
                    assert_eq!(peer_network_id, expected_peer_network_id);
                    assert_eq!(publisher_message, expected_publisher_message);
                },
                Err(elapsed) => panic!(
                    "Timed out while waiting to receive a consensus publisher message. Elapsed: {:?}",
                    elapsed
                ),
            }
        }
    }

    /// Waits for an outbound message and passes it to the inbound
    /// request senders (to emulate network wire transfer).
    async fn wait_for_outbound_processing(
        expected_peer_network_id: PeerNetworkId,
        outbound_request_receivers: &mut HashMap<
            NetworkId,
            aptos_channel::Receiver<(PeerId, ProtocolId), PeerManagerRequest>,
        >,
        inbound_request_senders: &mut HashMap<
            NetworkId,
            aptos_channel::Sender<(PeerId, ProtocolId), ReceivedMessage>,
        >,
        expected_direct_send_protocol: Option<ProtocolId>,
        expected_rpc_protocol: Option<ProtocolId>,
        is_rpc_request: bool,
    ) {
        // Extract the peer and network ID
        let expected_peer_id = expected_peer_network_id.peer_id();
        let expected_network_id = expected_peer_network_id.network_id();

        // Verify the message is received on the outbound request
        // receivers and forward it to the inbound request senders.
        let outbound_request_receiver = outbound_request_receivers
            .get_mut(&expected_network_id)
            .unwrap();
        match timeout(Duration::from_secs(MAX_CHANNEL_TIMEOUT_SECS), outbound_request_receiver.select_next_some()).await {
            Ok(peer_manager_request) => {
                let (protocol_id, peer_manager_notification) = match peer_manager_request {
                    PeerManagerRequest::SendRpc(peer_id, outbound_rpc_request) => {
                        // Verify the message is correct
                        assert!(is_rpc_request);
                        assert_eq!(peer_id, expected_peer_id);
                        assert_eq!(Some(outbound_rpc_request.protocol_id), expected_rpc_protocol);
                        assert_eq!(outbound_rpc_request.timeout, Duration::from_millis(RPC_REQUEST_TIMEOUT_MS));

                        // Create and return the received message
                        let received_message = ReceivedMessage {
                            message: NetworkMessage::RpcRequest(RpcRequest{
                                protocol_id: outbound_rpc_request.protocol_id,
                                request_id: 0,
                                priority: 0,
                                raw_request: outbound_rpc_request.data.into(),
                            }),
                            sender: PeerNetworkId::new(expected_network_id, peer_id),
                            receive_timestamp_micros: 0,
                            rpc_replier: Some(Arc::new(outbound_rpc_request.res_tx)),
                        };
                        (outbound_rpc_request.protocol_id, received_message)
                    }
                    PeerManagerRequest::SendDirectSend(peer_id, message) => {
                        // Verify the message is correct
                        assert!(!is_rpc_request);
                        assert_eq!(peer_id, expected_peer_id);
                        assert_eq!(Some(message.protocol_id), expected_direct_send_protocol);

                        // Create and return the received message
                        let received_message = ReceivedMessage {
                            message: NetworkMessage::DirectSendMsg(DirectSendMsg{
                                protocol_id: message.protocol_id,
                                priority: 0,
                                raw_msg: message.mdata.into(),
                            }),
                            sender: PeerNetworkId::new(expected_network_id, peer_id),
                            receive_timestamp_micros: 0,
                            rpc_replier: None,
                        };
                        (message.protocol_id, received_message)
                    }
                };

                // Pass the message from the outbound request receivers to the
                // inbound request senders. This emulates network wire transfer.
                let inbound_request_sender = inbound_request_senders.get_mut(&expected_network_id).unwrap();
                inbound_request_sender.push((expected_peer_id, protocol_id), peer_manager_notification).unwrap();
            }
            Err(elapsed) => panic!(
                "Timed out while waiting to receive a message on the outbound receivers channel. Elapsed: {:?}",
                elapsed
            ),
        }
    }
}
