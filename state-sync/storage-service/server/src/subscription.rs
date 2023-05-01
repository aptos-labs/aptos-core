// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::Error,
    handler::Handler,
    metrics,
    metrics::{increment_counter, increment_network_frame_overflow, SUBSCRIPTION_EVENT_EXPIRE},
    network::ResponseSender,
    storage::StorageReaderInterface,
    LogEntry, LogSchema,
};
use aptos_config::{config::StorageServiceConfig, network_id::PeerNetworkId};
use aptos_infallible::{Mutex, RwLock};
use aptos_logger::{debug, error, warn};
use aptos_network::ProtocolId;
use aptos_storage_service_types::{
    requests::{
        DataRequest, EpochEndingLedgerInfoRequest, StorageServiceRequest,
        TransactionOutputsWithProofRequest, TransactionsOrOutputsWithProofRequest,
        TransactionsWithProofRequest,
    },
    responses::{DataResponse, StorageServerSummary, StorageServiceResponse},
};
use aptos_time_service::{TimeService, TimeServiceTrait};
use aptos_types::ledger_info::LedgerInfoWithSignatures;
use lru::LruCache;
use std::{cmp::min, collections::HashMap, sync::Arc, time::Instant};

/// A subscription for data received by a client
pub struct DataSubscriptionRequest {
    protocol: ProtocolId,
    request: StorageServiceRequest,
    response_sender: ResponseSender,
    subscription_start_time: Instant,
    time_service: TimeService,
}

impl DataSubscriptionRequest {
    pub fn new(
        protocol: ProtocolId,
        request: StorageServiceRequest,
        response_sender: ResponseSender,
        time_service: TimeService,
    ) -> Self {
        Self {
            protocol,
            request,
            response_sender,
            subscription_start_time: time_service.now(),
            time_service,
        }
    }

    /// Creates a new storage service request to satisfy the subscription
    /// using the new data at the specified `target_ledger_info`.
    fn get_storage_request_for_missing_data(
        &self,
        config: StorageServiceConfig,
        target_ledger_info: &LedgerInfoWithSignatures,
    ) -> aptos_storage_service_types::Result<StorageServiceRequest, Error> {
        // Calculate the number of versions to fetch
        let known_version = self.highest_known_version();
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
            request => unreachable!("Unexpected subscription request: {:?}", request),
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
            request => unreachable!("Unexpected subscription request: {:?}", request),
        }
    }

    /// Returns the highest epoch known by the peer
    fn highest_known_epoch(&self) -> u64 {
        match &self.request.data_request {
            DataRequest::GetNewTransactionOutputsWithProof(request) => request.known_epoch,
            DataRequest::GetNewTransactionsWithProof(request) => request.known_epoch,
            DataRequest::GetNewTransactionsOrOutputsWithProof(request) => request.known_epoch,
            request => unreachable!("Unexpected subscription request: {:?}", request),
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
            request => unreachable!("Unexpected subscription request: {:?}", request),
        }
    }

    /// Returns true iff the subscription has expired
    fn is_expired(&self, timeout_ms: u64) -> bool {
        let current_time = self.time_service.now();
        let elapsed_time = current_time
            .duration_since(self.subscription_start_time)
            .as_millis();
        elapsed_time > timeout_ms as u128
    }
}

/// Handles ready (and expired) data subscriptions
pub(crate) fn handle_active_data_subscriptions<T: StorageReaderInterface>(
    cached_storage_server_summary: Arc<RwLock<StorageServerSummary>>,
    config: StorageServiceConfig,
    data_subscriptions: Arc<Mutex<HashMap<PeerNetworkId, DataSubscriptionRequest>>>,
    lru_response_cache: Arc<Mutex<LruCache<StorageServiceRequest, StorageServiceResponse>>>,
    storage: T,
    time_service: TimeService,
) {
    // Remove all expired subscriptions
    remove_expired_data_subscriptions(config, data_subscriptions.clone());

    // Identify the peers with ready subscriptions
    let peers_with_ready_subscriptions = match get_peers_with_ready_subscriptions(
        cached_storage_server_summary.clone(),
        data_subscriptions.clone(),
        lru_response_cache.clone(),
        storage.clone(),
        time_service.clone(),
    ) {
        Ok(peers_with_ready_subscriptions) => peers_with_ready_subscriptions,
        Err(error) => {
            error!(LogSchema::new(LogEntry::SubscriptionRefresh)
                .error(&Error::UnexpectedErrorEncountered(error.to_string())));
            return;
        },
    };

    // Remove and handle the ready subscriptions
    for (peer, target_ledger_info) in peers_with_ready_subscriptions {
        if let Some(data_subscription) = data_subscriptions.clone().lock().remove(&peer) {
            if let Err(error) = notify_peer_of_new_data(
                cached_storage_server_summary.clone(),
                config,
                data_subscriptions.clone(),
                lru_response_cache.clone(),
                storage.clone(),
                time_service.clone(),
                &peer,
                data_subscription,
                target_ledger_info,
            ) {
                warn!(LogSchema::new(LogEntry::SubscriptionResponse)
                    .error(&Error::UnexpectedErrorEncountered(error.to_string())));
            }
        }
    }
}

/// Identifies the data subscriptions that can be handled now.
/// Returns the list of peers that made those subscriptions
/// alongside the ledger info at the target version for the peer.
pub(crate) fn get_peers_with_ready_subscriptions<T: StorageReaderInterface>(
    cached_storage_server_summary: Arc<RwLock<StorageServerSummary>>,
    data_subscriptions: Arc<Mutex<HashMap<PeerNetworkId, DataSubscriptionRequest>>>,
    lru_response_cache: Arc<Mutex<LruCache<StorageServiceRequest, StorageServiceResponse>>>,
    storage: T,
    time_service: TimeService,
) -> aptos_storage_service_types::Result<Vec<(PeerNetworkId, LedgerInfoWithSignatures)>, Error> {
    // Fetch the latest storage summary and highest synced version
    let latest_storage_summary = cached_storage_server_summary.read().clone();
    let highest_synced_ledger_info = match latest_storage_summary.data_summary.synced_ledger_info {
        Some(ledger_info) => ledger_info,
        None => return Ok(vec![]),
    };
    let highest_synced_version = highest_synced_ledger_info.ledger_info().version();
    let highest_synced_epoch = highest_synced_ledger_info.ledger_info().epoch();

    // Identify the peers with ready subscriptions
    let mut ready_subscriptions = vec![];
    let mut invalid_peer_subscriptions = vec![];
    for (peer, data_subscription) in data_subscriptions.lock().iter() {
        let highest_known_version = data_subscription.highest_known_version();
        if highest_known_version < highest_synced_version {
            let highest_known_epoch = data_subscription.highest_known_epoch();
            if highest_known_epoch < highest_synced_epoch {
                // The peer needs to sync to their epoch ending ledger info
                let epoch_ending_ledger_info = get_epoch_ending_ledger_info(
                    cached_storage_server_summary.clone(),
                    data_subscriptions.clone(),
                    highest_known_epoch,
                    lru_response_cache.clone(),
                    peer,
                    data_subscription.protocol,
                    storage.clone(),
                    time_service.clone(),
                )?;

                // Check that we haven't been sent an invalid subscription request
                // (i.e., a request that does not respect an epoch boundary).
                if epoch_ending_ledger_info.ledger_info().version() <= highest_known_version {
                    invalid_peer_subscriptions.push(*peer);
                } else {
                    ready_subscriptions.push((*peer, epoch_ending_ledger_info));
                }
            } else {
                ready_subscriptions.push((*peer, highest_synced_ledger_info.clone()));
            };
        }
    }

    // Remove the invalid subscriptions
    for peer in invalid_peer_subscriptions {
        if let Some(data_subscription) = data_subscriptions.lock().remove(&peer) {
            debug!(LogSchema::new(LogEntry::SubscriptionRefresh)
                .error(&Error::InvalidRequest(
                    "Mismatch between known version and epoch!".into()
                ))
                .request(&data_subscription.request)
                .message("Dropping invalid subscription request!"));
        }
    }

    // Return the ready subscriptions
    Ok(ready_subscriptions)
}

/// Gets the epoch ending ledger info at the given epoch
fn get_epoch_ending_ledger_info<T: StorageReaderInterface>(
    cached_storage_server_summary: Arc<RwLock<StorageServerSummary>>,
    data_subscriptions: Arc<Mutex<HashMap<PeerNetworkId, DataSubscriptionRequest>>>,
    epoch: u64,
    lru_response_cache: Arc<Mutex<LruCache<StorageServiceRequest, StorageServiceResponse>>>,
    peer_network_id: &PeerNetworkId,
    protocol: ProtocolId,
    storage: T,
    time_service: TimeService,
) -> aptos_storage_service_types::Result<LedgerInfoWithSignatures, Error> {
    // Create a new storage request for the epoch ending ledger info
    let data_request = DataRequest::GetEpochEndingLedgerInfos(EpochEndingLedgerInfoRequest {
        start_epoch: epoch,
        expected_end_epoch: epoch,
    });
    let storage_request = StorageServiceRequest::new(
        data_request,
        false, // Don't compress because this isn't going over the wire
    );

    // Process the request
    let handler = Handler::new(
        cached_storage_server_summary,
        data_subscriptions,
        lru_response_cache,
        storage,
        time_service,
    );
    let storage_response =
        handler.process_request(peer_network_id, protocol, storage_request, true);

    // Verify the response
    match storage_response {
        Ok(storage_response) => match &storage_response.get_data_response() {
            Ok(DataResponse::EpochEndingLedgerInfos(epoch_change_proof)) => {
                if let Some(ledger_info) = epoch_change_proof.ledger_info_with_sigs.first() {
                    Ok(ledger_info.clone())
                } else {
                    Err(Error::UnexpectedErrorEncountered(
                        "Empty change proof found!".into(),
                    ))
                }
            },
            data_response => Err(Error::StorageErrorEncountered(format!(
                "Failed to get epoch ending ledger info! Got: {:?}",
                data_response
            ))),
        },
        Err(error) => Err(Error::StorageErrorEncountered(format!(
            "Failed to get epoch ending ledger info! Error: {:?}",
            error
        ))),
    }
}

/// Notifies a subscriber of new data according to the target ledger info
fn notify_peer_of_new_data<T: StorageReaderInterface>(
    cached_storage_server_summary: Arc<RwLock<StorageServerSummary>>,
    config: StorageServiceConfig,
    data_subscriptions: Arc<Mutex<HashMap<PeerNetworkId, DataSubscriptionRequest>>>,
    lru_response_cache: Arc<Mutex<LruCache<StorageServiceRequest, StorageServiceResponse>>>,
    storage: T,
    time_service: TimeService,
    peer_network_id: &PeerNetworkId,
    subscription: DataSubscriptionRequest,
    target_ledger_info: LedgerInfoWithSignatures,
) -> aptos_storage_service_types::Result<(), Error> {
    match subscription.get_storage_request_for_missing_data(config, &target_ledger_info) {
        Ok(storage_request) => {
            // Handle the storage service request to fetch the missing data
            let use_compression = storage_request.use_compression;
            let handler = Handler::new(
                cached_storage_server_summary,
                data_subscriptions,
                lru_response_cache,
                storage,
                time_service,
            );
            let storage_response = handler.process_request(
                peer_network_id,
                subscription.protocol,
                storage_request.clone(),
                true,
            );

            // Transform the missing data into a subscription response
            let transformed_data_response = match storage_response {
                Ok(storage_response) => match storage_response.get_data_response() {
                    Ok(DataResponse::TransactionsWithProof(transactions_with_proof)) => {
                        DataResponse::NewTransactionsWithProof((
                            transactions_with_proof,
                            target_ledger_info.clone(),
                        ))
                    },
                    Ok(DataResponse::TransactionOutputsWithProof(outputs_with_proof)) => {
                        DataResponse::NewTransactionOutputsWithProof((
                            outputs_with_proof,
                            target_ledger_info.clone(),
                        ))
                    },
                    Ok(DataResponse::TransactionsOrOutputsWithProof((
                        transactions_with_proof,
                        outputs_with_proof,
                    ))) => {
                        if let Some(transactions_with_proof) = transactions_with_proof {
                            DataResponse::NewTransactionsOrOutputsWithProof((
                                (Some(transactions_with_proof), None),
                                target_ledger_info.clone(),
                            ))
                        } else if let Some(outputs_with_proof) = outputs_with_proof {
                            DataResponse::NewTransactionsOrOutputsWithProof((
                                (None, Some(outputs_with_proof)),
                                target_ledger_info.clone(),
                            ))
                        } else {
                            return Err(Error::UnexpectedErrorEncountered(
                                "Failed to get a transaction or output response for peer!".into(),
                            ));
                        }
                    },
                    data_response => {
                        return Err(Error::UnexpectedErrorEncountered(format!(
                            "Failed to get appropriate data response for peer! Got: {:?}",
                            data_response
                        )))
                    },
                },
                response => {
                    return Err(Error::UnexpectedErrorEncountered(format!(
                        "Failed to fetch missing data for peer! {:?}",
                        response
                    )))
                },
            };
            let storage_response =
                match StorageServiceResponse::new(transformed_data_response, use_compression) {
                    Ok(storage_response) => storage_response,
                    Err(error) => {
                        return Err(Error::UnexpectedErrorEncountered(format!(
                            "Failed to create transformed response! Error: {:?}",
                            error
                        )));
                    },
                };

            // If the storage response has overflown the network frame size
            // return an error. We don't need to retry with less data because
            // subscription requests are best effort.
            let (overflow_frame, num_bytes) = crate::storage::check_overflow_network_frame(
                &storage_response,
                config.max_network_chunk_bytes,
            )?;
            if overflow_frame {
                increment_network_frame_overflow(&storage_response.get_label());
                debug!(
                    "The request for the new data was too large (num bytes: {:?})!",
                    num_bytes
                );
                return Err(Error::UnexpectedErrorEncountered(
                    "Failed to notify the peer of new data! The response overflowed the network frame size!".into(),
                ));
            }

            // Send the response to the peer
            handler.send_response(
                storage_request,
                Ok(storage_response),
                subscription.response_sender,
            );
            Ok(())
        },
        Err(error) => Err(error),
    }
}

/// Removes all expired data subscriptions
pub(crate) fn remove_expired_data_subscriptions(
    config: StorageServiceConfig,
    data_subscriptions: Arc<Mutex<HashMap<PeerNetworkId, DataSubscriptionRequest>>>,
) {
    data_subscriptions.lock().retain(|_, data_subscription| {
        // Update the expired subscription metrics
        if data_subscription.is_expired(config.max_subscription_period_ms) {
            let protocol = data_subscription.protocol;
            increment_counter(
                &metrics::SUBSCRIPTION_EVENT,
                protocol,
                SUBSCRIPTION_EVENT_EXPIRE.into(),
            );
        }

        // Only retain non-expired subscriptions
        !data_subscription.is_expired(config.max_subscription_period_ms)
    });
}
