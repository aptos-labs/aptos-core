// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::consensus_observer::{
    logging::{LogEntry, LogEvent, LogSchema},
    network_client::ConsensusObserverClient,
    network_message::{ConsensusObserverDirectSend, ConsensusObserverMessage},
};
use aptos_config::network_id::PeerNetworkId;
use aptos_logger::error;
use aptos_network::application::interface::NetworkClient;

/// The consensus publisher sends consensus updates to downstream observers
#[derive(Clone)]
pub struct ConsensusPublisher {
    consensus_observer_client: ConsensusObserverClient<NetworkClient<ConsensusObserverMessage>>,
}

impl ConsensusPublisher {
    pub fn new(
        consensus_observer_client: ConsensusObserverClient<NetworkClient<ConsensusObserverMessage>>,
    ) -> Self {
        Self {
            consensus_observer_client,
        }
    }

    /// Identifies all downstream peers to publish consensus updates
    fn get_downstream_peers(&self) -> Vec<PeerNetworkId> {
        // TODO: we should cache this (to avoid expensive calls to the network client) and use subscriptions

        // Get the connected peers and metadata
        let peers_and_metadata = self.consensus_observer_client.get_peers_and_metadata();
        let connected_peers_and_metadata = peers_and_metadata.get_connected_peers_and_metadata();

        // Identify the downstream peers. This is currently a heuristic for VFNs.
        match connected_peers_and_metadata {
            Ok(peers_and_metadata) => peers_and_metadata
                .into_iter()
                .filter(|(peer_network_id, peer_metadata)| {
                    // Ensure the peer is not a validator and that it dialed us
                    !peer_network_id.network_id().is_validator_network()
                        && !peer_metadata
                            .get_connection_metadata()
                            .is_outbound_connection()
                })
                .map(|(peer_network_id, _)| peer_network_id)
                .collect(),
            Err(error) => {
                // Log the error and return an empty set of peers
                error!(LogSchema::new(LogEntry::GetDownstreamPeers)
                    .event(LogEvent::UnexpectedError)
                    .error(&error.into()));
                vec![]
            },
        }
    }

    /// Publishes a direct send message to all downstream peers
    pub fn publish_message(&self, message: ConsensusObserverDirectSend) {
        let downstream_peers = self.get_downstream_peers();
        self.consensus_observer_client
            .send_message_to_peers(downstream_peers, message);
    }
}
