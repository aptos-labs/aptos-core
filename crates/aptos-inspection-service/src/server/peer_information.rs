// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::server::utils::CONTENT_TYPE_TEXT;
use aptos_config::{config::NodeConfig, network_id::NetworkId};
use aptos_network::application::storage::PeersAndMetadata;
use hyper::{Body, StatusCode};
use std::sync::Arc;

// The message to display when the peer information endpoint is disabled
pub const PEER_INFO_DISABLED_MESSAGE: &str =
    "This endpoint is disabled! Enable it in the node config at inspection_service.expose_peer_information: true";

/// Handles a new peer information request
pub fn handle_peer_information_request(
    node_config: &NodeConfig,
    peers_and_metadata: Arc<PeersAndMetadata>,
) -> (StatusCode, Body, String) {
    // Only return peer information if the endpoint is enabled
    let (status_code, body) = if node_config.inspection_service.expose_peer_information {
        let peer_information = get_peer_information(peers_and_metadata);
        (StatusCode::OK, Body::from(peer_information))
    } else {
        (
            StatusCode::FORBIDDEN,
            Body::from(PEER_INFO_DISABLED_MESSAGE),
        )
    };

    (status_code, body, CONTENT_TYPE_TEXT.into())
}

/// Returns a simple text formatted string with peer and network information
fn get_peer_information(peers_and_metadata: Arc<PeersAndMetadata>) -> String {
    let mut peer_information = Vec::<String>::new();

    // Display a summary of all peers and networks
    let all_peers = peers_and_metadata.get_all_peers().unwrap_or_default();
    let registered_networks: Vec<NetworkId> =
        peers_and_metadata.get_registered_networks().collect();
    peer_information.push("Peer information summary:".into());
    peer_information.push(format!("\t- Number of peers: {}", all_peers.len()));
    peer_information.push(format!(
        "\t- Registered networks: {:?}",
        registered_networks
    ));
    peer_information.push(format!("\t- Peers and network IDs: {:?}", all_peers));
    peer_information.push("\n".into());

    // Display connection metadata for each peer
    peer_information.push("Connection metadata for each peer:".into());
    for peer in &all_peers {
        if let Ok(peer_metadata) = peers_and_metadata.get_metadata_for_peer(*peer) {
            let connection_metadata = peer_metadata.get_connection_metadata();
            peer_information.push(format!(
                "\t- Peer: {}, connection state: {:?}, connection metadata: {}",
                peer,
                peer_metadata.get_connection_state(),
                serde_json::to_string(&connection_metadata).unwrap_or_default()
            ));
        }
    }
    peer_information.push("\n".into());

    // Display the entire set of trusted peers
    peer_information.push("Trusted peers:".into());
    for network in registered_networks {
        peer_information.push(format!("\t- Network: {}", network));
        if let Ok(trusted_peers) = peers_and_metadata.get_trusted_peers(&network) {
            let trusted_peers = trusted_peers.read().clone();
            for trusted_peer in trusted_peers {
                peer_information.push(format!("\t\t- Peer: {:?}", trusted_peer));
            }
        }
    }
    peer_information.push("\n".into());

    // Display basic peer metadata for each peer
    peer_information.push("Basic monitoring metadata for each peer:".into());
    for peer in &all_peers {
        if let Ok(peer_metadata) = peers_and_metadata.get_metadata_for_peer(*peer) {
            let peer_monitoring_metadata = peer_metadata.get_peer_monitoring_metadata();
            peer_information.push(format!(
                "\t- Peer: {}, basic metadata: {}", // Display formatting for basic metadata
                peer, peer_monitoring_metadata
            ));
        }
    }
    peer_information.push("\n".into());

    // Display detailed peer metadata for each peer
    peer_information.push("Detailed monitoring metadata for each peer:".into());
    for peer in &all_peers {
        if let Ok(peer_metadata) = peers_and_metadata.get_metadata_for_peer(*peer) {
            let peer_monitoring_metadata = peer_metadata.get_peer_monitoring_metadata();
            peer_information.push(format!(
                "\t- Peer: {}, detailed metadata: {:?}", // Debug formatting for detailed metadata
                peer, peer_monitoring_metadata
            ));
        }
    }
    peer_information.push("\n".into());

    // Display the internal client state for each peer
    peer_information.push("Internal client state for each peer:".into());
    for peer in &all_peers {
        if let Ok(peer_metadata) = peers_and_metadata.get_metadata_for_peer(*peer) {
            let peer_monitoring_metadata = peer_metadata.get_peer_monitoring_metadata();
            peer_information.push(format!(
                "\t- Peer: {}, internal client state: {:?}",
                peer, peer_monitoring_metadata.internal_client_state
            ));
        }
    }

    peer_information.join("\n") // Separate each entry with a newline
}
