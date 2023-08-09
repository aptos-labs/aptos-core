// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::Error,
    logging::{LogEntry, LogSchema},
    metrics,
    metrics::{
        increment_counter, start_timer, LRU_CACHE_HIT, LRU_CACHE_PROBE, OPTIMISTIC_FETCH_ADD,
    },
    moderator::RequestModerator,
    network::ResponseSender,
    optimistic_fetch::OptimisticFetchRequest,
    storage::StorageReaderInterface,
};
use aptos_config::network_id::PeerNetworkId;
use aptos_infallible::Mutex;
use aptos_logger::{debug, error, sample, sample::SampleRate, trace, warn};
use aptos_storage_service_types::{
    requests::{
        DataRequest, EpochEndingLedgerInfoRequest, StateValuesWithProofRequest,
        StorageServiceRequest, TransactionOutputsWithProofRequest,
        TransactionsOrOutputsWithProofRequest, TransactionsWithProofRequest,
    },
    responses::{
        DataResponse, ServerProtocolVersion, StorageServerSummary, StorageServiceResponse,
    },
    StorageServiceError,
};
use aptos_time_service::TimeService;
use aptos_types::transaction::Version;
use arc_swap::ArcSwap;
use dashmap::DashMap;
use lru::LruCache;
use std::{sync::Arc, time::Duration};

/// Storage server constants
const INVALID_REQUEST_LOG_FREQUENCY_SECS: u64 = 5; // The frequency to log invalid requests (secs)
const STORAGE_SERVER_VERSION: u64 = 1;
const SUMMARY_LOG_FREQUENCY_SECS: u64 = 5; // The frequency to log the storage server summary (secs)

/// The `Handler` is the "pure" inbound request handler. It contains all the
/// necessary context and state needed to construct a response to an inbound
/// request. We usually clone/create a new handler for every request.
#[derive(Clone)]
pub struct Handler<T> {
    cached_storage_server_summary: Arc<ArcSwap<StorageServerSummary>>,
    optimistic_fetches: Arc<DashMap<PeerNetworkId, OptimisticFetchRequest>>,
    lru_response_cache: Arc<Mutex<LruCache<StorageServiceRequest, StorageServiceResponse>>>,
    request_moderator: Arc<RequestModerator>,
    storage: T,
    time_service: TimeService,
}

impl<T: StorageReaderInterface> Handler<T> {
    pub fn new(
        cached_storage_server_summary: Arc<ArcSwap<StorageServerSummary>>,
        optimistic_fetches: Arc<DashMap<PeerNetworkId, OptimisticFetchRequest>>,
        lru_response_cache: Arc<Mutex<LruCache<StorageServiceRequest, StorageServiceResponse>>>,
        request_moderator: Arc<RequestModerator>,
        storage: T,
        time_service: TimeService,
    ) -> Self {
        Self {
            storage,
            cached_storage_server_summary,
            optimistic_fetches,
            lru_response_cache,
            request_moderator,
            time_service,
        }
    }

    /// Handles the given storage service request and responds to the
    /// request directly.
    pub fn process_request_and_respond(
        &self,
        peer_network_id: PeerNetworkId,
        request: StorageServiceRequest,
        response_sender: ResponseSender,
    ) {
        // Update the request count
        increment_counter(
            &metrics::STORAGE_REQUESTS_RECEIVED,
            peer_network_id.network_id(),
            request.get_label(),
        );

        // Handle any optimistic fetch requests
        if request.data_request.is_optimistic_fetch() {
            self.handle_optimistic_fetch_request(peer_network_id, request, response_sender);
            return;
        }

        // Process the request and return the response to the client
        let response = self.process_request(&peer_network_id, request.clone(), false);
        self.send_response(request, response, response_sender);
    }

    /// Processes the given request and returns the response
    pub(crate) fn process_request(
        &self,
        peer_network_id: &PeerNetworkId,
        request: StorageServiceRequest,
        optimistic_fetch_related: bool,
    ) -> aptos_storage_service_types::Result<StorageServiceResponse> {
        // Time the request processing (the timer will stop when it's dropped)
        let _timer = start_timer(
            &metrics::STORAGE_REQUEST_PROCESSING_LATENCY,
            peer_network_id.network_id(),
            request.get_label(),
        );

        // Process the request and handle any errors
        match self.validate_and_handle_request(peer_network_id, &request) {
            Err(error) => {
                // Update the error counter
                increment_counter(
                    &metrics::STORAGE_ERRORS_ENCOUNTERED,
                    peer_network_id.network_id(),
                    error.get_label().into(),
                );

                // Periodically log the validation failure
                sample!(
                        SampleRate::Duration(Duration::from_secs(INVALID_REQUEST_LOG_FREQUENCY_SECS)),
                        error!(LogSchema::new(LogEntry::StorageServiceError)
                            .error(&error)
                            .peer_network_id(peer_network_id)
                            .request(&request)
                            .optimistic_fetch_related(optimistic_fetch_related)
                    );
                );

                // Return an appropriate response to the client
                match error {
                    Error::InvalidRequest(error) => Err(StorageServiceError::InvalidRequest(error)),
                    Error::TooManyInvalidRequests(error) => {
                        Err(StorageServiceError::TooManyInvalidRequests(error))
                    },
                    error => Err(StorageServiceError::InternalError(error.to_string())),
                }
            },
            Ok(response) => {
                // Update the successful response counter
                increment_counter(
                    &metrics::STORAGE_RESPONSES_SENT,
                    peer_network_id.network_id(),
                    response.get_label(),
                );

                // Return the response
                Ok(response)
            },
        }
    }

    /// Validate the request and only handle it if the moderator allows
    fn validate_and_handle_request(
        &self,
        peer_network_id: &PeerNetworkId,
        request: &StorageServiceRequest,
    ) -> Result<StorageServiceResponse, Error> {
        // Validate the request with the moderator
        self.request_moderator
            .validate_request(peer_network_id, request)?;

        // Process the request
        match &request.data_request {
            DataRequest::GetServerProtocolVersion => {
                let data_response = self.get_server_protocol_version();
                StorageServiceResponse::new(data_response, request.use_compression)
                    .map_err(|error| error.into())
            },
            DataRequest::GetStorageServerSummary => {
                let data_response = self.get_storage_server_summary();
                StorageServiceResponse::new(data_response, request.use_compression)
                    .map_err(|error| error.into())
            },
            _ => self.process_cachable_request(peer_network_id, request),
        }
    }

    /// Sends a response via the provided sender
    pub(crate) fn send_response(
        &self,
        request: StorageServiceRequest,
        response: aptos_storage_service_types::Result<StorageServiceResponse>,
        response_sender: ResponseSender,
    ) {
        log_storage_response(request, &response);
        response_sender.send(response);
    }

    /// Handles the given optimistic fetch request
    pub fn handle_optimistic_fetch_request(
        &self,
        peer_network_id: PeerNetworkId,
        request: StorageServiceRequest,
        response_sender: ResponseSender,
    ) {
        // Create the optimistic fetch request
        let optimistic_fetch = OptimisticFetchRequest::new(
            request.clone(),
            response_sender,
            self.time_service.clone(),
        );

        // Store the optimistic fetch and check if any existing fetches were found
        if self
            .optimistic_fetches
            .insert(peer_network_id, optimistic_fetch)
            .is_some()
        {
            sample!(
                SampleRate::Duration(Duration::from_secs(INVALID_REQUEST_LOG_FREQUENCY_SECS)),
                warn!(LogSchema::new(LogEntry::OptimisticFetchRequest)
                    .error(&Error::InvalidRequest(
                        "An active optimistic fetch was already found for the peer!".into()
                    ))
                    .peer_network_id(&peer_network_id)
                    .request(&request)
                );
            );
        }

        // Update the optimistic fetch metrics
        increment_counter(
            &metrics::OPTIMISTIC_FETCH_EVENTS,
            peer_network_id.network_id(),
            OPTIMISTIC_FETCH_ADD.into(),
        );
    }

    /// Processes a storage service request for which the response
    /// might already be cached.
    fn process_cachable_request(
        &self,
        peer_network_id: &PeerNetworkId,
        request: &StorageServiceRequest,
    ) -> aptos_storage_service_types::Result<StorageServiceResponse, Error> {
        increment_counter(
            &metrics::LRU_CACHE_EVENT,
            peer_network_id.network_id(),
            LRU_CACHE_PROBE.into(),
        );

        // Check if the response is already in the cache
        if let Some(response) = self.lru_response_cache.lock().get(request) {
            increment_counter(
                &metrics::LRU_CACHE_EVENT,
                peer_network_id.network_id(),
                LRU_CACHE_HIT.into(),
            );
            return Ok(response.clone());
        }

        // Fetch the data response from storage
        let data_response = match &request.data_request {
            DataRequest::GetStateValuesWithProof(request) => {
                self.get_state_value_chunk_with_proof(request)
            },
            DataRequest::GetEpochEndingLedgerInfos(request) => {
                self.get_epoch_ending_ledger_infos(request)
            },
            DataRequest::GetNumberOfStatesAtVersion(version) => {
                self.get_number_of_states_at_version(*version)
            },
            DataRequest::GetTransactionOutputsWithProof(request) => {
                self.get_transaction_outputs_with_proof(request)
            },
            DataRequest::GetTransactionsWithProof(request) => {
                self.get_transactions_with_proof(request)
            },
            DataRequest::GetTransactionsOrOutputsWithProof(request) => {
                self.get_transactions_or_outputs_with_proof(request)
            },
            _ => Err(Error::UnexpectedErrorEncountered(format!(
                "Received an unexpected request: {:?}",
                request
            ))),
        }?;
        let storage_response = StorageServiceResponse::new(data_response, request.use_compression)?;

        // Cache the response before returning
        let _ = self
            .lru_response_cache
            .lock()
            .put(request.clone(), storage_response.clone());

        Ok(storage_response)
    }

    fn get_state_value_chunk_with_proof(
        &self,
        request: &StateValuesWithProofRequest,
    ) -> aptos_storage_service_types::Result<DataResponse, Error> {
        let state_value_chunk_with_proof = self.storage.get_state_value_chunk_with_proof(
            request.version,
            request.start_index,
            request.end_index,
        )?;

        Ok(DataResponse::StateValueChunkWithProof(
            state_value_chunk_with_proof,
        ))
    }

    fn get_epoch_ending_ledger_infos(
        &self,
        request: &EpochEndingLedgerInfoRequest,
    ) -> aptos_storage_service_types::Result<DataResponse, Error> {
        let epoch_change_proof = self
            .storage
            .get_epoch_ending_ledger_infos(request.start_epoch, request.expected_end_epoch)?;

        Ok(DataResponse::EpochEndingLedgerInfos(epoch_change_proof))
    }

    fn get_number_of_states_at_version(
        &self,
        version: Version,
    ) -> aptos_storage_service_types::Result<DataResponse, Error> {
        let number_of_states = self.storage.get_number_of_states(version)?;

        Ok(DataResponse::NumberOfStatesAtVersion(number_of_states))
    }

    fn get_server_protocol_version(&self) -> DataResponse {
        let server_protocol_version = ServerProtocolVersion {
            protocol_version: STORAGE_SERVER_VERSION,
        };
        DataResponse::ServerProtocolVersion(server_protocol_version)
    }

    fn get_storage_server_summary(&self) -> DataResponse {
        let storage_server_summary = self.cached_storage_server_summary.load().clone();
        DataResponse::StorageServerSummary(storage_server_summary.as_ref().clone())
    }

    fn get_transaction_outputs_with_proof(
        &self,
        request: &TransactionOutputsWithProofRequest,
    ) -> aptos_storage_service_types::Result<DataResponse, Error> {
        let transaction_output_list_with_proof = self.storage.get_transaction_outputs_with_proof(
            request.proof_version,
            request.start_version,
            request.end_version,
        )?;

        Ok(DataResponse::TransactionOutputsWithProof(
            transaction_output_list_with_proof,
        ))
    }

    fn get_transactions_with_proof(
        &self,
        request: &TransactionsWithProofRequest,
    ) -> aptos_storage_service_types::Result<DataResponse, Error> {
        let transactions_with_proof = self.storage.get_transactions_with_proof(
            request.proof_version,
            request.start_version,
            request.end_version,
            request.include_events,
        )?;

        Ok(DataResponse::TransactionsWithProof(transactions_with_proof))
    }

    fn get_transactions_or_outputs_with_proof(
        &self,
        request: &TransactionsOrOutputsWithProofRequest,
    ) -> aptos_storage_service_types::Result<DataResponse, Error> {
        let (transactions_with_proof, outputs_with_proof) =
            self.storage.get_transactions_or_outputs_with_proof(
                request.proof_version,
                request.start_version,
                request.end_version,
                request.include_events,
                request.max_num_output_reductions,
            )?;

        Ok(DataResponse::TransactionsOrOutputsWithProof((
            transactions_with_proof,
            outputs_with_proof,
        )))
    }
}

/// Logs the response sent by storage for a peer request
fn log_storage_response(
    storage_request: StorageServiceRequest,
    storage_response: &aptos_storage_service_types::Result<
        StorageServiceResponse,
        StorageServiceError,
    >,
) {
    match storage_response {
        Ok(storage_response) => {
            // We expect peers to be polling our storage server summary frequently,
            // so only log this response periodically.
            if matches!(
                storage_request.data_request,
                DataRequest::GetStorageServerSummary
            ) {
                sample!(
                    SampleRate::Duration(Duration::from_secs(SUMMARY_LOG_FREQUENCY_SECS)),
                    {
                        if let Ok(data_response) = storage_response.get_data_response() {
                            let response = format!("{}", data_response);
                            debug!(
                                LogSchema::new(LogEntry::SentStorageResponse).response(&response)
                            );
                        }
                    }
                );
            }
        },
        Err(storage_error) => {
            let storage_error = format!("{:?}", storage_error);
            trace!(LogSchema::new(LogEntry::SentStorageResponse).response(&storage_error));
        },
    };
}
