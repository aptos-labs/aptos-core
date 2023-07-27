// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::Error, handler::Handler, moderator::RequestModerator,
    optimistic_fetch::OptimisticFetchRequest, storage::StorageReaderInterface,
};
use aptos_config::{config::StorageServiceConfig, network_id::PeerNetworkId};
use aptos_infallible::Mutex;
use aptos_storage_service_types::{
    requests::{DataRequest, EpochEndingLedgerInfoRequest, StorageServiceRequest},
    responses::{DataResponse, StorageServerSummary, StorageServiceResponse},
};
use aptos_time_service::TimeService;
use aptos_types::ledger_info::LedgerInfoWithSignatures;
use arc_swap::ArcSwap;
use dashmap::DashMap;
use lru::LruCache;
use std::sync::Arc;

/// Gets the epoch ending ledger info at the given epoch
pub fn get_epoch_ending_ledger_info<T: StorageReaderInterface>(
    cached_storage_server_summary: Arc<ArcSwap<StorageServerSummary>>,
    optimistic_fetches: Arc<DashMap<PeerNetworkId, OptimisticFetchRequest>>,
    epoch: u64,
    lru_response_cache: Arc<Mutex<LruCache<StorageServiceRequest, StorageServiceResponse>>>,
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

/// Notifies a peer of new data according to the target ledger info.
///
/// Note: we don't need to check the size of the optimistic fetch response
/// because: (i) each sub-part should already be checked; and (ii)
/// optimistic fetch responses are best effort.
pub fn notify_peer_of_new_data<T: StorageReaderInterface>(
    cached_storage_server_summary: Arc<ArcSwap<StorageServerSummary>>,
    config: StorageServiceConfig,
    optimistic_fetches: Arc<DashMap<PeerNetworkId, OptimisticFetchRequest>>,
    lru_response_cache: Arc<Mutex<LruCache<StorageServiceRequest, StorageServiceResponse>>>,
    request_moderator: Arc<RequestModerator>,
    storage: T,
    time_service: TimeService,
    peer_network_id: &PeerNetworkId,
    optimistic_fetch: OptimisticFetchRequest,
    target_ledger_info: LedgerInfoWithSignatures,
) -> aptos_storage_service_types::Result<(), Error> {
    match optimistic_fetch.get_storage_request_for_missing_data(config, &target_ledger_info) {
        Ok(storage_request) => {
            // Handle the storage service request to fetch the missing data
            let use_compression = storage_request.use_compression;
            let handler = Handler::new(
                cached_storage_server_summary,
                optimistic_fetches,
                lru_response_cache,
                request_moderator,
                storage,
                time_service,
            );
            let storage_response =
                handler.process_request(peer_network_id, storage_request.clone(), true);

            // Transform the missing data into an optimistic fetch response
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

            // Send the response to the peer
            handler.send_response(
                storage_request,
                Ok(storage_response),
                optimistic_fetch.get_response_sender(),
            );
            Ok(())
        },
        Err(error) => Err(error),
    }
}
