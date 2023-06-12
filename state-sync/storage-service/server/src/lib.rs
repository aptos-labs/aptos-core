// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{
    logging::{LogEntry, LogSchema},
    network::StorageServiceNetworkEvents,
};
use aptos_bounded_executor::BoundedExecutor;
use aptos_config::{config::StorageServiceConfig, network_id::PeerNetworkId};
use aptos_infallible::{Mutex, RwLock};
use aptos_logger::prelude::*;
use aptos_network::application::storage::PeersAndMetadata;
use aptos_storage_service_types::{
    requests::StorageServiceRequest,
    responses::{ProtocolMetadata, StorageServerSummary, StorageServiceResponse},
};
use aptos_time_service::{TimeService, TimeServiceTrait};
use error::Error;
use futures::stream::StreamExt;
use handler::Handler;
use lru::LruCache;
use moderator::RequestModerator;
use optimistic_fetch::OptimisticFetchRequest;
use std::{collections::HashMap, sync::Arc, time::Duration};
use storage::StorageReaderInterface;
use thiserror::Error;
use tokio::runtime::Handle;

mod error;
mod handler;
mod logging;
pub mod metrics;
mod moderator;
pub mod network;
mod optimistic_fetch;
pub mod storage;

#[cfg(test)]
mod tests;

/// The server-side actor for the storage service. Handles inbound storage
/// service requests from clients.
pub struct StorageServiceServer<T> {
    bounded_executor: BoundedExecutor,
    config: StorageServiceConfig,
    network_requests: StorageServiceNetworkEvents,
    storage: T,
    time_service: TimeService,

    // A cached storage server summary to avoid hitting the DB for every
    // request. This is refreshed periodically.
    cached_storage_server_summary: Arc<RwLock<StorageServerSummary>>,

    // An LRU cache for commonly requested data items.
    // Note: This is not just a database cache because it contains
    // responses that have already been serialized and compressed.
    lru_response_cache: Arc<Mutex<LruCache<StorageServiceRequest, StorageServiceResponse>>>,

    // A set of active optimistic fetches for peers waiting for new data
    optimistic_fetches: Arc<Mutex<HashMap<PeerNetworkId, OptimisticFetchRequest>>>,

    // A moderator for incoming peer requests
    request_moderator: Arc<RequestModerator>,
}

impl<T: StorageReaderInterface> StorageServiceServer<T> {
    pub fn new(
        config: StorageServiceConfig,
        executor: Handle,
        storage: T,
        time_service: TimeService,
        peers_and_metadata: Arc<PeersAndMetadata>,
        network_requests: StorageServiceNetworkEvents,
    ) -> Self {
        let bounded_executor =
            BoundedExecutor::new(config.max_concurrent_requests as usize, executor);
        let cached_storage_server_summary = Arc::new(RwLock::new(StorageServerSummary::default()));
        let optimistic_fetches = Arc::new(Mutex::new(HashMap::new()));
        let lru_response_cache = Arc::new(Mutex::new(LruCache::new(
            config.max_lru_cache_size as usize,
        )));
        let request_moderator = Arc::new(RequestModerator::new(
            cached_storage_server_summary.clone(),
            peers_and_metadata,
            config,
            time_service.clone(),
        ));

        Self {
            config,
            bounded_executor,
            storage,
            network_requests,
            time_service,
            cached_storage_server_summary,
            lru_response_cache,
            optimistic_fetches,
            request_moderator,
        }
    }

    /// Spawns a non-terminating task that refreshes the cached storage server summary
    async fn spawn_storage_summary_refresher(&mut self) {
        let cached_storage_server_summary = self.cached_storage_server_summary.clone();
        let config = self.config;
        let storage = self.storage.clone();
        let time_service = self.time_service.clone();

        // Spawn the task
        self.bounded_executor
            .spawn(async move {
                // Create a ticker for the refresh interval
                let duration = Duration::from_millis(config.storage_summary_refresh_interval_ms);
                let ticker = time_service.interval(duration);
                futures::pin_mut!(ticker);

                // Periodically refresh the cache
                loop {
                    ticker.next().await;

                    // Refresh the cache
                    if let Err(error) = refresh_cached_storage_summary(
                        cached_storage_server_summary.clone(),
                        storage.clone(),
                        config,
                    ) {
                        error!(LogSchema::new(LogEntry::StorageSummaryRefresh)
                            .error(&error)
                            .message("Failed to refresh the cached storage summary!"));
                    }
                }
            })
            .await;
    }

    /// Spawns a non-terminating task that handles optimistic fetches
    async fn spawn_optimistic_fetch_handler(&mut self) {
        let cached_storage_server_summary = self.cached_storage_server_summary.clone();
        let config = self.config;
        let optimistic_fetches = self.optimistic_fetches.clone();
        let lru_response_cache = self.lru_response_cache.clone();
        let request_moderator = self.request_moderator.clone();
        let storage = self.storage.clone();
        let time_service = self.time_service.clone();

        // Spawn the task
        self.bounded_executor
            .spawn(async move {
                // Create a ticker for the refresh interval
                let duration = Duration::from_millis(config.storage_summary_refresh_interval_ms);
                let ticker = time_service.interval(duration);
                futures::pin_mut!(ticker);

                // Periodically check the optimistic fetches
                loop {
                    ticker.next().await;

                    // Check and handle the active optimistic fetches
                    if let Err(error) = optimistic_fetch::handle_active_optimistic_fetches(
                        cached_storage_server_summary.clone(),
                        config,
                        optimistic_fetches.clone(),
                        lru_response_cache.clone(),
                        request_moderator.clone(),
                        storage.clone(),
                        time_service.clone(),
                    ) {
                        error!(LogSchema::new(LogEntry::OptimisticFetchRefresh)
                            .error(&error)
                            .message("Failed to handle active optimistic fetches!"));
                    }
                }
            })
            .await;
    }

    /// Spawns a non-terminating task that refreshes the unhealthy
    /// peer states in the request moderator.
    async fn spawn_moderator_peer_refresher(&mut self) {
        let config = self.config;
        let request_moderator = self.request_moderator.clone();
        let time_service = self.time_service.clone();

        // Spawn the task
        self.bounded_executor
            .spawn(async move {
                // Create a ticker for the refresh interval
                let duration = Duration::from_millis(config.request_moderator_refresh_interval_ms);
                let ticker = time_service.interval(duration);
                futures::pin_mut!(ticker);

                // Periodically refresh the peer states
                loop {
                    ticker.next().await;

                    // Refresh the unhealthy peer states
                    if let Err(error) = request_moderator.refresh_unhealthy_peer_states() {
                        error!(LogSchema::new(LogEntry::RequestModeratorRefresh)
                            .error(&error)
                            .message("Failed to refresh the request moderator!"));
                    }
                }
            })
            .await;
    }

    /// Starts the storage service server thread
    pub async fn start(mut self) {
        // Spawn the refresher for the storage summary cache
        self.spawn_storage_summary_refresher().await;

        // Spawn the optimistic fetch handler
        self.spawn_optimistic_fetch_handler().await;

        // Spawn the refresher for the request moderator
        self.spawn_moderator_peer_refresher().await;

        // Handle the storage requests
        while let Some(network_request) = self.network_requests.next().await {
            // Log the request
            let peer_network_id = network_request.peer_network_id;
            let protocol_id = network_request.protocol_id;
            let storage_service_request = network_request.storage_service_request;
            trace!(LogSchema::new(LogEntry::ReceivedStorageRequest)
                .request(&storage_service_request)
                .message(&format!(
                    "Received storage request. Peer: {:?}, protocol: {:?}.",
                    peer_network_id, protocol_id,
                )));

            // All handler methods are currently CPU-bound and synchronous
            // I/O-bound, so we want to spawn on the blocking thread pool to
            // avoid starving other async tasks on the same runtime.
            let storage = self.storage.clone();
            let cached_storage_server_summary = self.cached_storage_server_summary.clone();
            let optimistic_fetches = self.optimistic_fetches.clone();
            let lru_response_cache = self.lru_response_cache.clone();
            let request_moderator = self.request_moderator.clone();
            let time_service = self.time_service.clone();
            self.bounded_executor
                .spawn_blocking(move || {
                    Handler::new(
                        cached_storage_server_summary,
                        optimistic_fetches,
                        lru_response_cache,
                        request_moderator,
                        storage,
                        time_service,
                    )
                    .process_request_and_respond(
                        peer_network_id,
                        storage_service_request,
                        network_request.response_sender,
                    );
                })
                .await;
        }
    }

    #[cfg(test)]
    /// Returns a copy of the request moderator for test purposes
    pub(crate) fn get_request_moderator(&self) -> Arc<RequestModerator> {
        self.request_moderator.clone()
    }

    #[cfg(test)]
    /// Returns a copy of the active optimistic fetches for test purposes
    pub(crate) fn get_optimistic_fetches(
        &self,
    ) -> Arc<Mutex<HashMap<PeerNetworkId, OptimisticFetchRequest>>> {
        self.optimistic_fetches.clone()
    }
}

/// Refreshes the cached storage server summary
fn refresh_cached_storage_summary<T: StorageReaderInterface>(
    cached_storage_server_summary: Arc<RwLock<StorageServerSummary>>,
    storage: T,
    storage_config: StorageServiceConfig,
) -> Result<(), Error> {
    // Fetch the data summary from storage
    let data_summary = storage
        .get_data_summary()
        .map_err(|error| Error::StorageErrorEncountered(error.to_string()))?;

    // Initialize the protocol metadata
    let protocol_metadata = ProtocolMetadata {
        max_epoch_chunk_size: storage_config.max_epoch_chunk_size,
        max_transaction_chunk_size: storage_config.max_transaction_chunk_size,
        max_state_chunk_size: storage_config.max_state_chunk_size,
        max_transaction_output_chunk_size: storage_config.max_transaction_output_chunk_size,
    };

    // Save the storage server summary
    let storage_server_summary = StorageServerSummary {
        protocol_metadata,
        data_summary,
    };
    *cached_storage_server_summary.write() = storage_server_summary;

    Ok(())
}
