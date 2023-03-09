// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{metrics, Error, PeerMonitorState, PeerMonitoringServiceClient, PeerState};
use aptos_config::network_id::PeerNetworkId;
use aptos_network::application::{interface::NetworkClient, metadata::PeerMetadata};
use aptos_peer_monitoring_service_types::PeerMonitoringServiceMessage;
use aptos_time_service::TimeService;
use key_value::PeerStateKey;
use std::collections::HashMap;
use tokio::runtime::Handle;

mod key_value;
mod latency_info;
mod network_info;
pub mod peer_state;
mod request_tracker;

/// Refreshes the states of the connected peers
pub fn refresh_peer_states(
    peer_monitor_state: PeerMonitorState,
    peer_monitoring_client: PeerMonitoringServiceClient<
        NetworkClient<PeerMonitoringServiceMessage>,
    >,
    connected_peers_and_metadata: HashMap<PeerNetworkId, PeerMetadata>,
    time_service: TimeService,
    runtime: Option<Handle>,
) -> Result<(), Error> {
    // Process all state entries (in order) and update the ones that
    // need to be refreshed for each peer.
    for peer_state_key in PeerStateKey::get_all_keys() {
        let mut num_in_flight_requests = 0;

        // Go through all connected peers and see if we should refresh the state
        for (peer_network_id, peer_metadata) in &connected_peers_and_metadata {
            // Get the peer state
            let peer_state = get_peer_state(&peer_monitor_state, peer_network_id)?;

            // If there's an-flight request, update the metrics counter
            let request_tracker = peer_state.get_request_tracker(&peer_state_key)?;
            if request_tracker.read().in_flight_request() {
                num_in_flight_requests += 1;
            }

            // Update the state if it needs to be refreshed
            let should_refresh_peer_state_key = request_tracker.read().new_request_required();
            if should_refresh_peer_state_key {
                peer_state.refresh_peer_state_key(
                    &peer_state_key,
                    peer_monitoring_client.clone(),
                    *peer_network_id,
                    peer_metadata.clone(),
                    peer_monitor_state.request_id_generator.clone(),
                    time_service.clone(),
                    runtime.clone(),
                )?;
            }
        }

        // Update the in-flight request metrics
        update_in_flight_metrics(peer_state_key, num_in_flight_requests);
    }

    Ok(())
}

/// Returns the peer state for the given peer
fn get_peer_state(
    peer_monitor_state: &PeerMonitorState,
    peer_network_id: &PeerNetworkId,
) -> Result<PeerState, Error> {
    let peer_state = peer_monitor_state
        .peer_states
        .read()
        .get(peer_network_id)
        .cloned();
    peer_state.ok_or_else(|| {
        Error::UnexpectedError(format!(
            "Failed to find the peer state. This shouldn't happen! Peer: {:?}",
            peer_network_id
        ))
    })
}

/// Updates the in-flight metrics for on-going requests
fn update_in_flight_metrics(peer_state_key: PeerStateKey, num_in_flight_requests: u64) {
    let request_label = peer_state_key.get_metrics_request_label();
    metrics::update_in_flight_requests(request_label, num_in_flight_requests);
}
