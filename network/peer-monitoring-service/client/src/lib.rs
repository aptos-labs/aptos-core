// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::logging::{LogEntry, LogEvent, LogSchema};
use aptos_config::{
    config::{NodeConfig, PeerMonitoringServiceConfig},
    network_id::PeerNetworkId,
};
use aptos_id_generator::U64IdGenerator;
use aptos_infallible::RwLock;
use aptos_logger::{info, warn};
use aptos_network::application::{
    interface::NetworkClient, metadata::PeerMonitoringMetadata, storage::PeersAndMetadata,
};
use aptos_peer_monitoring_service_types::PeerMonitoringServiceMessage;
use aptos_time_service::{TimeService, TimeServiceTrait};
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
mod peer_states;

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
}

/// Runs the peer monitor that continuously monitors
/// the state of the peers.
pub async fn start_peer_monitor(
    node_config: NodeConfig,
    network_client: NetworkClient<PeerMonitoringServiceMessage>,
    time_service: TimeService,
    runtime: Option<Handle>,
) {
    // Create a new client and peer monitor state
    let peer_monitoring_client = PeerMonitoringServiceClient::new(network_client);
    let peer_monitor_state = PeerMonitorState::new();

    // Get the peers and metadata struct
    let peers_and_metadata = peer_monitoring_client.get_peers_and_metadata();

    // Spawns the updater for the peers and metadata
    let peer_monitoring_config = node_config.peer_monitoring_service.clone();
    spawn_peer_metadata_updater(
        peer_monitoring_config.clone(),
        peer_monitor_state.clone(),
        peers_and_metadata.clone(),
        time_service.clone(),
        runtime.clone(),
    );

    // Create an interval ticker for the monitor loop
    let peer_monitor_duration =
        Duration::from_millis(peer_monitoring_config.peer_monitor_interval_ms);
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

        // Ensure all peers have a state (and create one for newly connected peers)
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

        // Refresh the peer states
        if let Err(error) = peer_states::refresh_peer_states(
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

/// Spawns a task that continuously updates the peers and metadata
/// struct with the latest information stored for each peer.
fn spawn_peer_metadata_updater(
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
            let all_peers = match peers_and_metadata.get_all_peers() {
                Ok(all_peers) => all_peers,
                Err(error) => {
                    warn!(LogSchema::new(LogEntry::MetadataUpdateLoop)
                        .event(LogEvent::UnexpectedErrorEncountered)
                        .error(&error.into())
                        .message("Failed to get all peers!"));
                    continue; // Move to the next loop iteration
                },
            };

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
