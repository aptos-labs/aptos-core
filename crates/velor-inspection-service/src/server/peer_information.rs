// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::server::utils::CONTENT_TYPE_TEXT;
use velor_config::{
    config::NodeConfig,
    network_id::{NetworkId, PeerNetworkId},
};
use velor_data_client::{
    client::VelorDataClient, interface::VelorDataClientInterface, peer_states,
};
use velor_network::application::storage::PeersAndMetadata;
use hyper::{Body, StatusCode};
use std::{collections::BTreeMap, ops::Deref, sync::Arc};

// The message to display when the peer information endpoint is disabled
pub const PEER_INFO_DISABLED_MESSAGE: &str =
    "This endpoint is disabled! Enable it in the node config at inspection_service.expose_peer_information: true";

/// Handles a new peer information request
pub fn handle_peer_information_request(
    node_config: &NodeConfig,
    velor_data_client: VelorDataClient,
    peers_and_metadata: Arc<PeersAndMetadata>,
) -> (StatusCode, Body, String) {
    // Only return peer information if the endpoint is enabled
    let (status_code, body) = if node_config.inspection_service.expose_peer_information {
        let peer_information = get_peer_information(velor_data_client, peers_and_metadata);
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
fn get_peer_information(
    velor_data_client: VelorDataClient,
    peers_and_metadata: Arc<PeersAndMetadata>,
) -> String {
    // Get all registered networks
    let registered_networks: Vec<NetworkId> =
        peers_and_metadata.get_registered_networks().collect();

    // Get all peers (sorted by peer ID)
    let mut all_peers = peers_and_metadata.get_all_peers();
    all_peers.sort();

    // Display a summary of all peers and networks
    let mut peer_information_output = Vec::<String>::new();
    display_peer_information_summary(
        &mut peer_information_output,
        &all_peers,
        &registered_networks,
    );
    peer_information_output.push("\n".into());

    // Display connection metadata for each peer
    display_peer_connection_metadata(
        &mut peer_information_output,
        &all_peers,
        peers_and_metadata.deref(),
    );
    peer_information_output.push("\n".into());

    // Display the entire set of trusted peers
    display_trusted_peers(
        &mut peer_information_output,
        registered_networks,
        peers_and_metadata.deref(),
    );
    peer_information_output.push("\n".into());

    // Display basic peer metadata for each peer
    display_peer_monitoring_metadata(
        &mut peer_information_output,
        &all_peers,
        peers_and_metadata.deref(),
    );
    peer_information_output.push("\n".into());

    // Display state sync metadata for each peer
    display_state_sync_metadata(&mut peer_information_output, &all_peers, velor_data_client);
    peer_information_output.push("\n".into());

    // Display detailed peer metadata for each peer
    display_detailed_monitoring_metadata(
        &mut peer_information_output,
        &all_peers,
        peers_and_metadata.deref(),
    );
    peer_information_output.push("\n".into());

    // Display the internal client state for each peer
    display_internal_client_state(
        &mut peer_information_output,
        &all_peers,
        peers_and_metadata.deref(),
    );

    peer_information_output.join("\n") // Separate each entry with a newline to construct the output
}

/// Displays detailed peer monitoring metadata for each peer
fn display_detailed_monitoring_metadata(
    peer_information_output: &mut Vec<String>,
    all_peers: &Vec<PeerNetworkId>,
    peers_and_metadata: &PeersAndMetadata,
) {
    peer_information_output.push("Detailed monitoring metadata for each peer:".into());

    // Fetch and display the detailed metadata for each peer
    for peer in all_peers {
        if let Ok(peer_metadata) = peers_and_metadata.get_metadata_for_peer(*peer) {
            let peer_monitoring_metadata = peer_metadata.get_peer_monitoring_metadata();
            peer_information_output.push(format!(
                "\t- Peer: {}, detailed metadata: {:?}", // Debug formatting for detailed metadata
                peer, peer_monitoring_metadata
            ));
        }
    }
}

/// Displays the internal client state for each peer
fn display_internal_client_state(
    peer_information_output: &mut Vec<String>,
    all_peers: &Vec<PeerNetworkId>,
    peers_and_metadata: &PeersAndMetadata,
) {
    peer_information_output.push("Internal client state for each peer:".into());

    // Fetch and display the internal client state for each peer
    for peer in all_peers {
        if let Ok(peer_metadata) = peers_and_metadata.get_metadata_for_peer(*peer) {
            let peer_monitoring_metadata = peer_metadata.get_peer_monitoring_metadata();
            peer_information_output.push(format!(
                "\t- Peer: {}, internal client state: {:?}",
                peer, peer_monitoring_metadata.internal_client_state
            ));
        }
    }
}

/// Displays connection metadata for each peer
fn display_peer_connection_metadata(
    peer_information_output: &mut Vec<String>,
    all_peers: &Vec<PeerNetworkId>,
    peers_and_metadata: &PeersAndMetadata,
) {
    peer_information_output.push("Connection metadata for each peer:".into());

    // Fetch and display the connection metadata for each peer
    for peer in all_peers {
        if let Ok(peer_metadata) = peers_and_metadata.get_metadata_for_peer(*peer) {
            let connection_metadata = peer_metadata.get_connection_metadata();
            peer_information_output.push(format!(
                "\t- Peer: {}, connection state: {:?}, connection metadata: {}",
                peer,
                peer_metadata.get_connection_state(),
                serde_json::to_string(&connection_metadata).unwrap_or_default()
            ));
        }
    }
}

/// Displays a summary of all peers and registered networks
fn display_peer_information_summary(
    peer_information_output: &mut Vec<String>,
    all_peers: &Vec<PeerNetworkId>,
    registered_networks: &Vec<NetworkId>,
) {
    peer_information_output.push("Peer information summary:".into());
    peer_information_output.push(format!("\t- Number of peers: {}", all_peers.len()));
    peer_information_output.push(format!(
        "\t- Registered networks: {:?}",
        registered_networks
    ));
    peer_information_output.push(format!("\t- Peers and network IDs: {:?}", all_peers));
}

/// Displays peer monitoring metadata for each peer
fn display_peer_monitoring_metadata(
    peer_information_output: &mut Vec<String>,
    all_peers: &Vec<PeerNetworkId>,
    peers_and_metadata: &PeersAndMetadata,
) {
    peer_information_output.push("Basic monitoring metadata for each peer:".into());

    // Fetch and display the basic metadata for each peer
    for peer in all_peers {
        if let Ok(peer_metadata) = peers_and_metadata.get_metadata_for_peer(*peer) {
            let peer_monitoring_metadata = peer_metadata.get_peer_monitoring_metadata();
            peer_information_output.push(format!(
                "\t- Peer: {}, basic metadata: {}", // Display formatting for basic metadata
                peer, peer_monitoring_metadata
            ));
        }
    }
}

/// Displays state sync metadata for each peer
fn display_state_sync_metadata(
    peer_information_output: &mut Vec<String>,
    all_peers: &Vec<PeerNetworkId>,
    velor_data_client: VelorDataClient,
) {
    peer_information_output.push("State sync metadata for each peer:".into());

    // Fetch and display the priority and regular peers
    if let Ok((priority_peers, regular_peers)) = velor_data_client.get_priority_and_regular_peers()
    {
        // Sort the peer lists before displaying them
        let mut priority_peers: Vec<_> = priority_peers.into_iter().collect();
        priority_peers.sort();
        let mut regular_peers: Vec<_> = regular_peers.into_iter().collect();
        regular_peers.sort();

        // Display the priority and regular peers
        peer_information_output.push(format!(
            "\t- Priority peers: {:?}, regular peers: {:?}",
            priority_peers, regular_peers
        ));
    }

    // Fetch and display the global advertised data summary
    let global_data_summary = velor_data_client.get_global_data_summary();
    peer_information_output.push(format!(
        "\t- Global advertised data summary: {:?}",
        global_data_summary
    ));

    // Fetch and display the state sync metadata for each peer
    let peer_to_state = velor_data_client.get_peer_states().get_peer_to_states();
    for peer in all_peers {
        if let Some(peer_state_entry) = peer_to_state.get(peer) {
            // Get the peer states
            let peer = *peer_state_entry.key();
            let peer_bucket_id = peer_states::get_bucket_id_for_peer(peer);
            let peer_score = peer_state_entry.get_score();
            let peer_storage_summary = peer_state_entry.get_storage_summary();

            // Display the peer states
            peer_information_output.push(format!(
                "\t- Peer: {}, score: {}, bucket ID: {}",
                peer, peer_score, peer_bucket_id
            ));
            peer_information_output.push(format!(
                "\t\t- Advertised storage summary: {:?}",
                peer_storage_summary
            ));

            // Get the peer's request/response counts
            let sent_requests_by_type = peer_state_entry.get_sent_requests_by_type();
            let received_responses_by_type = peer_state_entry.get_received_responses_by_type();

            // Display the peer's request/response counts
            peer_information_output.push(format!(
                "\t\t- Sent requests by type: {:?}",
                sent_requests_by_type
            ));
            peer_information_output.push(format!(
                "\t\t- Received responses by type: {:?}",
                received_responses_by_type
            ));
        }
    }
}

/// Displays the entire set of trusted peers
fn display_trusted_peers(
    peer_information_output: &mut Vec<String>,
    registered_networks: Vec<NetworkId>,
    peers_and_metadata: &PeersAndMetadata,
) {
    peer_information_output.push("Trusted peers (validator set & seeds):".into());

    // Fetch and display the trusted peers for each network
    for network in registered_networks {
        peer_information_output.push(format!("\t- Network: {}", network));
        if let Ok(trusted_peers) = peers_and_metadata.get_trusted_peers(&network) {
            // Sort the peers before displaying them
            let mut sorted_trusted_peers = BTreeMap::new();
            for (peer_id, peer_info) in trusted_peers {
                sorted_trusted_peers.insert(peer_id, peer_info);
            }

            // Display the trusted peers
            for (peer_id, peer_info) in sorted_trusted_peers {
                peer_information_output.push(format!(
                    "\t\t- Peer: {:?}, peer information: {:?}",
                    peer_id, peer_info
                ));
            }
        }
    }
}
