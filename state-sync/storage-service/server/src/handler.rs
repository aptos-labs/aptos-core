// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::Error,
    logging::{LogEntry, LogSchema},
    metrics,
    metrics::{
        increment_counter, LRU_CACHE_HIT, LRU_CACHE_PROBE, OPTIMISTIC_FETCH_ADD, SUBSCRIPTION_ADD,
        SUBSCRIPTION_FAILURE, SUBSCRIPTION_NEW_STREAM,
    },
    moderator::RequestModerator,
    network::ResponseSender,
    optimistic_fetch::OptimisticFetchRequest,
    storage::StorageReaderInterface,
    subscription::{SubscriptionRequest, SubscriptionStreamRequests},
    utils,
};
use aptos_config::{config::StorageServiceConfig, network_id::PeerNetworkId};
use aptos_logger::{debug, sample, sample::SampleRate, trace, warn};
use aptos_network::protocols::wire::handshake::v1::ProtocolId;
use aptos_storage_service_types::{
    requests::{
        DataRequest, EpochEndingLedgerInfoRequest, GetTransactionDataWithProofRequest,
        StateValuesWithProofRequest, StorageServiceRequest, TransactionOutputsWithProofRequest,
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
use dashmap::{mapref::entry::Entry, DashMap};
use mini_moka::sync::Cache;
use std::{sync::Arc, time::Duration};

/// Storage server constants
const ERROR_LOG_FREQUENCY_SECS: u64 = 5; // The frequency to log errors
const STORAGE_SERVER_VERSION: u64 = 1;
const SUMMARY_LOG_FREQUENCY_SECS: u64 = 5; // The frequency to log the storage server summary (secs)

/// The `Handler` is the "pure" inbound request handler. It contains all the
/// necessary context and state needed to construct a response to an inbound
/// request. We usually clone/create a new handler for every request.
#[derive(Clone)]
pub struct Handler<T> {
    cached_storage_server_summary: Arc<ArcSwap<StorageServerSummary>>,
    optimistic_fetches: Arc<DashMap<PeerNetworkId, OptimisticFetchRequest>>,
    lru_response_cache: Cache<StorageServiceRequest, StorageServiceResponse>,
    request_moderator: Arc<RequestModerator>,
    storage: T,
    subscriptions: Arc<DashMap<PeerNetworkId, SubscriptionStreamRequests>>,
    time_service: TimeService,
}

impl<T: StorageReaderInterface> Handler<T> {
    pub fn new(
        cached_storage_server_summary: Arc<ArcSwap<StorageServerSummary>>,
        optimistic_fetches: Arc<DashMap<PeerNetworkId, OptimisticFetchRequest>>,
        lru_response_cache: Cache<StorageServiceRequest, StorageServiceResponse>,
        request_moderator: Arc<RequestModerator>,
        storage: T,
        subscriptions: Arc<DashMap<PeerNetworkId, SubscriptionStreamRequests>>,
        time_service: TimeService,
    ) -> Self {
        Self {
            cached_storage_server_summary,
            optimistic_fetches,
            lru_response_cache,
            request_moderator,
            storage,
            subscriptions,
            time_service,
        }
    }

    /// Handles the given storage service request and responds to the
    /// request directly.
    pub fn process_request_and_respond(
        &self,
        storage_service_config: StorageServiceConfig,
        peer_network_id: PeerNetworkId,
        protocol_id: ProtocolId,
        request: StorageServiceRequest,
        response_sender: ResponseSender,
    ) {
        // Log the request
        trace!(LogSchema::new(LogEntry::ReceivedStorageRequest)
            .request(&request)
            .message(&format!(
                "Received storage request. Peer: {:?}, protocol: {:?}.",
                peer_network_id, protocol_id,
            )));

        // Update the request count
        increment_counter(
            &metrics::STORAGE_REQUESTS_RECEIVED,
            peer_network_id.network_id(),
            request.get_label(),
        );

        // If the request is for transaction v2 data, only process it
        // if the server supports it. Otherwise, drop the request.
        if request.data_request.is_transaction_data_v2_request()
            && !storage_service_config.enable_transaction_data_v2
        {
            warn!(LogSchema::new(LogEntry::StorageServiceError)
                .error(&Error::InvalidRequest(format!(
                    "Received a v2 data request ({}), which is not supported!",
                    request.get_label()
                )))
                .peer_network_id(&peer_network_id));
            return;
        }

        // Handle any optimistic fetch requests
        if request.data_request.is_optimistic_fetch() {
            self.handle_optimistic_fetch_request(peer_network_id, request, response_sender);
            return;
        }

        // Handle any subscription requests
        if request.data_request.is_subscription_request() {
            self.handle_subscription_request(
                storage_service_config,
                peer_network_id,
                request,
                response_sender,
            );
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
        // Process the request and time the operation
        let process_request = || {
            // Process the request and handle any errors
            match self.validate_and_handle_request(peer_network_id, &request) {
                Err(error) => {
                    // Update the error counter
                    increment_counter(
                        &metrics::STORAGE_ERRORS_ENCOUNTERED,
                        peer_network_id.network_id(),
                        error.get_label().into(),
                    );

                    // Periodically log the failure
                    sample!(
                            SampleRate::Duration(Duration::from_secs(ERROR_LOG_FREQUENCY_SECS)),
                            warn!(LogSchema::new(LogEntry::StorageServiceError)
                                .error(&error)
                                .peer_network_id(peer_network_id)
                                .request(&request)
                                .optimistic_fetch_related(optimistic_fetch_related)
                        );
                    );

                    // Return the error
                    Err(error)
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
        };
        let process_result = utils::execute_and_time_duration(
            &metrics::STORAGE_REQUEST_PROCESSING_LATENCY,
            Some((peer_network_id, &request)),
            None,
            process_request,
            None,
        );

        // Transform the request error into a storage service error (for the client)
        process_result.map_err(|error| match error {
            Error::InvalidRequest(error) => StorageServiceError::InvalidRequest(error),
            Error::TooManyInvalidRequests(error) => {
                StorageServiceError::TooManyInvalidRequests(error)
            },
            error => StorageServiceError::InternalError(error.to_string()),
        })
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
                SampleRate::Duration(Duration::from_secs(ERROR_LOG_FREQUENCY_SECS)),
                trace!(LogSchema::new(LogEntry::OptimisticFetchRequest)
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

    /// Handles the given subscription request. If a failure
    /// occurs during handling, the client is notified.
    pub fn handle_subscription_request(
        &self,
        storage_service_config: StorageServiceConfig,
        peer_network_id: PeerNetworkId,
        request: StorageServiceRequest,
        response_sender: ResponseSender,
    ) {
        // Create a new subscription request and get the stream ID
        let subscription_request =
            SubscriptionRequest::new(request.clone(), response_sender, self.time_service.clone());
        let request_stream_id = subscription_request.subscription_stream_id();

        // Update the subscription metrics with the new request
        update_new_subscription_metrics(peer_network_id);

        // Get the subscription stream entry for the peer. Internally, this will
        // lock the entry, to prevent other requests (for the same peer) from
        // modifying the subscription stream entry.
        let subscription_stream_entry = self.subscriptions.entry(peer_network_id);

        // If the entry is empty, or the stream ID does not match the request ID,
        // create a new subscription stream for the peer. Otherwise, add the
        // request to the existing stream (the stream IDs match!).
        match subscription_stream_entry {
            Entry::Occupied(mut occupied_entry) => {
                // If the stream has a different ID than the request, replace the stream.
                // Otherwise, add the request to the existing stream.
                let existing_stream_id = occupied_entry.get().subscription_stream_id();
                if existing_stream_id != request_stream_id {
                    // Create a new subscription stream for the peer
                    let subscription_stream = SubscriptionStreamRequests::new(
                        subscription_request,
                        self.time_service.clone(),
                    );
                    occupied_entry.replace_entry(subscription_stream);

                    // Update the subscription metrics
                    update_created_stream_metrics(&peer_network_id);
                } else {
                    // Add the request to the existing stream
                    if let Err((error, subscription_request)) = occupied_entry
                        .get_mut()
                        .add_subscription_request(storage_service_config, subscription_request)
                    {
                        // Handle the subscription failure
                        self.handle_subscription_request_failure(
                            peer_network_id,
                            request,
                            error,
                            subscription_request,
                        );
                    }
                }
            },
            Entry::Vacant(vacant_entry) => {
                // Create a new subscription stream for the peer
                let subscription_stream = SubscriptionStreamRequests::new(
                    subscription_request,
                    self.time_service.clone(),
                );
                vacant_entry.insert(subscription_stream);

                // Update the subscription metrics
                update_created_stream_metrics(&peer_network_id);
            },
        }
    }

    /// Handles a subscription request failure by logging the error,
    /// updating the subscription metrics, and notifying the client.
    fn handle_subscription_request_failure(
        &self,
        peer_network_id: PeerNetworkId,
        request: StorageServiceRequest,
        error: Error,
        subscription_request: SubscriptionRequest,
    ) {
        // Something went wrong when adding the request to the stream
        sample!(
            SampleRate::Duration(Duration::from_secs(ERROR_LOG_FREQUENCY_SECS)),
            warn!(LogSchema::new(LogEntry::SubscriptionRequest)
                .error(&error)
                .peer_network_id(&peer_network_id)
                .request(&request)
            );
        );

        // Update the subscription metrics
        update_failed_subscription_metrics(peer_network_id);

        // Notify the client of the failure
        self.send_response(
            request,
            Err(StorageServiceError::InvalidRequest(error.to_string())),
            subscription_request.take_response_sender(),
        );
    }

    /// Processes a storage service request for which the response
    /// might already be cached.
    fn process_cachable_request(
        &self,
        peer_network_id: &PeerNetworkId,
        request: &StorageServiceRequest,
    ) -> aptos_storage_service_types::Result<StorageServiceResponse, Error> {
        // Increment the LRU cache probe counter
        increment_counter(
            &metrics::LRU_CACHE_EVENT,
            peer_network_id.network_id(),
            LRU_CACHE_PROBE.into(),
        );

        // Check if the response is already in the cache
        if let Some(response) = self.lru_response_cache.get(request) {
            increment_counter(
                &metrics::LRU_CACHE_EVENT,
                peer_network_id.network_id(),
                LRU_CACHE_HIT.into(),
            );
            return Ok(response.clone());
        }

        // Otherwise, fetch the data from storage and time the operation
        let fetch_data_response = || match &request.data_request {
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
            DataRequest::GetTransactionDataWithProof(request) => {
                self.get_transaction_data_with_proof(request)
            },
            _ => Err(Error::UnexpectedErrorEncountered(format!(
                "Received an unexpected request: {:?}",
                request
            ))),
        };
        let data_response = utils::execute_and_time_duration(
            &metrics::STORAGE_FETCH_PROCESSING_LATENCY,
            Some((peer_network_id, request)),
            None,
            fetch_data_response,
            None,
        )?;

        // Create the storage response and time the operation
        let create_storage_response = || {
            StorageServiceResponse::new(data_response, request.use_compression)
                .map_err(|error| error.into())
        };
        let storage_response = utils::execute_and_time_duration(
            &metrics::STORAGE_RESPONSE_CREATION_LATENCY,
            Some((peer_network_id, request)),
            None,
            create_storage_response,
            None,
        )?;

        // Create and cache the storage response
        self.lru_response_cache
            .insert(request.clone(), storage_response.clone());

        // Return the storage response
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
        let response = self.storage.get_transaction_outputs_with_proof(
            request.proof_version,
            request.start_version,
            request.end_version,
        )?;

        Ok(DataResponse::TransactionOutputsWithProof(
            response
                .transaction_output_list_with_proof
                .unwrap()
                .into_parts()
                .0,
        ))
    }

    fn get_transactions_with_proof(
        &self,
        request: &TransactionsWithProofRequest,
    ) -> aptos_storage_service_types::Result<DataResponse, Error> {
        let response = self.storage.get_transactions_with_proof(
            request.proof_version,
            request.start_version,
            request.end_version,
            request.include_events,
        )?;

        Ok(DataResponse::TransactionsWithProof(
            response.transaction_list_with_proof.unwrap().into_parts().0,
        ))
    }

    fn get_transactions_or_outputs_with_proof(
        &self,
        request: &TransactionsOrOutputsWithProofRequest,
    ) -> aptos_storage_service_types::Result<DataResponse, Error> {
        let response = self.storage.get_transactions_or_outputs_with_proof(
            request.proof_version,
            request.start_version,
            request.end_version,
            request.include_events,
            request.max_num_output_reductions,
        )?;

        Ok(DataResponse::TransactionsOrOutputsWithProof((
            response
                .transaction_list_with_proof
                .map(|t| t.into_parts().0),
            response
                .transaction_output_list_with_proof
                .map(|t| t.into_parts().0),
        )))
    }

    fn get_transaction_data_with_proof(
        &self,
        request: &GetTransactionDataWithProofRequest,
    ) -> aptos_storage_service_types::Result<DataResponse, Error> {
        let transaction_data_with_proof = self.storage.get_transaction_data_with_proof(request)?;
        Ok(DataResponse::TransactionDataWithProof(
            transaction_data_with_proof,
        ))
    }
}

/// Updates the subscription metrics with a created subscription stream event
fn update_created_stream_metrics(peer_network_id: &PeerNetworkId) {
    increment_counter(
        &metrics::SUBSCRIPTION_EVENTS,
        peer_network_id.network_id(),
        SUBSCRIPTION_NEW_STREAM.into(),
    );
}

/// Updates the subscription metrics with a failed stream request
fn update_failed_subscription_metrics(peer_network_id: PeerNetworkId) {
    increment_counter(
        &metrics::SUBSCRIPTION_EVENTS,
        peer_network_id.network_id(),
        SUBSCRIPTION_FAILURE.into(),
    );
}

/// Updates the subscription metrics with a new stream request
fn update_new_subscription_metrics(peer_network_id: PeerNetworkId) {
    increment_counter(
        &metrics::SUBSCRIPTION_EVENTS,
        peer_network_id.network_id(),
        SUBSCRIPTION_ADD.into(),
    );
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
