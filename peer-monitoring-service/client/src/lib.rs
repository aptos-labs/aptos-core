// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::logging::{LogEntry, LogEvent, LogSchema};
use velor_config::{
    config::{NodeConfig, PeerMonitoringServiceConfig},
    network_id::PeerNetworkId,
};
use velor_id_generator::U64IdGenerator;
use velor_infallible::RwLock;
use velor_logger::{info, warn};
use velor_network::application::{
    interface::NetworkClient, metadata::PeerMetadata, storage::PeersAndMetadata,
};
use velor_peer_monitoring_service_types::{PeerMonitoringMetadata, PeerMonitoringServiceMessage};
use velor_time_service::{TimeService, TimeServiceTrait};
use error::Error;
use futures::StreamExt;
use network::PeerMonitoringServiceClient;
use peer_states::peer_state::PeerState;
use std::{collections::HashMap, sync::Arc, time::Duration};
use thiserror::Error;
use tokio::{runtime::Handle, task::JoinHandle};

mod error;
mod logging;
mod metrics;
mod network;
pub mod peer_states;
#[cfg(test)]
mod tests;

/// A simple container that holds the state of the peer monitor
#[derive(Clone, Debug, Default)]
pub struct PeerMonitorState {
    peer_states: Arc<RwLock<HashMap<PeerNetworkId, PeerState>>>, // Map of peers to states
    request_id_generator: Arc<U64IdGenerator>, // Used for generating request/response IDs
}

impl PeerMonitorState {
    pub fn new() -> Self {
        Self {
            peer_states: Arc::new(RwLock::new(HashMap::new())),
            request_id_generator: Arc::new(U64IdGenerator::new()),
        }
    }

    /// Returns the peer state for the given peer (only used for testing)
    #[cfg(test)]
    pub fn get_peer_state(&self, peer_network_id: &PeerNetworkId) -> Option<PeerState> {
        self.peer_states.read().get(peer_network_id).cloned()
    }
}

/// Runs the peer monitor that continuously monitors
/// the state of the peers.
pub async fn start_peer_monitor(
    node_config: NodeConfig,
    network_client: NetworkClient<PeerMonitoringServiceMessage>,
    runtime: Option<Handle>,
) {
    // Create a new monitoring client and peer monitor state
    let peer_monitoring_client = PeerMonitoringServiceClient::new(network_client);
    let peer_monitor_state = PeerMonitorState::new();

    // Spawn the peer metadata updater
    let time_service = TimeService::real();
    spawn_peer_metadata_updater(
        node_config.peer_monitoring_service,
        peer_monitor_state.clone(),
        peer_monitoring_client.get_peers_and_metadata(),
        time_service.clone(),
        runtime.clone(),
    );

    // Start the peer monitor
    start_peer_monitor_with_state(
        node_config,
        peer_monitoring_client,
        peer_monitor_state,
        time_service,
        runtime,
    )
    .await
}

/// A helpful utility function for spawning the peer
/// monitoring client with the given state.
async fn start_peer_monitor_with_state(
    node_config: NodeConfig,
    peer_monitoring_client: PeerMonitoringServiceClient<
        NetworkClient<PeerMonitoringServiceMessage>,
    >,
    peer_monitor_state: PeerMonitorState,
    time_service: TimeService,
    runtime: Option<Handle>,
) {
    // Get the peers and metadata
    let peers_and_metadata = peer_monitoring_client.get_peers_and_metadata();

    // Create an interval ticker for the monitor loop
    let monitoring_service_config = node_config.peer_monitoring_service;
    let peer_monitor_duration =
        Duration::from_micros(monitoring_service_config.peer_monitor_interval_usec);
    let peer_monitor_ticker = time_service.interval(peer_monitor_duration);
    futures::pin_mut!(peer_monitor_ticker);

    // Start the peer monitoring loop
    info!(LogSchema::new(LogEntry::PeerMonitorLoop)
        .event(LogEvent::StartedPeerMonitorLoop)
        .message("Starting the peer monitor!"));
    loop {
        // Wait for the next round before pinging peers
        peer_monitor_ticker.next().await;

        // Get all connected peers
        let connected_peers_and_metadata =
            match peers_and_metadata.get_connected_peers_and_metadata() {
                Ok(connected_peers_and_metadata) => connected_peers_and_metadata,
                Err(error) => {
                    warn!(LogSchema::new(LogEntry::PeerMonitorLoop)
                        .event(LogEvent::UnexpectedErrorEncountered)
                        .error(&error.into())
                        .message("Failed to get connected peers and metadata!"));
                    continue; // Move to the next loop iteration
                },
            };

        // Garbage collect the peer states (to remove disconnected peers)
        garbage_collect_peer_states(&peer_monitor_state, &connected_peers_and_metadata);

        // Ensure all peers have a state (and create one for newly connected peers)
        create_states_for_new_peers(
            &node_config,
            &peer_monitor_state,
            &time_service,
            &connected_peers_and_metadata,
        );

        // Refresh the peer states
        if let Err(error) = peer_states::refresh_peer_states(
            &monitoring_service_config,
            peer_monitor_state.clone(),
            peer_monitoring_client.clone(),
            connected_peers_and_metadata,
            time_service.clone(),
            runtime.clone(),
        ) {
            warn!(LogSchema::new(LogEntry::PeerMonitorLoop)
                .event(LogEvent::UnexpectedErrorEncountered)
                .error(&error)
                .message("Failed to refresh peer states!"));
        }
    }
}

/// Creates a new peer state for peers that don't yet have one
fn create_states_for_new_peers(
    node_config: &NodeConfig,
    peer_monitor_state: &PeerMonitorState,
    time_service: &TimeService,
    connected_peers_and_metadata: &HashMap<PeerNetworkId, PeerMetadata>,
) {
    for peer_network_id in connected_peers_and_metadata.keys() {
        let state_exists = peer_monitor_state
            .peer_states
            .read()
            .contains_key(peer_network_id);
        if !state_exists {
            peer_monitor_state.peer_states.write().insert(
                *peer_network_id,
                PeerState::new(node_config.clone(), time_service.clone()),
            );
        }
    }
}

/// Garbage collects peer states for peers that are no longer connected
fn garbage_collect_peer_states(
    peer_monitor_state: &PeerMonitorState,
    connected_peers_and_metadata: &HashMap<PeerNetworkId, PeerMetadata>,
) {
    // Get the set of peers with existing states
    let peers_with_existing_states: Vec<PeerNetworkId> = peer_monitor_state
        .peer_states
        .read()
        .keys()
        .cloned()
        .collect();

    // Remove the states for disconnected peers
    for peer_network_id in peers_with_existing_states {
        if !connected_peers_and_metadata.contains_key(&peer_network_id) {
            peer_monitor_state
                .peer_states
                .write()
                .remove(&peer_network_id);
        }
    }
}

/// Spawns a task that continuously updates the peers and metadata
/// struct with the latest information stored for each peer.
pub(crate) fn spawn_peer_metadata_updater(
    peer_monitoring_config: PeerMonitoringServiceConfig,
    peer_monitor_state: PeerMonitorState,
    peers_and_metadata: Arc<PeersAndMetadata>,
    time_service: TimeService,
    runtime: Option<Handle>,
) -> JoinHandle<()> {
    // Create the updater task for the peers and metadata struct
    let metadata_updater = async move {
        // Create an interval ticker for the updater loop
        let metadata_update_loop_duration =
            Duration::from_millis(peer_monitoring_config.metadata_update_interval_ms);
        let metadata_update_loop_ticker = time_service.interval(metadata_update_loop_duration);
        futures::pin_mut!(metadata_update_loop_ticker);

        // Start the updater loop
        info!(LogSchema::new(LogEntry::MetadataUpdateLoop)
            .event(LogEvent::StartedMetadataUpdaterLoop)
            .message("Starting the peers and metadata updater!"));
        loop {
            // Wait for the next round before updating peers and metadata
            metadata_update_loop_ticker.next().await;

            // Get all peers
            let all_peers = peers_and_metadata.get_all_peers();

            // Update the latest peer monitoring metadata
            for peer_network_id in all_peers {
                let peer_monitoring_metadata =
                    match peer_monitor_state.peer_states.read().get(&peer_network_id) {
                        Some(peer_state) => {
                            peer_state
                                .extract_peer_monitoring_metadata()
                                .unwrap_or_else(|error| {
                                    // Log the error and return the default
                                    warn!(LogSchema::new(LogEntry::MetadataUpdateLoop)
                                        .event(LogEvent::UnexpectedErrorEncountered)
                                        .peer(&peer_network_id)
                                        .error(&error));
                                    PeerMonitoringMetadata::default()
                                })
                        },
                        None => PeerMonitoringMetadata::default(), // Use the default
                    };

                // Insert the latest peer monitoring metadata into peers and metadata
                if let Err(error) = peers_and_metadata
                    .update_peer_monitoring_metadata(peer_network_id, peer_monitoring_metadata)
                {
                    warn!(LogSchema::new(LogEntry::MetadataUpdateLoop)
                        .event(LogEvent::UnexpectedErrorEncountered)
                        .peer(&peer_network_id)
                        .error(&error.into()));
                }
            }
        }
    };

    // Spawn the peer metadata updater task
    if let Some(runtime) = runtime {
        runtime.spawn(metadata_updater)
    } else {
        tokio::spawn(metadata_updater)
    }
}
