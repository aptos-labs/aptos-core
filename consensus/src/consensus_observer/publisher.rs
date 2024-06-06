// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::consensus_observer::{
    network_client::ConsensusObserverClient,
    network_events::ResponseSender,
    network_message::{
        ConsensusObserverDirectSend, ConsensusObserverMessage, ConsensusObserverRequest,
        ConsensusObserverResponse,
    },
};
use aptos_config::network_id::PeerNetworkId;
use aptos_infallible::RwLock;
use aptos_logger::{info, warn};
use aptos_network::application::interface::NetworkClient;
use std::{collections::HashSet, sync::Arc};

/// The consensus publisher sends consensus updates to downstream observers
#[derive(Clone)]
pub struct ConsensusPublisher {
    // The consensus observer client to send network messages
    consensus_observer_client:
        Arc<ConsensusObserverClient<NetworkClient<ConsensusObserverMessage>>>,

    // The set of active subscribers that have subscribed to consensus updates
    active_subscribers: Arc<RwLock<HashSet<PeerNetworkId>>>,
}

impl ConsensusPublisher {
    pub fn new(network_client: NetworkClient<ConsensusObserverMessage>) -> Self {
        Self {
            consensus_observer_client: Arc::new(ConsensusObserverClient::new(network_client)),
            active_subscribers: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Handles a subscription message from a peer
    pub fn handle_subscription_request(
        &self,
        peer_network_id: &PeerNetworkId,
        request: ConsensusObserverRequest,
        response_sender: ResponseSender,
    ) {
        match request {
            ConsensusObserverRequest::Subscribe => {
                // Add the peer to the set of active subscribers
                self.active_subscribers.write().insert(*peer_network_id);
                info!(
                    "New peer subscribed to consensus updates! Peer: {}",
                    peer_network_id
                );

                // Send a simple subscription ACK
                response_sender.send(ConsensusObserverResponse::SubscribeAck);
            },
            ConsensusObserverRequest::Unsubscribe => {
                // Remove the peer from the set of active subscribers
                self.active_subscribers.write().remove(peer_network_id);
                info!(
                    "Peer unsubscribed from consensus updates! Peer: {}",
                    peer_network_id
                );

                // Send a simple unsubscription ACK
                response_sender.send(ConsensusObserverResponse::UnsubscribeAck);
            },
        }
    }

    /// Publishes a direct send message to all active subscribers
    pub fn publish_message(&self, message: ConsensusObserverDirectSend) {
        // Get the set of active subscribers
        let active_subscribers = self.active_subscribers.read().clone();

        // Send the message to all active subscribers
        for peer_network_id in &active_subscribers {
            if let Err(error) = self
                .consensus_observer_client
                .send_message_to_peer(peer_network_id, message.clone())
            {
                // The message send failed. Log the error and remove the subscriber.
                warn!(
                    "Failed to send message to peer {}: {:?}",
                    peer_network_id, error
                );
                self.active_subscribers.write().remove(peer_network_id);
            }
        }
    }
}
