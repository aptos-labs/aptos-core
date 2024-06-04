// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::consensus_observer::network::ConsensusObserverMessage;
use aptos_config::network_id::PeerNetworkId;
use aptos_logger::info;
use aptos_network::application::interface::{NetworkClient, NetworkClientInterface};

/// Publish updates to downstream observers.
#[derive(Clone)]
pub struct Publisher {
    network_client: NetworkClient<ConsensusObserverMessage>,
}

impl Publisher {
    pub fn new(network_client: NetworkClient<ConsensusObserverMessage>) -> Self {
        Self { network_client }
    }

    fn get_downstream_peers(&self) -> Vec<PeerNetworkId> {
        if let Ok(peers) = self
            .network_client
            .get_peers_and_metadata()
            .get_connected_peers_and_metadata()
        {
            peers
                .into_iter()
                .filter(|(key, value)| {
                    !key.network_id().is_validator_network()
                        && !value.get_connection_metadata().is_outbound_connection()
                })
                .map(|(key, _)| key)
                .collect()
        } else {
            vec![]
        }
    }

    pub fn publish(&self, msg: ConsensusObserverMessage) {
        let downstream_peers = self.get_downstream_peers();
        for peer in &downstream_peers {
            info!("[Publisher] Sending {} to {}", msg, peer);
        }
        let _ = self.network_client.send_to_peers(msg, &downstream_peers);
    }
}
