// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::Error,
    metrics,
    metrics::{increment_counter, SUBSCRIPTION_EXPIRE},
    moderator::RequestModerator,
    network::ResponseSender,
    optimistic_fetch::OptimisticFetchRequest,
    storage::StorageReaderInterface,
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
        SubscriptionStreamMetadata, TransactionDataRequestType, TransactionOutputsWithProofRequest,
        TransactionsOrOutputsWithProofRequest, TransactionsWithProofRequest,
    },
    responses::{
        DataResponse, StorageServerSummary, StorageServiceResponse, TransactionDataResponseType,
    },
};
use aptos_time_service::{TimeService, TimeServiceTrait};
use aptos_types::{ledger_info::LedgerInfoWithSignatures, transaction::Version};
use arc_swap::ArcSwap;
use dashmap::DashMap;
use futures::future::join_all;
use mini_moka::sync::Cache;
use std::{
    cmp::min,
    collections::{BTreeMap, HashMap},
    fmt::Debug,
    ops::Deref,
    sync::Arc,
    time::Instant,
};
use tokio::runtime::Handle;

/// A single subscription request that is part of a stream
pub struct SubscriptionRequest {
    request: StorageServiceRequest,  // The original request
    response_sender: ResponseSender, // The sender along which to send the response
    request_start_time: Instant,     // The time the request started (i.e., when it was received)
}

impl SubscriptionRequest {
    pub fn new(
        request: StorageServiceRequest,
        response_sender: ResponseSender,
        time_service: TimeService,
    ) -> Self {
        Self {
            request,
            response_sender,
            request_start_time: time_service.now(),
        }
    }

    /// Creates a new storage service request to satisfy the request
    /// using the new data at the specified `target_ledger_info`.
    fn get_storage_request_for_missing_data(
        &self,
        config: StorageServiceConfig,
        known_version: u64,
        target_ledger_info: &LedgerInfoWithSignatures,
    ) -> aptos_storage_service_types::Result<StorageServiceRequest, Error> {
        // Calculate the number of versions to fetch
        let target_version = target_ledger_info.ledger_info().version();
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
            DataRequest::SubscribeTransactionOutputsWithProof(_) => {
                DataRequest::GetTransactionOutputsWithProof(TransactionOutputsWithProofRequest {
                    proof_version: target_version,
                    start_version,
                    end_version,
                })
            },
            DataRequest::SubscribeTransactionsWithProof(request) => {
                DataRequest::GetTransactionsWithProof(TransactionsWithProofRequest {
                    proof_version: target_version,
                    start_version,
                    end_version,
                    include_events: request.include_events,
                })
            },
            DataRequest::SubscribeTransactionsOrOutputsWithProof(request) => {
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
            DataRequest::SubscribeTransactionDataWithProof(request) => {
                DataRequest::GetTransactionDataWithProof(GetTransactionDataWithProofRequest {
                    transaction_data_request_type: request.transaction_data_request_type,
                    proof_version: target_version,
                    start_version,
                    end_version,
                    max_response_bytes: request.max_response_bytes,
                })
            },
            request => unreachable!("Unexpected subscription request: {:?}", request),
        };
        let storage_request =
            StorageServiceRequest::new(data_request, self.request.use_compression);
        Ok(storage_request)
    }

    /// Returns the highest version known by the peer when the stream started
    fn highest_known_version_at_stream_start(&self) -> u64 {
        match &self.request.data_request {
            DataRequest::SubscribeTransactionOutputsWithProof(request) => {
                request
                    .subscription_stream_metadata
                    .known_version_at_stream_start
            },
            DataRequest::SubscribeTransactionsWithProof(request) => {
                request
                    .subscription_stream_metadata
                    .known_version_at_stream_start
            },
            DataRequest::SubscribeTransactionsOrOutputsWithProof(request) => {
                request
                    .subscription_stream_metadata
                    .known_version_at_stream_start
            },
            DataRequest::SubscribeTransactionDataWithProof(request) => {
                request
                    .subscription_stream_metadata
                    .known_version_at_stream_start
            },
            request => unreachable!("Unexpected subscription request: {:?}", request),
        }
    }

    /// Returns the highest epoch known by the peer when the stream started
    fn highest_known_epoch_at_stream_start(&self) -> u64 {
        match &self.request.data_request {
            DataRequest::SubscribeTransactionOutputsWithProof(request) => {
                request
                    .subscription_stream_metadata
                    .known_epoch_at_stream_start
            },
            DataRequest::SubscribeTransactionsWithProof(request) => {
                request
                    .subscription_stream_metadata
                    .known_epoch_at_stream_start
            },
            DataRequest::SubscribeTransactionsOrOutputsWithProof(request) => {
                request
                    .subscription_stream_metadata
                    .known_epoch_at_stream_start
            },
            DataRequest::SubscribeTransactionDataWithProof(request) => {
                request
                    .subscription_stream_metadata
                    .known_epoch_at_stream_start
            },
            request => unreachable!("Unexpected subscription request: {:?}", request),
        }
    }

    /// Returns the maximum chunk size for the request
    /// depending on the request type.
    fn max_chunk_size_for_request(&self, config: StorageServiceConfig) -> u64 {
        match &self.request.data_request {
            DataRequest::SubscribeTransactionOutputsWithProof(_) => {
                config.max_transaction_output_chunk_size
            },
            DataRequest::SubscribeTransactionsWithProof(_) => config.max_transaction_chunk_size,
            DataRequest::SubscribeTransactionsOrOutputsWithProof(_) => {
                config.max_transaction_output_chunk_size
            },
            DataRequest::SubscribeTransactionDataWithProof(request) => {
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
            request => unreachable!("Unexpected subscription request: {:?}", request),
        }
    }

    /// Returns the subscription stream id for the request
    pub fn subscription_stream_id(&self) -> u64 {
        match &self.request.data_request {
            DataRequest::SubscribeTransactionOutputsWithProof(request) => {
                request.subscription_stream_metadata.subscription_stream_id
            },
            DataRequest::SubscribeTransactionsWithProof(request) => {
                request.subscription_stream_metadata.subscription_stream_id
            },
            DataRequest::SubscribeTransactionsOrOutputsWithProof(request) => {
                request.subscription_stream_metadata.subscription_stream_id
            },
            DataRequest::SubscribeTransactionDataWithProof(request) => {
                request.subscription_stream_metadata.subscription_stream_id
            },
            request => unreachable!("Unexpected subscription request: {:?}", request),
        }
    }

    /// Returns the subscription stream index for the request
    fn subscription_stream_index(&self) -> u64 {
        match &self.request.data_request {
            DataRequest::SubscribeTransactionOutputsWithProof(request) => {
                request.subscription_stream_index
            },
            DataRequest::SubscribeTransactionsWithProof(request) => {
                request.subscription_stream_index
            },
            DataRequest::SubscribeTransactionsOrOutputsWithProof(request) => {
                request.subscription_stream_index
            },
            DataRequest::SubscribeTransactionDataWithProof(request) => {
                request.subscription_stream_index
            },
            request => unreachable!("Unexpected subscription request: {:?}", request),
        }
    }

    /// Returns the subscription stream metadata for the request
    fn subscription_stream_metadata(&self) -> SubscriptionStreamMetadata {
        match &self.request.data_request {
            DataRequest::SubscribeTransactionOutputsWithProof(request) => {
                request.subscription_stream_metadata
            },
            DataRequest::SubscribeTransactionsWithProof(request) => {
                request.subscription_stream_metadata
            },
            DataRequest::SubscribeTransactionsOrOutputsWithProof(request) => {
                request.subscription_stream_metadata
            },
            DataRequest::SubscribeTransactionDataWithProof(request) => {
                request.subscription_stream_metadata
            },
            request => unreachable!("Unexpected subscription request: {:?}", request),
        }
    }

    /// Returns the response sender and consumes the request
    pub fn take_response_sender(self) -> ResponseSender {
        self.response_sender
    }
}

impl Debug for SubscriptionRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SubscriptionRequest: {{ request_start_time: {:?}, request: {:?} }}",
            self.request_start_time, self.request
        )
    }
}

/// A set of subscription requests that together form a stream
#[derive(Debug)]
pub struct SubscriptionStreamRequests {
    subscription_stream_metadata: SubscriptionStreamMetadata, // The metadata for the subscription stream (as specified by the client)

    highest_known_version: u64, // The highest version known by the peer (at this point in the stream)
    highest_known_epoch: u64,   // The highest epoch known by the peer (at this point in the stream)

    next_index_to_serve: u64, // The next subscription stream request index to serve
    pending_subscription_requests: BTreeMap<u64, SubscriptionRequest>, // The pending subscription requests by stream index

    last_stream_update_time: Instant, // The last time the stream was updated
    time_service: TimeService,        // The time service
}

impl SubscriptionStreamRequests {
    pub fn new(subscription_request: SubscriptionRequest, time_service: TimeService) -> Self {
        // Extract the relevant information from the request
        let highest_known_version = subscription_request.highest_known_version_at_stream_start();
        let highest_known_epoch = subscription_request.highest_known_epoch_at_stream_start();
        let subscription_stream_metadata = subscription_request.subscription_stream_metadata();

        // Create a new set of pending subscription requests using the first request
        let mut pending_subscription_requests = BTreeMap::new();
        pending_subscription_requests.insert(
            subscription_request.subscription_stream_index(),
            subscription_request,
        );

        Self {
            highest_known_version,
            highest_known_epoch,
            next_index_to_serve: 0,
            pending_subscription_requests,
            subscription_stream_metadata,
            last_stream_update_time: time_service.now(),
            time_service,
        }
    }

    /// Adds a subscription request to the existing stream. If this operation
    /// fails, the request is returned to the caller so that the client
    /// can be notified of the error.
    pub fn add_subscription_request(
        &mut self,
        storage_service_config: StorageServiceConfig,
        subscription_request: SubscriptionRequest,
    ) -> Result<(), (Error, SubscriptionRequest)> {
        // Verify that the subscription metadata is valid
        let subscription_stream_metadata = subscription_request.subscription_stream_metadata();
        if subscription_stream_metadata != self.subscription_stream_metadata {
            return Err((
                Error::InvalidRequest(format!(
                    "The subscription request stream metadata is invalid! Expected: {:?}, found: {:?}",
                    self.subscription_stream_metadata, subscription_stream_metadata
                )),
                subscription_request,
            ));
        }

        // Verify that the subscription request index is valid
        let subscription_request_index = subscription_request.subscription_stream_index();
        if subscription_request_index < self.next_index_to_serve {
            return Err((
                Error::InvalidRequest(format!(
                    "The subscription request index is too low! Next index to serve: {:?}, found: {:?}",
                    self.next_index_to_serve, subscription_request_index
                )),
                subscription_request,
            ));
        }

        // Verify that the number of active subscriptions respects the maximum
        let max_num_active_subscriptions =
            storage_service_config.max_num_active_subscriptions as usize;
        if self.pending_subscription_requests.len() >= max_num_active_subscriptions {
            return Err((
                Error::InvalidRequest(format!(
                    "The maximum number of active subscriptions has been reached! Max: {:?}, found: {:?}",
                    max_num_active_subscriptions, self.pending_subscription_requests.len()
                )),
                subscription_request,
            ));
        }

        // Insert the subscription request into the pending requests
        let existing_request = self.pending_subscription_requests.insert(
            subscription_request.subscription_stream_index(),
            subscription_request,
        );

        // Refresh the last stream update time
        self.refresh_last_stream_update_time();

        // If a pending request already existed, return the previous request to the caller
        if let Some(existing_request) = existing_request {
            return Err((
                Error::InvalidRequest(format!(
                    "Overwriting an existing subscription request for the given index: {:?}",
                    subscription_request_index
                )),
                existing_request,
            ));
        }

        Ok(())
    }

    /// Returns a reference to the first pending subscription request
    /// in the stream (if it exists).
    pub fn first_pending_request(&self) -> Option<&SubscriptionRequest> {
        self.pending_subscription_requests
            .first_key_value()
            .map(|(_, request)| request)
    }

    /// Returns true iff the subscription stream has expired.
    /// There are two ways a stream can expire: (i) the first
    /// pending request has been blocked for too long; or (ii)
    /// the stream has been idle for too long.
    fn is_expired(&self, timeout_ms: u64) -> bool {
        // Determine the time when the stream was first blocked
        let time_when_first_blocked =
            if let Some(subscription_request) = self.first_pending_request() {
                subscription_request.request_start_time // The stream is blocked on the first pending request
            } else {
                self.last_stream_update_time // The stream is idle and hasn't been updated in a while
            };

        // Verify the stream hasn't been blocked for too long
        let current_time = self.time_service.now();
        let elapsed_time = current_time
            .duration_since(time_when_first_blocked)
            .as_millis();
        elapsed_time > (timeout_ms as u128)
    }

    /// Returns true iff there is at least one pending request
    /// and that request is ready to be served (i.e., it has the
    /// same index as the next index to serve).
    fn first_request_ready_to_be_served(&self) -> bool {
        if let Some(subscription_request) = self.first_pending_request() {
            subscription_request.subscription_stream_index() == self.next_index_to_serve
        } else {
            false
        }
    }

    /// Removes the first pending subscription request from the stream
    /// and returns it (if it exists).
    fn pop_first_pending_request(&mut self) -> Option<SubscriptionRequest> {
        self.pending_subscription_requests
            .pop_first()
            .map(|(_, request)| request)
    }

    /// Refreshes the last stream update time to the current time
    fn refresh_last_stream_update_time(&mut self) {
        self.last_stream_update_time = self.time_service.now();
    }

    /// Returns the unique stream id for the stream
    pub fn subscription_stream_id(&self) -> u64 {
        self.subscription_stream_metadata.subscription_stream_id
    }

    /// Updates the highest known version and epoch for the stream
    /// using the latest data response that was sent to the client.
    fn update_known_version_and_epoch(
        &mut self,
        data_response: &DataResponse,
    ) -> Result<(), Error> {
        // Determine the number of data items and target ledger info sent to the client
        let (num_data_items, target_ledger_info) = match data_response {
            DataResponse::NewTransactionOutputsWithProof((
                transaction_output_list,
                target_ledger_info,
            )) => (
                transaction_output_list.transactions_and_outputs.len(),
                target_ledger_info,
            ),
            DataResponse::NewTransactionsWithProof((transaction_list, target_ledger_info)) => {
                (transaction_list.transactions.len(), target_ledger_info)
            },
            DataResponse::NewTransactionsOrOutputsWithProof((
                (transaction_list, transaction_output_list),
                target_ledger_info,
            )) => {
                if let Some(transaction_list) = transaction_list {
                    (transaction_list.transactions.len(), target_ledger_info)
                } else if let Some(transaction_output_list) = transaction_output_list {
                    (
                        transaction_output_list.transactions_and_outputs.len(),
                        target_ledger_info,
                    )
                } else {
                    return Err(Error::UnexpectedErrorEncountered(format!(
                        "New transactions or outputs response is missing data: {:?}",
                        data_response
                    )));
                }
            },
            DataResponse::NewTransactionDataWithProof(response) => {
                let num_data_items = match response.transaction_data_response_type {
                    TransactionDataResponseType::TransactionData => {
                        if let Some(transaction_list_with_proof_v2) =
                            &response.transaction_list_with_proof
                        {
                            transaction_list_with_proof_v2
                                .get_transaction_list_with_proof()
                                .transactions
                                .len()
                        } else {
                            return Err(Error::UnexpectedErrorEncountered(format!(
                                "Transaction data response is missing transaction list: {:?}",
                                data_response
                            )));
                        }
                    },
                    TransactionDataResponseType::TransactionOutputData => {
                        if let Some(output_list_with_proof_v2) =
                            &response.transaction_output_list_with_proof
                        {
                            output_list_with_proof_v2
                                .get_output_list_with_proof()
                                .transactions_and_outputs
                                .len()
                        } else {
                            return Err(Error::UnexpectedErrorEncountered(format!(
                                "Transaction output data response is missing output list: {:?}",
                                data_response
                            )));
                        }
                    },
                };
                let target_ledger_info = &response.ledger_info_with_signatures;

                (num_data_items, target_ledger_info)
            },
            _ => {
                return Err(Error::UnexpectedErrorEncountered(format!(
                    "Unexpected data response type: {:?}",
                    data_response
                )))
            },
        };

        // Update the highest known version
        self.highest_known_version += num_data_items as u64;

        // Update the highest known epoch if we've now hit an epoch ending ledger info
        if self.highest_known_version == target_ledger_info.ledger_info().version()
            && target_ledger_info.ledger_info().ends_epoch()
        {
            self.highest_known_epoch += 1;
        }

        // Update the next index to serve
        self.next_index_to_serve += 1;

        // Refresh the last stream update time
        self.refresh_last_stream_update_time();

        Ok(())
    }

    #[cfg(test)]
    /// Returns the highest known version and epoch for test purposes
    pub fn get_highest_known_version_and_epoch(&self) -> (u64, u64) {
        (self.highest_known_version, self.highest_known_epoch)
    }

    #[cfg(test)]
    /// Returns the next index to serve for test purposes
    pub fn get_next_index_to_serve(&self) -> u64 {
        self.next_index_to_serve
    }

    #[cfg(test)]
    /// Returns the pending subscription requests for test purposes
    pub fn get_pending_subscription_requests(&mut self) -> &mut BTreeMap<u64, SubscriptionRequest> {
        &mut self.pending_subscription_requests
    }

    #[cfg(test)]
    /// Sets the next index to serve for test purposes
    pub fn set_next_index_to_serve(&mut self, next_index_to_serve: u64) {
        self.next_index_to_serve = next_index_to_serve;
    }
}

/// Handles active and ready subscriptions
pub(crate) async fn handle_active_subscriptions<T: StorageReaderInterface>(
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
    // Continuously handle the subscriptions until we identify that
    // there are no more subscriptions ready to be served now.
    loop {
        // Update the number of active subscriptions
        update_active_subscription_metrics(subscriptions.clone());

        // Identify the peers with ready subscriptions
        let peers_with_ready_subscriptions = get_peers_with_ready_subscriptions(
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

        // If there are no peers with ready subscriptions, we're finished
        if peers_with_ready_subscriptions.is_empty() {
            return Ok(());
        }

        // Remove and handle the ready subscriptions
        handle_ready_subscriptions(
            runtime.clone(),
            cached_storage_server_summary.clone(),
            config,
            optimistic_fetches.clone(),
            lru_response_cache.clone(),
            request_moderator.clone(),
            storage.clone(),
            subscriptions.clone(),
            time_service.clone(),
            peers_with_ready_subscriptions,
        )
        .await;
    }
}

/// Handles the ready subscriptions by removing them from the
/// active map and notifying the peer of the new data.
async fn handle_ready_subscriptions<T: StorageReaderInterface>(
    runtime: Handle,
    cached_storage_server_summary: Arc<ArcSwap<StorageServerSummary>>,
    config: StorageServiceConfig,
    optimistic_fetches: Arc<DashMap<PeerNetworkId, OptimisticFetchRequest>>,
    lru_response_cache: Cache<StorageServiceRequest, StorageServiceResponse>,
    request_moderator: Arc<RequestModerator>,
    storage: T,
    subscriptions: Arc<DashMap<PeerNetworkId, SubscriptionStreamRequests>>,
    time_service: TimeService,
    peers_with_ready_subscriptions: Vec<(PeerNetworkId, LedgerInfoWithSignatures)>,
) {
    // Go through all peers with ready subscriptions
    let mut active_tasks = vec![];
    for (peer_network_id, target_ledger_info) in peers_with_ready_subscriptions {
        // Remove the subscription from the active subscription stream
        let subscription_request_and_known_version =
            subscriptions
                .get_mut(&peer_network_id)
                .map(|mut subscription_stream_requests| {
                    (
                        subscription_stream_requests.pop_first_pending_request(),
                        subscription_stream_requests.highest_known_version,
                    )
                });

        // Handle the subscription
        if let Some((Some(subscription_request), known_version)) =
            subscription_request_and_known_version
        {
            // Clone all required components for the task
            let cached_storage_server_summary = cached_storage_server_summary.clone();
            let optimistic_fetches = optimistic_fetches.clone();
            let lru_response_cache = lru_response_cache.clone();
            let request_moderator = request_moderator.clone();
            let storage = storage.clone();
            let subscriptions = subscriptions.clone();
            let time_service = time_service.clone();

            // Spawn a blocking task to handle the subscription
            let active_task = runtime.spawn_blocking(move || {
                // Get the subscription start time and request
                let subscription_start_time = subscription_request.request_start_time;
                let subscription_data_request = subscription_request.request.clone();

                // Handle the subscription request and time the operation
                let handle_request = || {
                    // Get the storage service request for the missing data
                    let missing_data_request = subscription_request
                        .get_storage_request_for_missing_data(
                            config,
                            known_version,
                            &target_ledger_info,
                        )?;

                    // Notify the peer of the new data
                    let data_response = utils::notify_peer_of_new_data(
                        cached_storage_server_summary,
                        optimistic_fetches,
                        subscriptions.clone(),
                        lru_response_cache,
                        request_moderator,
                        storage,
                        time_service.clone(),
                        &peer_network_id,
                        missing_data_request,
                        target_ledger_info,
                        subscription_request.take_response_sender(),
                    )?;

                    // Update the stream's known version and epoch
                    if let Some(mut subscription_stream_requests) =
                        subscriptions.get_mut(&peer_network_id)
                    {
                        subscription_stream_requests
                            .update_known_version_and_epoch(&data_response)?;
                    }

                    Ok(())
                };
                let result = utils::execute_and_time_duration(
                    &metrics::SUBSCRIPTION_LATENCIES,
                    Some((&peer_network_id, &subscription_data_request)),
                    None,
                    handle_request,
                    Some(subscription_start_time),
                );

                // Log an error if the handler failed
                if let Err(error) = result {
                    warn!(LogSchema::new(LogEntry::SubscriptionResponse)
                        .error(&Error::UnexpectedErrorEncountered(error.to_string())));
                }
            });

            // Add the task to the list of active tasks
            active_tasks.push(active_task);
        }
    }

    // Wait for all the active tasks to complete
    join_all(active_tasks).await;
}

/// Identifies the subscriptions that can be handled now.
/// Returns the list of peers that made those subscriptions
/// alongside the ledger info at the target version for the peer.
pub(crate) async fn get_peers_with_ready_subscriptions<T: StorageReaderInterface>(
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
    let highest_synced_version = highest_synced_ledger_info.ledger_info().version();
    let highest_synced_epoch = highest_synced_ledger_info.ledger_info().epoch();

    // Identify the peers with expired, invalid and ready subscriptions
    let (
        peers_with_expired_subscriptions,
        peers_with_invalid_subscriptions,
        peers_with_ready_subscriptions,
    ) = identify_expired_invalid_and_ready_subscriptions(
        runtime.clone(),
        config,
        cached_storage_server_summary.clone(),
        optimistic_fetches.clone(),
        subscriptions.clone(),
        lru_response_cache.clone(),
        request_moderator.clone(),
        storage.clone(),
        time_service.clone(),
        highest_synced_ledger_info,
        highest_synced_version,
        highest_synced_epoch,
    )
    .await;

    // Remove the expired subscriptions
    remove_expired_subscriptions(subscriptions.clone(), peers_with_expired_subscriptions);

    // Remove the invalid subscriptions
    remove_invalid_subscriptions(subscriptions.clone(), peers_with_invalid_subscriptions);

    // Return the ready subscriptions
    Ok(peers_with_ready_subscriptions)
}

/// Identifies the expired, invalid and ready subscriptions
/// from the active map. Returns each peer list separately.
async fn identify_expired_invalid_and_ready_subscriptions<T: StorageReaderInterface>(
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
    highest_synced_version: Version,
    highest_synced_epoch: u64,
) -> (
    Vec<PeerNetworkId>,
    Vec<PeerNetworkId>,
    Vec<(PeerNetworkId, LedgerInfoWithSignatures)>,
) {
    // Gather the highest synced version and epoch for each peer
    // that has an active subscription ready to be served.
    let mut peers_and_highest_synced_data = HashMap::new();
    let mut peers_with_expired_subscriptions = vec![];
    for subscription in subscriptions.iter() {
        // Get the peer and the subscription stream requests
        let peer_network_id = *subscription.key();
        let subscription_stream_requests = subscription.value();

        // Gather the peer's highest synced version and epoch
        if !subscription_stream_requests.is_expired(config.max_subscription_period_ms) {
            // Ensure that the first request is ready to be served
            if subscription_stream_requests.first_request_ready_to_be_served() {
                let highest_known_version = subscription_stream_requests.highest_known_version;
                let highest_known_epoch = subscription_stream_requests.highest_known_epoch;

                // Save the peer's version and epoch
                peers_and_highest_synced_data.insert(
                    peer_network_id,
                    (highest_known_version, highest_known_epoch),
                );
            }
        } else {
            // The request has expired -- there's nothing to do
            peers_with_expired_subscriptions.push(peer_network_id);
        }
    }

    // Identify the peers with ready and invalid subscriptions
    let (peers_with_ready_subscriptions, peers_with_invalid_subscriptions) =
        identify_ready_and_invalid_subscriptions(
            runtime,
            cached_storage_server_summary,
            optimistic_fetches,
            subscriptions,
            lru_response_cache,
            request_moderator,
            storage,
            time_service,
            highest_synced_ledger_info,
            highest_synced_version,
            highest_synced_epoch,
            peers_and_highest_synced_data,
        )
        .await;

    // Return all peer lists
    (
        peers_with_expired_subscriptions,
        peers_with_invalid_subscriptions,
        peers_with_ready_subscriptions,
    )
}

/// Identifies the ready and invalid subscriptions from the given
/// map of peers and their highest synced versions and epochs.
async fn identify_ready_and_invalid_subscriptions<T: StorageReaderInterface>(
    runtime: Handle,
    cached_storage_server_summary: Arc<ArcSwap<StorageServerSummary>>,
    optimistic_fetches: Arc<DashMap<PeerNetworkId, OptimisticFetchRequest>>,
    subscriptions: Arc<DashMap<PeerNetworkId, SubscriptionStreamRequests>>,
    lru_response_cache: Cache<StorageServiceRequest, StorageServiceResponse>,
    request_moderator: Arc<RequestModerator>,
    storage: T,
    time_service: TimeService,
    highest_synced_ledger_info: LedgerInfoWithSignatures,
    highest_synced_version: Version,
    highest_synced_epoch: u64,
    peers_and_highest_synced_data: HashMap<PeerNetworkId, (u64, u64)>,
) -> (
    Vec<(PeerNetworkId, LedgerInfoWithSignatures)>,
    Vec<PeerNetworkId>,
) {
    // Create the peer lists for ready and invalid subscriptions
    let peers_with_ready_subscriptions = Arc::new(Mutex::new(vec![]));
    let peers_with_invalid_subscriptions = Arc::new(Mutex::new(vec![]));

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
        let peers_with_invalid_subscriptions = peers_with_invalid_subscriptions.clone();
        let peers_with_ready_subscriptions = peers_with_ready_subscriptions.clone();

        // Spawn a blocking task to determine if the subscription is ready or
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
                            error!(LogSchema::new(LogEntry::SubscriptionRefresh)
                                .error(&error)
                                .message(&format!(
                                    "Failed to get the epoch ending ledger info for epoch: {:?} !",
                                    highest_known_epoch
                                )));

                            return;
                        },
                    };

                    // Check that we haven't been sent an invalid subscription request
                    // (i.e., a request that does not respect an epoch boundary).
                    if epoch_ending_ledger_info.ledger_info().version() <= highest_known_version {
                        peers_with_invalid_subscriptions
                            .lock()
                            .push(peer_network_id);
                    } else {
                        peers_with_ready_subscriptions
                            .lock()
                            .push((peer_network_id, epoch_ending_ledger_info));
                    }
                } else {
                    peers_with_ready_subscriptions
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

    // Gather the invalid and ready subscriptions
    let peers_with_invalid_subscriptions = peers_with_invalid_subscriptions.lock().deref().clone();
    let peers_with_ready_subscriptions = peers_with_ready_subscriptions.lock().deref().clone();

    (
        peers_with_ready_subscriptions,
        peers_with_invalid_subscriptions,
    )
}

/// Removes the expired subscription streams from the active map
fn remove_expired_subscriptions(
    subscriptions: Arc<DashMap<PeerNetworkId, SubscriptionStreamRequests>>,
    peers_with_expired_subscriptions: Vec<PeerNetworkId>,
) {
    for peer_network_id in peers_with_expired_subscriptions {
        if subscriptions.remove(&peer_network_id).is_some() {
            increment_counter(
                &metrics::SUBSCRIPTION_EVENTS,
                peer_network_id.network_id(),
                SUBSCRIPTION_EXPIRE.into(),
            );
        }
    }
}

/// Removes the invalid subscription streams from the active map
fn remove_invalid_subscriptions(
    subscriptions: Arc<DashMap<PeerNetworkId, SubscriptionStreamRequests>>,
    peers_with_invalid_subscriptions: Vec<PeerNetworkId>,
) {
    for peer_network_id in peers_with_invalid_subscriptions {
        if let Some((peer_network_id, subscription_stream_requests)) =
            subscriptions.remove(&peer_network_id)
        {
            warn!(LogSchema::new(LogEntry::SubscriptionRefresh)
                .error(&Error::InvalidRequest(
                    "Mismatch between known version and epoch!".into()
                ))
                .message(&format!(
                    "Dropping invalid subscription stream with ID: {:?}, for peer: {:?}!",
                    subscription_stream_requests.subscription_stream_id(),
                    peer_network_id
                )));
        }
    }
}

/// Updates the active subscription metrics for each network
fn update_active_subscription_metrics(
    subscriptions: Arc<DashMap<PeerNetworkId, SubscriptionStreamRequests>>,
) {
    // Calculate the total number of subscriptions for each network
    let mut num_validator_subscriptions = 0;
    let mut num_vfn_subscriptions = 0;
    let mut num_public_subscriptions = 0;
    for subscription in subscriptions.iter() {
        // Get the peer network ID
        let peer_network_id = *subscription.key();

        // Increment the number of subscriptions for the peer's network
        match peer_network_id.network_id() {
            NetworkId::Validator => num_validator_subscriptions += 1,
            NetworkId::Vfn => num_vfn_subscriptions += 1,
            NetworkId::Public => num_public_subscriptions += 1,
        }
    }

    // Update the number of active subscriptions for each network
    metrics::set_gauge(
        &metrics::SUBSCRIPTION_COUNT,
        NetworkId::Validator.as_str(),
        num_validator_subscriptions as u64,
    );
    metrics::set_gauge(
        &metrics::SUBSCRIPTION_COUNT,
        NetworkId::Vfn.as_str(),
        num_vfn_subscriptions as u64,
    );
    metrics::set_gauge(
        &metrics::SUBSCRIPTION_COUNT,
        NetworkId::Public.as_str(),
        num_public_subscriptions as u64,
    );
}
