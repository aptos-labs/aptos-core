// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::Error,
    metrics,
    metrics::{increment_counter, OPTIMISTIC_FETCH_EXPIRE},
    moderator::RequestModerator,
    network::ResponseSender,
    storage::StorageReaderInterface,
    subscription::SubscriptionStreamRequests,
    utils, LogEntry, LogSchema,
};
use aptos_config::{
    config::StorageServiceConfig,
    network_id::{NetworkId, PeerNetworkId},
};
use aptos_infallible::Mutex;
use aptos_logger::{error, warn};
use aptos_storage_service_types::{
    requests::{
        DataRequest, GetTransactionDataWithProofRequest, StorageServiceRequest,
        TransactionDataRequestType, TransactionOutputsWithProofRequest,
        TransactionsOrOutputsWithProofRequest, TransactionsWithProofRequest,
    },
    responses::{StorageServerSummary, StorageServiceResponse},
};
use aptos_time_service::{TimeService, TimeServiceTrait};
use aptos_types::ledger_info::LedgerInfoWithSignatures;
use arc_swap::ArcSwap;
use dashmap::DashMap;
use futures::future::join_all;
use mini_moka::sync::Cache;
use std::{cmp::min, collections::HashMap, ops::Deref, sync::Arc, time::Instant};
use tokio::runtime::Handle;

/// An optimistic fetch request from a peer
pub struct OptimisticFetchRequest {
    request: StorageServiceRequest,
    response_sender: ResponseSender,
    fetch_start_time: Instant,
    time_service: TimeService,
}

impl OptimisticFetchRequest {
    pub fn new(
        request: StorageServiceRequest,
        response_sender: ResponseSender,
        time_service: TimeService,
    ) -> Self {
        Self {
            request,
            response_sender,
            fetch_start_time: time_service.now(),
            time_service,
        }
    }

    /// Creates a new storage service request to satisfy the optimistic fetch
    /// using the new data at the specified `target_ledger_info`.
    pub fn get_storage_request_for_missing_data(
        &self,
        config: StorageServiceConfig,
        target_ledger_info: &LedgerInfoWithSignatures,
    ) -> aptos_storage_service_types::Result<StorageServiceRequest, Error> {
        // Verify that the target version is higher than the highest known version
        let known_version = self.highest_known_version();
        let target_version = target_ledger_info.ledger_info().version();
        if target_version <= known_version {
            return Err(Error::InvalidRequest(format!(
                "Target version: {:?} is not higher than known version: {:?}!",
                target_version, known_version
            )));
        }

        // Calculate the number of versions to fetch
        let mut num_versions_to_fetch =
            target_version.checked_sub(known_version).ok_or_else(|| {
                Error::UnexpectedErrorEncountered(
                    "Number of versions to fetch has overflown!".into(),
                )
            })?;

        // Bound the number of versions to fetch by the maximum chunk size
        num_versions_to_fetch = min(
            num_versions_to_fetch,
            self.max_chunk_size_for_request(config),
        );

        // Calculate the start and end versions
        let start_version = known_version.checked_add(1).ok_or_else(|| {
            Error::UnexpectedErrorEncountered("Start version has overflown!".into())
        })?;
        let end_version = known_version
            .checked_add(num_versions_to_fetch)
            .ok_or_else(|| {
                Error::UnexpectedErrorEncountered("End version has overflown!".into())
            })?;

        // Create the storage request
        let data_request = match &self.request.data_request {
            DataRequest::GetNewTransactionOutputsWithProof(_) => {
                DataRequest::GetTransactionOutputsWithProof(TransactionOutputsWithProofRequest {
                    proof_version: target_version,
                    start_version,
                    end_version,
                })
            },
            DataRequest::GetNewTransactionsWithProof(request) => {
                DataRequest::GetTransactionsWithProof(TransactionsWithProofRequest {
                    proof_version: target_version,
                    start_version,
                    end_version,
                    include_events: request.include_events,
                })
            },
            DataRequest::GetNewTransactionsOrOutputsWithProof(request) => {
                DataRequest::GetTransactionsOrOutputsWithProof(
                    TransactionsOrOutputsWithProofRequest {
                        proof_version: target_version,
                        start_version,
                        end_version,
                        include_events: request.include_events,
                        max_num_output_reductions: request.max_num_output_reductions,
                    },
                )
            },
            DataRequest::GetNewTransactionDataWithProof(request) => {
                DataRequest::GetTransactionDataWithProof(GetTransactionDataWithProofRequest {
                    transaction_data_request_type: request.transaction_data_request_type,
                    proof_version: target_version,
                    start_version,
                    end_version,
                    max_response_bytes: request.max_response_bytes,
                })
            },
            request => unreachable!("Unexpected optimistic fetch request: {:?}", request),
        };
        let storage_request =
            StorageServiceRequest::new(data_request, self.request.use_compression);
        Ok(storage_request)
    }

    /// Returns the highest version known by the peer
    fn highest_known_version(&self) -> u64 {
        match &self.request.data_request {
            DataRequest::GetNewTransactionOutputsWithProof(request) => request.known_version,
            DataRequest::GetNewTransactionsWithProof(request) => request.known_version,
            DataRequest::GetNewTransactionsOrOutputsWithProof(request) => request.known_version,
            DataRequest::GetNewTransactionDataWithProof(request) => request.known_version,
            request => unreachable!("Unexpected optimistic fetch request: {:?}", request),
        }
    }

    /// Returns the highest epoch known by the peer
    fn highest_known_epoch(&self) -> u64 {
        match &self.request.data_request {
            DataRequest::GetNewTransactionOutputsWithProof(request) => request.known_epoch,
            DataRequest::GetNewTransactionsWithProof(request) => request.known_epoch,
            DataRequest::GetNewTransactionsOrOutputsWithProof(request) => request.known_epoch,
            DataRequest::GetNewTransactionDataWithProof(request) => request.known_epoch,
            request => unreachable!("Unexpected optimistic fetch request: {:?}", request),
        }
    }

    /// Returns the maximum chunk size for the request depending
    /// on the request type.
    fn max_chunk_size_for_request(&self, config: StorageServiceConfig) -> u64 {
        match &self.request.data_request {
            DataRequest::GetNewTransactionOutputsWithProof(_) => {
                config.max_transaction_output_chunk_size
            },
            DataRequest::GetNewTransactionsWithProof(_) => config.max_transaction_chunk_size,
            DataRequest::GetNewTransactionsOrOutputsWithProof(_) => {
                config.max_transaction_output_chunk_size
            },
            DataRequest::GetNewTransactionDataWithProof(request) => {
                match request.transaction_data_request_type {
                    TransactionDataRequestType::TransactionData(_) => {
                        config.max_transaction_chunk_size
                    },
                    TransactionDataRequestType::TransactionOutputData => {
                        config.max_transaction_output_chunk_size
                    },
                    TransactionDataRequestType::TransactionOrOutputData(_) => {
                        config.max_transaction_output_chunk_size
                    },
                }
            },
            request => unreachable!("Unexpected optimistic fetch request: {:?}", request),
        }
    }

    /// Returns true iff the optimistic fetch has expired
    fn is_expired(&self, timeout_ms: u64) -> bool {
        let current_time = self.time_service.now();
        let elapsed_time = current_time
            .duration_since(self.fetch_start_time)
            .as_millis();
        elapsed_time > timeout_ms as u128
    }

    /// Returns the response sender and consumes the request
    pub fn take_response_sender(self) -> ResponseSender {
        self.response_sender
    }
}

/// Handles active and ready optimistic fetches
pub(crate) async fn handle_active_optimistic_fetches<T: StorageReaderInterface>(
    runtime: Handle,
    cached_storage_server_summary: Arc<ArcSwap<StorageServerSummary>>,
    config: StorageServiceConfig,
    optimistic_fetches: Arc<DashMap<PeerNetworkId, OptimisticFetchRequest>>,
    lru_response_cache: Cache<StorageServiceRequest, StorageServiceResponse>,
    request_moderator: Arc<RequestModerator>,
    storage: T,
    subscriptions: Arc<DashMap<PeerNetworkId, SubscriptionStreamRequests>>,
    time_service: TimeService,
) -> Result<(), Error> {
    // Update the number of active optimistic fetches
    update_optimistic_fetch_metrics(optimistic_fetches.clone());

    // Identify the peers with ready optimistic fetches
    let peers_with_ready_optimistic_fetches = get_peers_with_ready_optimistic_fetches(
        runtime.clone(),
        config,
        cached_storage_server_summary.clone(),
        optimistic_fetches.clone(),
        lru_response_cache.clone(),
        request_moderator.clone(),
        storage.clone(),
        subscriptions.clone(),
        time_service.clone(),
    )
    .await?;

    // Remove and handle the ready optimistic fetches
    handle_ready_optimistic_fetches(
        runtime,
        cached_storage_server_summary,
        config,
        optimistic_fetches,
        lru_response_cache,
        request_moderator,
        storage,
        subscriptions,
        time_service,
        peers_with_ready_optimistic_fetches,
    )
    .await;

    Ok(())
}

/// Handles the ready optimistic fetches by removing them from the
/// active map and notifying the peer of the new data.
pub(crate) async fn handle_ready_optimistic_fetches<T: StorageReaderInterface>(
    runtime: Handle,
    cached_storage_server_summary: Arc<ArcSwap<StorageServerSummary>>,
    config: StorageServiceConfig,
    optimistic_fetches: Arc<DashMap<PeerNetworkId, OptimisticFetchRequest>>,
    lru_response_cache: Cache<StorageServiceRequest, StorageServiceResponse>,
    request_moderator: Arc<RequestModerator>,
    storage: T,
    subscriptions: Arc<DashMap<PeerNetworkId, SubscriptionStreamRequests>>,
    time_service: TimeService,
    peers_with_ready_optimistic_fetches: Vec<(PeerNetworkId, LedgerInfoWithSignatures)>,
) {
    for (peer_network_id, target_ledger_info) in peers_with_ready_optimistic_fetches {
        // Remove the optimistic fetch from the active map. Note: we only do this if
        // the known version is lower than the target version. This is because
        // the peer may have updated their highest known version since we last checked.
        let ready_optimistic_fetch =
            optimistic_fetches.remove_if(&peer_network_id, |_, optimistic_fetch| {
                optimistic_fetch.highest_known_version()
                    < target_ledger_info.ledger_info().version()
            });

        // Handle the optimistic fetch request
        if let Some((_, optimistic_fetch)) = ready_optimistic_fetch {
            // Clone all required components for the task
            let cached_storage_server_summary = cached_storage_server_summary.clone();
            let optimistic_fetches = optimistic_fetches.clone();
            let lru_response_cache = lru_response_cache.clone();
            let request_moderator = request_moderator.clone();
            let storage = storage.clone();
            let subscriptions = subscriptions.clone();
            let time_service = time_service.clone();

            // Spawn a blocking task to handle the optimistic fetch
            runtime.spawn_blocking(move || {
                // Get the fetch start time and request
                let optimistic_fetch_start_time = optimistic_fetch.fetch_start_time;
                let optimistic_fetch_request = optimistic_fetch.request.clone();

                // Handle the optimistic fetch request and time the operation
                let handle_request = || {
                    // Get the storage service request for the missing data
                    let missing_data_request = optimistic_fetch
                        .get_storage_request_for_missing_data(config, &target_ledger_info)?;

                    // Notify the peer of the new data
                    utils::notify_peer_of_new_data(
                        cached_storage_server_summary.clone(),
                        optimistic_fetches.clone(),
                        subscriptions.clone(),
                        lru_response_cache.clone(),
                        request_moderator.clone(),
                        storage.clone(),
                        time_service.clone(),
                        &peer_network_id,
                        missing_data_request,
                        target_ledger_info,
                        optimistic_fetch.take_response_sender(),
                    )
                };
                let result = utils::execute_and_time_duration(
                    &metrics::OPTIMISTIC_FETCH_LATENCIES,
                    Some((&peer_network_id, &optimistic_fetch_request)),
                    None,
                    handle_request,
                    Some(optimistic_fetch_start_time),
                );

                // Log an error if the handler failed
                if let Err(error) = result {
                    warn!(LogSchema::new(LogEntry::OptimisticFetchResponse)
                        .error(&Error::UnexpectedErrorEncountered(error.to_string())));
                }
            });
        }
    }
}

/// Identifies the optimistic fetches that can be handled now.
/// Returns the list of peers that made those optimistic fetches
/// alongside the ledger info at the target version for the peer.
pub(crate) async fn get_peers_with_ready_optimistic_fetches<T: StorageReaderInterface>(
    runtime: Handle,
    config: StorageServiceConfig,
    cached_storage_server_summary: Arc<ArcSwap<StorageServerSummary>>,
    optimistic_fetches: Arc<DashMap<PeerNetworkId, OptimisticFetchRequest>>,
    lru_response_cache: Cache<StorageServiceRequest, StorageServiceResponse>,
    request_moderator: Arc<RequestModerator>,
    storage: T,
    subscriptions: Arc<DashMap<PeerNetworkId, SubscriptionStreamRequests>>,
    time_service: TimeService,
) -> aptos_storage_service_types::Result<Vec<(PeerNetworkId, LedgerInfoWithSignatures)>, Error> {
    // Fetch the latest storage summary and highest synced version
    let latest_storage_summary = cached_storage_server_summary.load().clone();
    let highest_synced_ledger_info = match &latest_storage_summary.data_summary.synced_ledger_info {
        Some(ledger_info) => ledger_info.clone(),
        None => return Ok(vec![]),
    };

    // Identify the peers with expired, invalid and ready optimistic fetches
    let (
        peers_with_expired_optimistic_fetches,
        peers_with_invalid_optimistic_fetches,
        peers_with_ready_optimistic_fetches,
    ) = identify_expired_invalid_and_ready_fetches(
        runtime,
        config,
        cached_storage_server_summary,
        optimistic_fetches.clone(),
        subscriptions,
        lru_response_cache,
        request_moderator,
        storage,
        time_service,
        highest_synced_ledger_info,
    )
    .await;

    // Remove the expired optimistic fetches
    removed_expired_optimistic_fetches(
        optimistic_fetches.clone(),
        peers_with_expired_optimistic_fetches,
    );

    // Remove the invalid optimistic fetches
    remove_invalid_optimistic_fetches(optimistic_fetches, peers_with_invalid_optimistic_fetches);

    // Return the ready optimistic fetches
    Ok(peers_with_ready_optimistic_fetches)
}

/// Identifies the expired, invalid and ready optimistic fetches
/// from the active map. Returns each peer list separately.
async fn identify_expired_invalid_and_ready_fetches<T: StorageReaderInterface>(
    runtime: Handle,
    config: StorageServiceConfig,
    cached_storage_server_summary: Arc<ArcSwap<StorageServerSummary>>,
    optimistic_fetches: Arc<DashMap<PeerNetworkId, OptimisticFetchRequest>>,
    subscriptions: Arc<DashMap<PeerNetworkId, SubscriptionStreamRequests>>,
    lru_response_cache: Cache<StorageServiceRequest, StorageServiceResponse>,
    request_moderator: Arc<RequestModerator>,
    storage: T,
    time_service: TimeService,
    highest_synced_ledger_info: LedgerInfoWithSignatures,
) -> (
    Vec<PeerNetworkId>,
    Vec<PeerNetworkId>,
    Vec<(PeerNetworkId, LedgerInfoWithSignatures)>,
) {
    // Gather the highest synced version and epoch for each peer
    let mut peers_and_highest_synced_data = HashMap::new();
    let mut peers_with_expired_optimistic_fetches = vec![];
    for optimistic_fetch in optimistic_fetches.iter() {
        // Get the peer and the optimistic fetch request
        let peer_network_id = *optimistic_fetch.key();
        let optimistic_fetch = optimistic_fetch.value();

        // Gather the peer's highest synced version and epoch
        if !optimistic_fetch.is_expired(config.max_optimistic_fetch_period_ms) {
            let highest_known_version = optimistic_fetch.highest_known_version();
            let highest_known_epoch = optimistic_fetch.highest_known_epoch();

            // Save the peer's version and epoch
            peers_and_highest_synced_data.insert(
                peer_network_id,
                (highest_known_version, highest_known_epoch),
            );
        } else {
            // The request has expired -- there's nothing to do
            peers_with_expired_optimistic_fetches.push(peer_network_id);
        }
    }

    // Identify the peers with ready and invalid optimistic fetches
    let (peers_with_ready_optimistic_fetches, peers_with_invalid_optimistic_fetches) =
        identify_ready_and_invalid_optimistic_fetches(
            runtime,
            cached_storage_server_summary,
            optimistic_fetches,
            subscriptions,
            lru_response_cache,
            request_moderator,
            storage,
            time_service,
            highest_synced_ledger_info,
            peers_and_highest_synced_data,
        )
        .await;

    // Return all peer lists
    (
        peers_with_expired_optimistic_fetches,
        peers_with_invalid_optimistic_fetches,
        peers_with_ready_optimistic_fetches,
    )
}

/// Identifies the ready and invalid optimistic fetches from the given
/// map of peers and their highest synced versions and epochs.
async fn identify_ready_and_invalid_optimistic_fetches<T: StorageReaderInterface>(
    runtime: Handle,
    cached_storage_server_summary: Arc<ArcSwap<StorageServerSummary>>,
    optimistic_fetches: Arc<DashMap<PeerNetworkId, OptimisticFetchRequest>>,
    subscriptions: Arc<DashMap<PeerNetworkId, SubscriptionStreamRequests>>,
    lru_response_cache: Cache<StorageServiceRequest, StorageServiceResponse>,
    request_moderator: Arc<RequestModerator>,
    storage: T,
    time_service: TimeService,
    highest_synced_ledger_info: LedgerInfoWithSignatures,
    peers_and_highest_synced_data: HashMap<PeerNetworkId, (u64, u64)>,
) -> (
    Vec<(PeerNetworkId, LedgerInfoWithSignatures)>,
    Vec<PeerNetworkId>,
) {
    // Create the peer lists for ready and invalid optimistic fetches
    let peers_with_ready_optimistic_fetches = Arc::new(Mutex::new(vec![]));
    let peers_with_invalid_optimistic_fetches = Arc::new(Mutex::new(vec![]));

    // Identify the highest synced version and epoch
    let highest_synced_version = highest_synced_ledger_info.ledger_info().version();
    let highest_synced_epoch = highest_synced_ledger_info.ledger_info().epoch();

    // Go through all peers and highest synced data and identify the relevant entries
    let mut active_tasks = vec![];
    for (peer_network_id, (highest_known_version, highest_known_epoch)) in
        peers_and_highest_synced_data.into_iter()
    {
        // Clone all required components for the task
        let runtime = runtime.clone();
        let cached_storage_server_summary = cached_storage_server_summary.clone();
        let highest_synced_ledger_info = highest_synced_ledger_info.clone();
        let optimistic_fetches = optimistic_fetches.clone();
        let subscriptions = subscriptions.clone();
        let lru_response_cache = lru_response_cache.clone();
        let request_moderator = request_moderator.clone();
        let storage = storage.clone();
        let time_service = time_service.clone();
        let peers_with_invalid_optimistic_fetches = peers_with_invalid_optimistic_fetches.clone();
        let peers_with_ready_optimistic_fetches = peers_with_ready_optimistic_fetches.clone();

        // Spawn a blocking task to determine if the optimistic fetch is ready or
        // invalid. We do this because each entry may require reading from storage.
        let active_task = runtime.spawn_blocking(move || {
            // Check if we have synced beyond the highest known version
            if highest_known_version < highest_synced_version {
                if highest_known_epoch < highest_synced_epoch {
                    // Fetch the epoch ending ledger info from storage (the
                    // peer needs to sync to their epoch ending ledger info).
                    let epoch_ending_ledger_info = match utils::get_epoch_ending_ledger_info(
                        cached_storage_server_summary.clone(),
                        optimistic_fetches.clone(),
                        subscriptions.clone(),
                        highest_known_epoch,
                        lru_response_cache.clone(),
                        request_moderator.clone(),
                        &peer_network_id,
                        storage.clone(),
                        time_service.clone(),
                    ) {
                        Ok(epoch_ending_ledger_info) => epoch_ending_ledger_info,
                        Err(error) => {
                            // Log the failure to fetch the epoch ending ledger info
                            error!(LogSchema::new(LogEntry::OptimisticFetchRefresh)
                                .error(&error)
                                .message(&format!(
                                    "Failed to get the epoch ending ledger info for epoch: {:?} !",
                                    highest_known_epoch
                                )));

                            return;
                        },
                    };

                    // Check that we haven't been sent an invalid optimistic fetch request
                    // (i.e., a request that does not respect an epoch boundary).
                    if epoch_ending_ledger_info.ledger_info().version() <= highest_known_version {
                        peers_with_invalid_optimistic_fetches
                            .lock()
                            .push(peer_network_id);
                    } else {
                        peers_with_ready_optimistic_fetches
                            .lock()
                            .push((peer_network_id, epoch_ending_ledger_info));
                    }
                } else {
                    peers_with_ready_optimistic_fetches
                        .lock()
                        .push((peer_network_id, highest_synced_ledger_info.clone()));
                };
            }
        });

        // Add the task to the list of active tasks
        active_tasks.push(active_task);
    }

    // Wait for all the active tasks to complete
    join_all(active_tasks).await;

    // Gather the invalid and ready optimistic fetches
    let peers_with_invalid_optimistic_fetches =
        peers_with_invalid_optimistic_fetches.lock().deref().clone();
    let peers_with_ready_optimistic_fetches =
        peers_with_ready_optimistic_fetches.lock().deref().clone();

    (
        peers_with_ready_optimistic_fetches,
        peers_with_invalid_optimistic_fetches,
    )
}

/// Removes the expired optimistic fetches from the active map
fn removed_expired_optimistic_fetches(
    optimistic_fetches: Arc<DashMap<PeerNetworkId, OptimisticFetchRequest>>,
    peers_with_expired_optimistic_fetches: Vec<PeerNetworkId>,
) {
    for peer_network_id in peers_with_expired_optimistic_fetches {
        if optimistic_fetches.remove(&peer_network_id).is_some() {
            increment_counter(
                &metrics::OPTIMISTIC_FETCH_EVENTS,
                peer_network_id.network_id(),
                OPTIMISTIC_FETCH_EXPIRE.into(),
            );
        }
    }
}

/// Removes the invalid optimistic fetches from the active map
fn remove_invalid_optimistic_fetches(
    optimistic_fetches: Arc<DashMap<PeerNetworkId, OptimisticFetchRequest>>,
    peers_with_invalid_optimistic_fetches: Vec<PeerNetworkId>,
) {
    for peer_network_id in peers_with_invalid_optimistic_fetches {
        if let Some((peer_network_id, optimistic_fetch)) =
            optimistic_fetches.remove(&peer_network_id)
        {
            warn!(LogSchema::new(LogEntry::OptimisticFetchRefresh)
                .error(&Error::InvalidRequest(
                    "Mismatch between known version and epoch!".into()
                ))
                .request(&optimistic_fetch.request)
                .message(&format!(
                    "Dropping invalid optimistic fetch request for peer: {:?}!",
                    peer_network_id
                )));
        }
    }
}

/// Updates the active optimistic fetch metrics for each network
fn update_optimistic_fetch_metrics(
    optimistic_fetches: Arc<DashMap<PeerNetworkId, OptimisticFetchRequest>>,
) {
    // Calculate the total number of optimistic fetches for each network
    let mut num_validator_optimistic_fetches = 0;
    let mut num_vfn_optimistic_fetches = 0;
    let mut num_public_optimistic_fetches = 0;
    for optimistic_fetch in optimistic_fetches.iter() {
        // Get the peer network ID
        let peer_network_id = optimistic_fetch.key();

        // Increment the number of optimistic fetches for the peer's network
        match peer_network_id.network_id() {
            NetworkId::Validator => num_validator_optimistic_fetches += 1,
            NetworkId::Vfn => num_vfn_optimistic_fetches += 1,
            NetworkId::Public => num_public_optimistic_fetches += 1,
        }
    }

    // Update the number of active optimistic fetches for each network
    metrics::set_gauge(
        &metrics::OPTIMISTIC_FETCH_COUNT,
        NetworkId::Validator.as_str(),
        num_validator_optimistic_fetches as u64,
    );
    metrics::set_gauge(
        &metrics::OPTIMISTIC_FETCH_COUNT,
        NetworkId::Vfn.as_str(),
        num_vfn_optimistic_fetches as u64,
    );
    metrics::set_gauge(
        &metrics::OPTIMISTIC_FETCH_COUNT,
        NetworkId::Public.as_str(),
        num_public_optimistic_fetches as u64,
    );
}
