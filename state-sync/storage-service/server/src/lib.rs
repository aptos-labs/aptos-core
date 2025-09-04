// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{
    logging::{LogEntry, LogSchema},
    network::StorageServiceNetworkEvents,
    subscription::SubscriptionStreamRequests,
};
use velor_channels::{velor_channel, message_queues::QueueStyle};
use velor_config::{
    config::{StateSyncConfig, StorageServiceConfig},
    network_id::PeerNetworkId,
};
use velor_logger::prelude::*;
use velor_network::application::storage::PeersAndMetadata;
use velor_storage_service_notifications::StorageServiceNotificationListener;
use velor_storage_service_types::{
    requests::StorageServiceRequest,
    responses::{ProtocolMetadata, StorageServerSummary, StorageServiceResponse},
};
use velor_time_service::{TimeService, TimeServiceTrait};
use arc_swap::ArcSwap;
use dashmap::DashMap;
use error::Error;
use futures::stream::StreamExt;
use handler::Handler;
use mini_moka::sync::Cache;
use moderator::RequestModerator;
use optimistic_fetch::OptimisticFetchRequest;
use std::{ops::Deref, sync::Arc, time::Duration};
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
mod subscription;
mod utils;

#[cfg(test)]
mod tests;

// Note: we limit the queue depth to 1 because it doesn't make sense for the optimistic handler
// to execute for every notification (because it reads the latest version in the cache). Thus,
// if there are X pending notifications, the first one will refresh using the latest version and
// the next X-1 will execute with an unchanged version (thus, becoming a no-op and wasting the CPU).
const CACHED_SUMMARY_UPDATE_CHANNEL_SIZE: usize = 1;

/// The server-side actor for the storage service. Handles inbound storage
/// service requests from clients.
pub struct StorageServiceServer<T> {
    network_requests: StorageServiceNetworkEvents,
    storage: T,
    storage_service_config: StorageServiceConfig,
    time_service: TimeService,

    // A cached storage server summary to avoid hitting the DB for every
    // request. This is refreshed periodically.
    cached_storage_server_summary: Arc<ArcSwap<StorageServerSummary>>,

    // An LRU cache for commonly requested data items.
    // Note: This is not just a database cache because it contains
    // responses that have already been serialized and compressed.
    lru_response_cache: Cache<StorageServiceRequest, StorageServiceResponse>,

    // A set of active optimistic fetches for peers waiting for new data
    optimistic_fetches: Arc<DashMap<PeerNetworkId, OptimisticFetchRequest>>,

    // A set of active subscriptions for peers waiting for new data
    subscriptions: Arc<DashMap<PeerNetworkId, SubscriptionStreamRequests>>,

    // A moderator for incoming peer requests
    request_moderator: Arc<RequestModerator>,

    // The listener for notifications from state sync
    storage_service_listener: Option<StorageServiceNotificationListener>,

    // The runtime on which to spawn tasks
    runtime: Handle,
}

impl<T: StorageReaderInterface + Send + Sync> StorageServiceServer<T> {
    pub fn new(
        config: StateSyncConfig,
        runtime: Handle,
        storage: T,
        time_service: TimeService,
        peers_and_metadata: Arc<PeersAndMetadata>,
        network_requests: StorageServiceNetworkEvents,
        storage_service_listener: StorageServiceNotificationListener,
    ) -> Self {
        // Extract the individual component configs
        let velor_data_client_config = config.velor_data_client;
        let storage_service_config = config.storage_service;

        // Create the required components
        let cached_storage_server_summary =
            Arc::new(ArcSwap::from(Arc::new(StorageServerSummary::default())));
        let optimistic_fetches = Arc::new(DashMap::new());
        let lru_response_cache = Cache::new(storage_service_config.max_lru_cache_size);
        let subscriptions = Arc::new(DashMap::new());
        let request_moderator = Arc::new(RequestModerator::new(
            velor_data_client_config,
            cached_storage_server_summary.clone(),
            peers_and_metadata,
            storage_service_config,
            time_service.clone(),
        ));
        let storage_service_listener = Some(storage_service_listener);

        Self {
            network_requests,
            storage,
            storage_service_config,
            time_service,
            cached_storage_server_summary,
            lru_response_cache,
            optimistic_fetches,
            subscriptions,
            request_moderator,
            storage_service_listener,
            runtime,
        }
    }

    /// Spawns all continuously running utility tasks
    async fn spawn_continuous_storage_summary_tasks(&mut self) {
        // Create channels to notify the optimistic fetch and subscription
        // handlers about updates to the cached storage summary.
        let (cache_update_notifier_optimistic_fetch, cache_update_listener_optimistic_fetch) =
            velor_channel::new(QueueStyle::LIFO, CACHED_SUMMARY_UPDATE_CHANNEL_SIZE, None);
        let (cache_update_notifier_subscription, cache_update_listener_subscription) =
            velor_channel::new(QueueStyle::LIFO, CACHED_SUMMARY_UPDATE_CHANNEL_SIZE, None);

        // Spawn the refresher for the storage summary cache
        let cache_update_notifiers = vec![
            cache_update_notifier_optimistic_fetch.clone(),
            cache_update_notifier_subscription.clone(),
        ];
        self.spawn_storage_summary_refresher(cache_update_notifiers)
            .await;

        // Spawn the optimistic fetch handler
        self.spawn_optimistic_fetch_handler(cache_update_listener_optimistic_fetch)
            .await;

        // Spawn the subscription handler
        self.spawn_subscription_handler(cache_update_listener_subscription)
            .await;

        // Spawn the refresher for the request moderator
        self.spawn_moderator_peer_refresher().await;
    }

    /// Spawns a non-terminating task that refreshes the cached storage server summary
    async fn spawn_storage_summary_refresher(
        &mut self,
        cache_update_notifiers: Vec<velor_channel::Sender<(), CachedSummaryUpdateNotification>>,
    ) {
        // Clone all required components for the task
        let cached_storage_server_summary = self.cached_storage_server_summary.clone();
        let config = self.storage_service_config;
        let storage = self.storage.clone();
        let time_service = self.time_service.clone();

        // Take the storage service listener
        let mut storage_service_listener = self
            .storage_service_listener
            .take()
            .expect("The storage service listener must be present!");

        // Spawn the task
        self.runtime.spawn(async move {
            // Create a ticker for the refresh interval
            let duration = Duration::from_millis(config.storage_summary_refresh_interval_ms);
            let ticker = time_service.interval(duration);
            futures::pin_mut!(ticker);

            // Continuously refresh the cache
            loop {
                futures::select! {
                    _ = ticker.select_next_some() => {
                        // Refresh the cache periodically
                        refresh_cached_storage_summary(
                            cached_storage_server_summary.clone(),
                            storage.clone(),
                            config,
                            cache_update_notifiers.clone(),
                        )
                    },
                    notification = storage_service_listener.select_next_some() => {
                        trace!(LogSchema::new(LogEntry::ReceivedCommitNotification)
                            .message(&format!(
                                "Received commit notification for highest synced version: {:?}.",
                                notification.highest_synced_version
                            ))
                        );

                        // Refresh the cache because of a commit notification
                        refresh_cached_storage_summary(
                            cached_storage_server_summary.clone(),
                            storage.clone(),
                            config,
                            cache_update_notifiers.clone(),
                        )
                    },
                }
            }
        });
    }

    /// Spawns a non-terminating task that handles optimistic fetches
    async fn spawn_optimistic_fetch_handler(
        &mut self,
        mut cached_summary_update_listener: velor_channel::Receiver<
            (),
            CachedSummaryUpdateNotification,
        >,
    ) {
        // Clone all required components for the task
        let runtime = self.runtime.clone();
        let cached_storage_server_summary = self.cached_storage_server_summary.clone();
        let config = self.storage_service_config;
        let optimistic_fetches = self.optimistic_fetches.clone();
        let lru_response_cache = self.lru_response_cache.clone();
        let request_moderator = self.request_moderator.clone();
        let storage = self.storage.clone();
        let subscriptions = self.subscriptions.clone();
        let time_service = self.time_service.clone();

        // Spawn the task
        self.runtime
            .spawn(async move {
                // Create a ticker for the refresh interval
                let duration = Duration::from_millis(config.storage_summary_refresh_interval_ms);
                let ticker = time_service.interval(duration);
                futures::pin_mut!(ticker);

                // Continuously handle the optimistic fetches
                loop {
                    futures::select! {
                        _ = ticker.select_next_some() => {
                            // Handle the optimistic fetches periodically
                            handle_active_optimistic_fetches(
                                runtime.clone(),
                                cached_storage_server_summary.clone(),
                                config,
                                optimistic_fetches.clone(),
                                lru_response_cache.clone(),
                                request_moderator.clone(),
                                storage.clone(),
                                subscriptions.clone(),
                                time_service.clone(),
                            ).await;
                        },
                        notification = cached_summary_update_listener.select_next_some() => {
                            trace!(LogSchema::new(LogEntry::ReceivedCacheUpdateNotification)
                                .message(&format!("Received cache update notification for optimistic fetch handler! Highest synced version: {:?}", notification.highest_synced_version))
                            );

                            // Handle the optimistic fetches because of a cache update
                            handle_active_optimistic_fetches(
                                runtime.clone(),
                                cached_storage_server_summary.clone(),
                                config,
                                optimistic_fetches.clone(),
                                lru_response_cache.clone(),
                                request_moderator.clone(),
                                storage.clone(),
                                subscriptions.clone(),
                                time_service.clone(),
                            ).await;
                        },
                    }
                }
            });
    }

    /// Spawns a non-terminating task that handles subscriptions
    async fn spawn_subscription_handler(
        &mut self,
        mut cached_summary_update_listener: velor_channel::Receiver<
            (),
            CachedSummaryUpdateNotification,
        >,
    ) {
        // Clone all required components for the task
        let runtime = self.runtime.clone();
        let cached_storage_server_summary = self.cached_storage_server_summary.clone();
        let config = self.storage_service_config;
        let optimistic_fetches = self.optimistic_fetches.clone();
        let lru_response_cache = self.lru_response_cache.clone();
        let request_moderator = self.request_moderator.clone();
        let storage = self.storage.clone();
        let subscriptions = self.subscriptions.clone();
        let time_service = self.time_service.clone();

        // Spawn the task
        self.runtime
            .spawn(async move {
                // Create a ticker for the refresh interval
                let duration = Duration::from_millis(config.storage_summary_refresh_interval_ms);
                let ticker = time_service.interval(duration);
                futures::pin_mut!(ticker);

                // Continuously handle the subscriptions
                loop {
                    futures::select! {
                        _ = ticker.select_next_some() => {
                            // Handle the subscriptions periodically
                            handle_active_subscriptions(
                                runtime.clone(),
                                cached_storage_server_summary.clone(),
                                config,
                                optimistic_fetches.clone(),
                                lru_response_cache.clone(),
                                request_moderator.clone(),
                                storage.clone(),
                                subscriptions.clone(),
                                time_service.clone(),
                            ).await;
                        },
                        notification = cached_summary_update_listener.select_next_some() => {
                            trace!(LogSchema::new(LogEntry::ReceivedCacheUpdateNotification)
                                .message(&format!("Received cache update notification for subscription handler! Highest synced version: {:?}", notification.highest_synced_version))
                            );

                            // Handle the subscriptions because of a cache update
                            handle_active_subscriptions(
                                                                runtime.clone(),
                                cached_storage_server_summary.clone(),
                                config,
                                optimistic_fetches.clone(),
                                lru_response_cache.clone(),
                                request_moderator.clone(),
                                storage.clone(),
                                subscriptions.clone(),
                                time_service.clone(),
                            ).await;
                        },
                    }
                }
            });
    }

    /// Spawns a non-terminating task that refreshes the unhealthy
    /// peer states in the request moderator.
    async fn spawn_moderator_peer_refresher(&mut self) {
        // Clone all required components for the task
        let config = self.storage_service_config;
        let request_moderator = self.request_moderator.clone();
        let time_service = self.time_service.clone();

        // Spawn the task
        self.runtime.spawn(async move {
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
        });
    }

    /// Starts the storage service server thread
    pub async fn start(mut self) {
        // Spawn the continuously running tasks
        self.spawn_continuous_storage_summary_tasks().await;

        // Handle the storage requests as they arrive
        while let Some(network_request) = self.network_requests.next().await {
            // All handler methods are currently CPU-bound and synchronous
            // I/O-bound, so we want to spawn on the blocking thread pool to
            // avoid starving other async tasks on the same runtime.
            let storage = self.storage.clone();
            let config = self.storage_service_config;
            let cached_storage_server_summary = self.cached_storage_server_summary.clone();
            let optimistic_fetches = self.optimistic_fetches.clone();
            let subscriptions = self.subscriptions.clone();
            let lru_response_cache = self.lru_response_cache.clone();
            let request_moderator = self.request_moderator.clone();
            let time_service = self.time_service.clone();
            self.runtime.spawn_blocking(move || {
                Handler::new(
                    cached_storage_server_summary,
                    optimistic_fetches,
                    lru_response_cache,
                    request_moderator,
                    storage,
                    subscriptions,
                    time_service,
                )
                .process_request_and_respond(
                    config,
                    network_request.peer_network_id,
                    network_request.protocol_id,
                    network_request.storage_service_request,
                    network_request.response_sender,
                );
            });
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
    ) -> Arc<DashMap<PeerNetworkId, OptimisticFetchRequest>> {
        self.optimistic_fetches.clone()
    }

    #[cfg(test)]
    /// Returns a copy of the active subscriptions for test purposes
    pub(crate) fn get_subscriptions(
        &self,
    ) -> Arc<DashMap<PeerNetworkId, SubscriptionStreamRequests>> {
        self.subscriptions.clone()
    }
}

/// Handles the active optimistic fetches and logs any
/// errors that were encountered.
async fn handle_active_optimistic_fetches<T: StorageReaderInterface>(
    runtime: Handle,
    cached_storage_server_summary: Arc<ArcSwap<StorageServerSummary>>,
    config: StorageServiceConfig,
    optimistic_fetches: Arc<DashMap<PeerNetworkId, OptimisticFetchRequest>>,
    lru_response_cache: Cache<StorageServiceRequest, StorageServiceResponse>,
    request_moderator: Arc<RequestModerator>,
    storage: T,
    subscriptions: Arc<DashMap<PeerNetworkId, SubscriptionStreamRequests>>,
    time_service: TimeService,
) {
    if let Err(error) = optimistic_fetch::handle_active_optimistic_fetches(
        runtime,
        cached_storage_server_summary,
        config,
        optimistic_fetches,
        lru_response_cache,
        request_moderator,
        storage,
        subscriptions,
        time_service,
    )
    .await
    {
        error!(LogSchema::new(LogEntry::OptimisticFetchRefresh)
            .error(&error)
            .message("Failed to handle active optimistic fetches!"));
    }
}

/// Handles the active subscriptions and logs any
/// errors that were encountered.
async fn handle_active_subscriptions<T: StorageReaderInterface>(
    runtime: Handle,
    cached_storage_server_summary: Arc<ArcSwap<StorageServerSummary>>,
    config: StorageServiceConfig,
    optimistic_fetches: Arc<DashMap<PeerNetworkId, OptimisticFetchRequest>>,
    lru_response_cache: Cache<StorageServiceRequest, StorageServiceResponse>,
    request_moderator: Arc<RequestModerator>,
    storage: T,
    subscriptions: Arc<DashMap<PeerNetworkId, SubscriptionStreamRequests>>,
    time_service: TimeService,
) {
    if let Err(error) = subscription::handle_active_subscriptions(
        runtime,
        cached_storage_server_summary,
        config,
        optimistic_fetches,
        lru_response_cache,
        request_moderator,
        storage,
        subscriptions,
        time_service,
    )
    .await
    {
        error!(LogSchema::new(LogEntry::SubscriptionRequest)
            .error(&error)
            .message("Failed to handle active subscriptions!"));
    }
}

/// Refreshes the cached storage server summary and sends
/// a notification via the given channels. If an error
/// occurs, it is logged.
pub(crate) fn refresh_cached_storage_summary<T: StorageReaderInterface>(
    cached_storage_server_summary: Arc<ArcSwap<StorageServerSummary>>,
    storage: T,
    storage_config: StorageServiceConfig,
    cache_update_notifiers: Vec<velor_channel::Sender<(), CachedSummaryUpdateNotification>>,
) {
    // Fetch the new data summary from storage
    let new_data_summary = match storage.get_data_summary() {
        Ok(data_summary) => data_summary,
        Err(error) => {
            error!(LogSchema::new(LogEntry::StorageSummaryRefresh)
                .error(&Error::StorageErrorEncountered(error.to_string()))
                .message("Failed to refresh the cached storage summary!"));
            return;
        },
    };

    // Initialize the protocol metadata
    let new_protocol_metadata = ProtocolMetadata {
        max_epoch_chunk_size: storage_config.max_epoch_chunk_size,
        max_transaction_chunk_size: storage_config.max_transaction_chunk_size,
        max_state_chunk_size: storage_config.max_state_chunk_size,
        max_transaction_output_chunk_size: storage_config.max_transaction_output_chunk_size,
    };

    // Create the new storage server summary
    let new_storage_server_summary = StorageServerSummary {
        protocol_metadata: new_protocol_metadata,
        data_summary: new_data_summary,
    };

    // If the new storage server summary is different to the existing one,
    // update the cache and send a notification via the notifier channel.
    let existing_storage_server_summary = cached_storage_server_summary.load().clone();
    if existing_storage_server_summary.deref().clone() != new_storage_server_summary {
        // Update the storage server summary cache
        cached_storage_server_summary.store(Arc::new(new_storage_server_summary.clone()));

        // Create an update notification
        let highest_synced_version = new_storage_server_summary
            .data_summary
            .get_synced_ledger_info_version();
        let update_notification = CachedSummaryUpdateNotification::new(highest_synced_version);

        // Send a notification via each notifier channel
        for cached_summary_update_notifier in cache_update_notifiers {
            if let Err(error) = cached_summary_update_notifier.push((), update_notification) {
                error!(LogSchema::new(LogEntry::StorageSummaryRefresh)
                    .error(&Error::StorageErrorEncountered(error.to_string()))
                    .message("Failed to send an update notification for the new cached summary!"));
            }
        }
    }
}

/// A simple notification sent to the optimistic fetch handler that the
/// cached storage summary has been updated with the specified version.
#[derive(Clone, Copy)]
pub struct CachedSummaryUpdateNotification {
    highest_synced_version: Option<u64>,
}

impl CachedSummaryUpdateNotification {
    pub fn new(highest_synced_version: Option<u64>) -> Self {
        Self {
            highest_synced_version,
        }
    }
}
