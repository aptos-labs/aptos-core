// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::Error, handler::Handler, metrics, moderator::RequestModerator, network::ResponseSender,
    optimistic_fetch::OptimisticFetchRequest, storage::StorageReaderInterface,
    subscription::SubscriptionStreamRequests,
};
use aptos_config::network_id::PeerNetworkId;
use aptos_metrics_core::HistogramVec;
use aptos_storage_service_types::{
    requests::{DataRequest, EpochEndingLedgerInfoRequest, StorageServiceRequest},
    responses::{
        DataResponse, NewTransactionDataWithProofResponse, StorageServerSummary,
        StorageServiceResponse,
    },
};
use aptos_time_service::TimeService;
use aptos_types::ledger_info::LedgerInfoWithSignatures;
use arc_swap::ArcSwap;
use dashmap::DashMap;
use mini_moka::sync::Cache;
use once_cell::sync::Lazy;
use std::{sync::Arc, time::Instant};

/// Gets the epoch ending ledger info at the given epoch
pub fn get_epoch_ending_ledger_info<T: StorageReaderInterface>(
    cached_storage_server_summary: Arc<ArcSwap<StorageServerSummary>>,
    optimistic_fetches: Arc<DashMap<PeerNetworkId, OptimisticFetchRequest>>,
    subscriptions: Arc<DashMap<PeerNetworkId, SubscriptionStreamRequests>>,
    epoch: u64,
    lru_response_cache: Cache<StorageServiceRequest, StorageServiceResponse>,
    request_moderator: Arc<RequestModerator>,
    peer_network_id: &PeerNetworkId,
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
        optimistic_fetches,
        lru_response_cache,
        request_moderator,
        storage,
        subscriptions,
        time_service,
    );
    let storage_response = handler.process_request(peer_network_id, storage_request, true);

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

/// Notifies a peer of new data according to the target ledger info
/// and returns a copy of the raw data response that was sent.
///
/// Note: we don't need to check the size of the response because:
/// (i) each sub-part should already be checked; and (ii) responses
pub fn notify_peer_of_new_data<T: StorageReaderInterface>(
    cached_storage_server_summary: Arc<ArcSwap<StorageServerSummary>>,
    optimistic_fetches: Arc<DashMap<PeerNetworkId, OptimisticFetchRequest>>,
    subscriptions: Arc<DashMap<PeerNetworkId, SubscriptionStreamRequests>>,
    lru_response_cache: Cache<StorageServiceRequest, StorageServiceResponse>,
    request_moderator: Arc<RequestModerator>,
    storage: T,
    time_service: TimeService,
    peer_network_id: &PeerNetworkId,
    missing_data_request: StorageServiceRequest,
    target_ledger_info: LedgerInfoWithSignatures,
    response_sender: ResponseSender,
) -> aptos_storage_service_types::Result<DataResponse, Error> {
    // Handle the storage service request to fetch the missing data
    let use_compression = missing_data_request.use_compression;
    let handler = Handler::new(
        cached_storage_server_summary,
        optimistic_fetches,
        lru_response_cache,
        request_moderator,
        storage,
        subscriptions,
        time_service,
    );
    let storage_response =
        handler.process_request(peer_network_id, missing_data_request.clone(), true);

    // Transform the missing data into an optimistic fetch response
    let transformed_data_response = match storage_response {
        Ok(storage_response) => match storage_response.get_data_response() {
            Ok(DataResponse::TransactionsWithProof(transactions_with_proof)) => {
                DataResponse::NewTransactionsWithProof((
                    transactions_with_proof,
                    target_ledger_info,
                ))
            },
            Ok(DataResponse::TransactionOutputsWithProof(outputs_with_proof)) => {
                DataResponse::NewTransactionOutputsWithProof((
                    outputs_with_proof,
                    target_ledger_info,
                ))
            },
            Ok(DataResponse::TransactionsOrOutputsWithProof((
                transactions_with_proof,
                outputs_with_proof,
            ))) => {
                if let Some(transactions_with_proof) = transactions_with_proof {
                    DataResponse::NewTransactionsOrOutputsWithProof((
                        (Some(transactions_with_proof), None),
                        target_ledger_info,
                    ))
                } else if let Some(outputs_with_proof) = outputs_with_proof {
                    DataResponse::NewTransactionsOrOutputsWithProof((
                        (None, Some(outputs_with_proof)),
                        target_ledger_info,
                    ))
                } else {
                    return Err(Error::UnexpectedErrorEncountered(
                        "Failed to get a transaction or output response for peer!".into(),
                    ));
                }
            },
            Ok(DataResponse::TransactionDataWithProof(transaction_data_with_proof)) => {
                DataResponse::NewTransactionDataWithProof(NewTransactionDataWithProofResponse {
                    transaction_data_response_type: transaction_data_with_proof
                        .transaction_data_response_type,
                    transaction_list_with_proof: transaction_data_with_proof
                        .transaction_list_with_proof,
                    transaction_output_list_with_proof: transaction_data_with_proof
                        .transaction_output_list_with_proof,
                    ledger_info_with_signatures: target_ledger_info,
                })
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

    // Create the storage service response
    let storage_response =
        match StorageServiceResponse::new(transformed_data_response.clone(), use_compression) {
            Ok(storage_response) => storage_response,
            Err(error) => {
                return Err(Error::UnexpectedErrorEncountered(format!(
                    "Failed to create transformed response! Error: {:?}",
                    error
                )));
            },
        };

    // Send the response to the peer
    handler.send_response(missing_data_request, Ok(storage_response), response_sender);

    Ok(transformed_data_response)
}

/// An utility that calls and times the given function. The metric histogram
/// is updated with the given labels (e.g., peer, request result and duration).
/// If no start time is specified, the timer begins before calling the function.
pub fn execute_and_time_duration<T, F>(
    histogram: &Lazy<HistogramVec>,
    peer_and_request: Option<(&PeerNetworkId, &StorageServiceRequest)>,
    histogram_label: Option<String>,
    function_to_call: F,
    start_time: Option<Instant>,
) -> Result<T, Error>
where
    F: FnOnce() -> Result<T, Error>,
{
    // Start the timer (if not already started)
    let start_time = start_time.unwrap_or_else(Instant::now);

    // Call the function and get the result
    let result = function_to_call();

    // Identify the result label
    let result_label = if result.is_ok() {
        metrics::RESULT_SUCCESS
    } else {
        metrics::RESULT_FAILURE
    };

    // Determine the label values to use
    let mut label_values = vec![];
    if let Some((peer, request)) = peer_and_request {
        // Add the peer and request labels
        label_values.push(peer.network_id().as_str().into());
        label_values.push(request.get_label());
    }
    if let Some(histogram_label) = histogram_label {
        label_values.push(histogram_label); // Add the histogram label
    }
    label_values.push(result_label.into()); // Add the result label to the end

    // Observe the function duration
    metrics::observe_duration(histogram, label_values, start_time);

    // Return the result
    result
}
