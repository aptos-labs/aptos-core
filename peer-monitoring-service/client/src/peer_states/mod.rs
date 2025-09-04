// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    metrics, Error, LogEntry, LogEvent, LogSchema, PeerMonitorState, PeerMonitoringServiceClient,
    PeerState,
};
use velor_config::{config::PeerMonitoringServiceConfig, network_id::PeerNetworkId};
use velor_logger::{info, sample, sample::SampleRate};
use velor_network::application::{interface::NetworkClient, metadata::PeerMetadata};
use velor_peer_monitoring_service_types::PeerMonitoringServiceMessage;
use velor_time_service::TimeService;
use key_value::PeerStateKey;
use std::{collections::HashMap, time::Duration};
use tokio::runtime::Handle;

pub mod key_value;
pub mod latency_info;
pub mod network_info;
pub mod node_info;
pub mod peer_state;
mod request_tracker;

// Useful constants
const LOGS_FREQUENCY_SECS: u64 = 180; // 3 minutes
const METRICS_FREQUENCY_SECS: u64 = 60; // 1 minute

/// Refreshes the states of the connected peers
pub fn refresh_peer_states(
    monitoring_service_config: &PeerMonitoringServiceConfig,
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
                    monitoring_service_config,
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

    // Periodically update the metrics
    sample!(
        SampleRate::Duration(Duration::from_secs(METRICS_FREQUENCY_SECS)),
        update_peer_state_metrics(&peer_monitor_state, &connected_peers_and_metadata)?;
    );

    // Periodically update the logs
    sample!(
        SampleRate::Duration(Duration::from_secs(LOGS_FREQUENCY_SECS)),
        update_peer_state_logs(&peer_monitor_state, &connected_peers_and_metadata)?;
    );

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

/// Updates the logs for the peer monitoring states
fn update_peer_state_logs(
    peer_monitor_state: &PeerMonitorState,
    connected_peers_and_metadata: &HashMap<PeerNetworkId, PeerMetadata>,
) -> Result<(), Error> {
    // Get the list of connected peers
    let connected_peers: Vec<PeerNetworkId> =
        connected_peers_and_metadata.keys().cloned().collect();

    // Collect the peer states for logging
    let mut all_peer_states = HashMap::new();
    for peer_network_id in &connected_peers {
        let peer_state = get_peer_state(peer_monitor_state, peer_network_id)?;
        all_peer_states.insert(peer_network_id, format!("{}", peer_state));
    }

    // Log the peer states
    info!(LogSchema::new(LogEntry::PeerMonitorLoop)
        .event(LogEvent::LogAllPeerStates)
        .message(&format!("All peer states: {:?}", all_peer_states)));

    Ok(())
}

/// Updates the metrics for the peer monitoring states
fn update_peer_state_metrics(
    peer_monitor_state: &PeerMonitorState,
    connected_peers_and_metadata: &HashMap<PeerNetworkId, PeerMetadata>,
) -> Result<(), Error> {
    // Get the list of connected peers
    let connected_peers: Vec<PeerNetworkId> =
        connected_peers_and_metadata.keys().cloned().collect();

    // Update the peer state metrics
    for peer_state_key in PeerStateKey::get_all_keys() {
        for peer_network_id in &connected_peers {
            // Get the peer state and update the metrics
            let peer_state = get_peer_state(peer_monitor_state, peer_network_id)?;
            peer_state.update_peer_state_metrics(peer_network_id, &peer_state_key)?;
        }
    }

    Ok(())
}
