// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::Error,
    global_summary::GlobalDataSummary,
    interface::{
        AptosDataClientInterface, Response, ResponseCallback, ResponseContext, ResponseError,
        ResponseId,
    },
    logging::{LogEntry, LogEvent, LogSchema},
    metrics,
    metrics::{
        increment_request_counter, set_gauge, start_request_timer, PRIORITIZED_PEER, REGULAR_PEER,
    },
    peer_states::{ErrorType, PeerStates},
    poller::DataSummaryPoller,
};
use aptos_config::{
    config::{AptosDataClientConfig, BaseConfig},
    network_id::PeerNetworkId,
};
use aptos_id_generator::{IdGenerator, U64IdGenerator};
use aptos_infallible::RwLock;
use aptos_logger::{debug, info, sample, sample::SampleRate, trace, warn};
use aptos_network::{application::interface::NetworkClient, protocols::network::RpcError};
use aptos_storage_interface::DbReader;
use aptos_storage_service_client::StorageServiceClient;
use aptos_storage_service_types::{
    requests::{
        DataRequest, EpochEndingLedgerInfoRequest, NewTransactionOutputsWithProofRequest,
        NewTransactionsOrOutputsWithProofRequest, NewTransactionsWithProofRequest,
        StateValuesWithProofRequest, StorageServiceRequest, TransactionOutputsWithProofRequest,
        TransactionsOrOutputsWithProofRequest, TransactionsWithProofRequest,
    },
    responses::{StorageServerSummary, StorageServiceResponse, TransactionOrOutputListWithProof},
    Epoch, StorageServiceMessage,
};
use aptos_time_service::TimeService;
use aptos_types::{
    epoch_change::EpochChangeProof,
    ledger_info::LedgerInfoWithSignatures,
    state_store::state_value::StateValueChunkWithProof,
    transaction::{TransactionListWithProof, TransactionOutputListWithProof, Version},
};
use async_trait::async_trait;
use rand::prelude::SliceRandom;
use std::{fmt, sync::Arc, time::Duration};
use tokio::runtime::Handle;

// Useful constants
const IN_FLIGHT_METRICS_SAMPLE_FREQ: u64 = 5;
const PEER_LOG_FREQ_SECS: u64 = 10;

/// An [`AptosDataClientInterface`] that fulfills requests from remote peers' Storage Service
/// over AptosNet.
///
/// The `AptosDataClient`:
///
/// 1. Sends requests to connected Aptos peers.
/// 2. Does basic type conversions and error handling on the responses.
/// 3. Routes requests to peers that advertise availability for that data.
/// 4. Maintains peer scores based on each peer's observed quality of service
///    and upper client reports of invalid or malicious data.
/// 5. Selects high quality peers to send each request to.
/// 6. Exposes a condensed data summary of our peers' data advertisements.
///
/// The client currently assumes 1-request => 1-response. Streaming responses
/// are handled at an upper layer.
///
/// The client is expected to be cloneable and usable from many concurrent tasks
/// and/or threads.
#[derive(Clone, Debug)]
pub struct AptosDataClient {
    /// Config for AptosNet data client.
    data_client_config: AptosDataClientConfig,
    /// The underlying AptosNet storage service client.
    storage_service_client: StorageServiceClient<NetworkClient<StorageServiceMessage>>,
    /// All of the data-client specific data we have on each network peer.
    peer_states: Arc<RwLock<PeerStates>>,
    /// A cached, aggregate data summary of all unbanned peers' data summaries.
    global_summary_cache: Arc<RwLock<GlobalDataSummary>>,
    /// Used for generating the next request/response id.
    response_id_generator: Arc<U64IdGenerator>,
}

impl AptosDataClient {
    pub fn new(
        data_client_config: AptosDataClientConfig,
        base_config: BaseConfig,
        time_service: TimeService,
        storage: Arc<dyn DbReader>,
        storage_service_client: StorageServiceClient<NetworkClient<StorageServiceMessage>>,
        runtime: Option<Handle>,
    ) -> (Self, DataSummaryPoller) {
        // Create the data client
        let data_client = Self {
            data_client_config,
            storage_service_client: storage_service_client.clone(),
            peer_states: Arc::new(RwLock::new(PeerStates::new(
                base_config,
                data_client_config,
                storage_service_client.get_peers_and_metadata(),
            ))),
            global_summary_cache: Arc::new(RwLock::new(GlobalDataSummary::empty())),
            response_id_generator: Arc::new(U64IdGenerator::new()),
        };

        // Create the data summary poller
        let data_summary_poller = DataSummaryPoller::new(
            data_client_config,
            data_client.clone(),
            Duration::from_millis(data_client.data_client_config.summary_poll_loop_interval_ms),
            runtime,
            storage,
            time_service,
        );

        (data_client, data_summary_poller)
    }

    /// Returns true iff compression should be requested
    pub fn use_compression(&self) -> bool {
        self.data_client_config.use_compression
    }

    /// Returns the response timeout in milliseconds
    pub fn get_response_timeout_ms(&self) -> u64 {
        self.data_client_config.response_timeout_ms
    }

    /// Returns the max number of output reductions as defined by the config
    fn get_max_num_output_reductions(&self) -> u64 {
        self.data_client_config.max_num_output_reductions
    }

    /// Generates a new response id
    fn next_response_id(&self) -> u64 {
        self.response_id_generator.next()
    }

    /// Update a peer's data summary.
    pub fn update_summary(&self, peer: PeerNetworkId, summary: StorageServerSummary) {
        self.peer_states.write().update_summary(peer, summary)
    }

    /// Recompute and update the global data summary cache
    pub fn update_global_summary_cache(&self) -> crate::error::Result<(), Error> {
        // Before calculating the summary, we should garbage collect
        // the peer states (to handle disconnected peers).
        self.garbage_collect_peer_states()?;

        // Calculate the aggregate data summary
        let aggregate = self.peer_states.read().calculate_aggregate_summary();
        *self.global_summary_cache.write() = aggregate;

        Ok(())
    }

    /// Garbage collects the peer states to remove data for disconnected peers
    fn garbage_collect_peer_states(&self) -> crate::error::Result<(), Error> {
        // Get all connected peers
        let all_connected_peers = self.get_all_connected_peers()?;

        // Garbage collect the disconnected peers
        self.peer_states
            .write()
            .garbage_collect_peer_states(all_connected_peers);

        Ok(())
    }

    /// Choose a connected peer that can service the given request.
    /// Returns an error if no such peer can be found.
    pub(crate) fn choose_peer_for_request(
        &self,
        request: &StorageServiceRequest,
    ) -> crate::error::Result<PeerNetworkId, Error> {
        // All requests should be sent to prioritized peers (if possible).
        // If none can handle the request, fall back to the regular peers.
        let (priority_peers, regular_peers) = self.get_priority_and_regular_peers()?;
        let priority_serviceable = self.identify_serviceable(priority_peers, request);
        let serviceable_peers = if !priority_serviceable.is_empty() {
            priority_serviceable
        } else {
            self.identify_serviceable(regular_peers, request)
        };

        // Randomly select a peer to handle the request
        serviceable_peers
            .choose(&mut rand::thread_rng())
            .copied()
            .ok_or_else(|| {
                Error::DataIsUnavailable(
                    format!("No connected peers are advertising that they can serve this data! Request: {:?}",request),
                )
            })
    }

    /// Identifies the peers in the given set of prospective peers
    /// that can service the specified request.
    fn identify_serviceable(
        &self,
        prospective_peers: Vec<PeerNetworkId>,
        request: &StorageServiceRequest,
    ) -> Vec<PeerNetworkId> {
        prospective_peers
            .into_iter()
            .filter(|peer| self.peer_states.read().can_service_request(peer, request))
            .collect::<Vec<_>>()
    }

    /// Fetches the next prioritized peer to poll
    pub fn fetch_prioritized_peer_to_poll(
        &self,
    ) -> crate::error::Result<Option<PeerNetworkId>, Error> {
        // Fetch the number of in-flight polls and update the metrics
        let num_in_flight_polls = self.peer_states.read().num_in_flight_priority_polls();
        update_in_flight_metrics(PRIORITIZED_PEER, num_in_flight_polls);

        // Ensure we don't go over the maximum number of in-flight polls
        if num_in_flight_polls >= self.data_client_config.max_num_in_flight_priority_polls {
            return Ok(None);
        }

        // Select a priority peer to poll
        let (priority_connected_peers, _) = self.get_priority_and_regular_peers()?;
        self.select_peer_to_poll(priority_connected_peers)
    }

    /// Fetches the next regular peer to poll
    pub fn fetch_regular_peer_to_poll(&self) -> crate::error::Result<Option<PeerNetworkId>, Error> {
        // Fetch the number of in-flight polls and update the metrics
        let num_in_flight_polls = self.peer_states.read().num_in_flight_regular_polls();
        update_in_flight_metrics(REGULAR_PEER, num_in_flight_polls);

        // Ensure we don't go over the maximum number of in-flight polls
        if num_in_flight_polls >= self.data_client_config.max_num_in_flight_regular_polls {
            return Ok(None);
        }

        // Select a regular peer to poll
        let (_, regular_connected_peers) = self.get_priority_and_regular_peers()?;
        self.select_peer_to_poll(regular_connected_peers)
    }

    /// Randomly selects a peer to poll that does not have an in-flight request
    fn select_peer_to_poll(
        &self,
        mut peers: Vec<PeerNetworkId>,
    ) -> crate::error::Result<Option<PeerNetworkId>, Error> {
        // Identify the peers who do not already have in-flight requests.
        peers.retain(|peer| !self.peer_states.read().existing_in_flight_request(peer));

        // Select a peer at random for polling
        let peer_to_poll = peers.choose(&mut rand::thread_rng());
        Ok(peer_to_poll.cloned())
    }

    /// Marks the given peers as having an in-flight poll request
    pub fn in_flight_request_started(&self, peer: &PeerNetworkId) {
        self.peer_states.write().new_in_flight_request(peer);
    }

    /// Marks the given peers as polled
    pub fn in_flight_request_complete(&self, peer: &PeerNetworkId) {
        self.peer_states
            .write()
            .mark_in_flight_request_complete(peer);
    }

    /// Returns all peers connected to us
    fn get_all_connected_peers(&self) -> crate::error::Result<Vec<PeerNetworkId>, Error> {
        let connected_peers = self.storage_service_client.get_available_peers()?;
        if connected_peers.is_empty() {
            return Err(Error::DataIsUnavailable(
                "No connected AptosNet peers!".to_owned(),
            ));
        }

        Ok(connected_peers)
    }

    /// Returns all priority and regular peers
    pub(crate) fn get_priority_and_regular_peers(
        &self,
    ) -> crate::error::Result<(Vec<PeerNetworkId>, Vec<PeerNetworkId>), Error> {
        // Get all connected peers
        let all_connected_peers = self.get_all_connected_peers()?;

        // Filter the peers based on priority
        let mut priority_peers = vec![];
        let mut regular_peers = vec![];
        for peer in all_connected_peers {
            if self.peer_states.read().is_priority_peer(&peer) {
                priority_peers.push(peer);
            } else {
                regular_peers.push(peer);
            }
        }

        // Log the peers, periodically.
        sample!(
            SampleRate::Duration(Duration::from_secs(PEER_LOG_FREQ_SECS)),
            update_connected_peer_metrics(priority_peers.len(), regular_peers.len());
        );

        Ok((priority_peers, regular_peers))
    }

    /// Sends a request (to an undecided peer) and decodes the response
    async fn send_request_and_decode<T, E>(
        &self,
        request: StorageServiceRequest,
        request_timeout_ms: u64,
    ) -> crate::error::Result<Response<T>>
    where
        T: TryFrom<StorageServiceResponse, Error = E>,
        E: Into<Error>,
    {
        let peer = self.choose_peer_for_request(&request).map_err(|error| {
            debug!(
                (LogSchema::new(LogEntry::StorageServiceRequest)
                    .event(LogEvent::PeerSelectionError)
                    .message("Unable to select peer")
                    .error(&error))
            );
            error
        })?;
        let _timer = start_request_timer(&metrics::REQUEST_LATENCIES, &request.get_label(), peer);
        self.send_request_to_peer_and_decode(peer, request, request_timeout_ms)
            .await
    }

    /// Sends a request to a specific peer and decodes the response
    pub async fn send_request_to_peer_and_decode<T, E>(
        &self,
        peer: PeerNetworkId,
        request: StorageServiceRequest,
        request_timeout_ms: u64,
    ) -> crate::error::Result<Response<T>>
    where
        T: TryFrom<StorageServiceResponse, Error = E>,
        E: Into<Error>,
    {
        let response = self
            .send_request_to_peer(peer, request.clone(), request_timeout_ms)
            .await?;

        let (context, storage_response) = response.into_parts();

        // Ensure the response obeys the compression requirements
        if request.use_compression && !storage_response.is_compressed() {
            return Err(Error::InvalidResponse(format!(
                "Requested compressed data, but the response was uncompressed! Response: {:?}",
                storage_response.get_label()
            )));
        } else if !request.use_compression && storage_response.is_compressed() {
            return Err(Error::InvalidResponse(format!(
                "Requested uncompressed data, but the response was compressed! Response: {:?}",
                storage_response.get_label()
            )));
        }

        // try to convert the storage service enum into the exact variant we're expecting.
        match T::try_from(storage_response) {
            Ok(new_payload) => Ok(Response::new(context, new_payload)),
            // if the variant doesn't match what we're expecting, report the issue.
            Err(err) => {
                context
                    .response_callback
                    .notify_bad_response(ResponseError::InvalidPayloadDataType);
                Err(err.into())
            },
        }
    }

    /// Sends a request to a specific peer
    async fn send_request_to_peer(
        &self,
        peer: PeerNetworkId,
        request: StorageServiceRequest,
        request_timeout_ms: u64,
    ) -> crate::error::Result<Response<StorageServiceResponse>, Error> {
        let id = self.next_response_id();
        trace!(
            (LogSchema::new(LogEntry::StorageServiceRequest)
                .event(LogEvent::SendRequest)
                .request_type(&request.get_label())
                .request_id(id)
                .peer(&peer)
                .request_data(&request))
        );
        increment_request_counter(&metrics::SENT_REQUESTS, &request.get_label(), peer);

        // Send the request and process the result
        let result = self
            .storage_service_client
            .send_request(
                peer,
                Duration::from_millis(request_timeout_ms),
                request.clone(),
            )
            .await;
        match result {
            Ok(response) => {
                trace!(
                    (LogSchema::new(LogEntry::StorageServiceResponse)
                        .event(LogEvent::ResponseSuccess)
                        .request_type(&request.get_label())
                        .request_id(id)
                        .peer(&peer))
                );

                increment_request_counter(&metrics::SUCCESS_RESPONSES, &request.get_label(), peer);

                // For now, record all responses that at least pass the data
                // client layer successfully. An alternative might also have the
                // consumer notify both success and failure via the callback.
                // On the one hand, scoring dynamics are simpler when each request
                // is successful or failed but not both; on the other hand, this
                // feels simpler for the consumer.
                self.peer_states.write().update_score_success(peer);

                // Package up all of the context needed to fully report an error
                // with this RPC.
                let response_callback = AptosNetResponseCallback {
                    data_client: self.clone(),
                    id,
                    peer,
                    request,
                };
                let context = ResponseContext {
                    id,
                    response_callback: Box::new(response_callback),
                };
                Ok(Response::new(context, response))
            },
            Err(error) => {
                // Convert network error and storage service error types into
                // data client errors. Also categorize the error type for scoring
                // purposes.
                let client_error = match error {
                    aptos_storage_service_client::Error::RpcError(rpc_error) => match rpc_error {
                        RpcError::NotConnected(_) => {
                            Error::DataIsUnavailable(rpc_error.to_string())
                        },
                        RpcError::TimedOut => {
                            Error::TimeoutWaitingForResponse(rpc_error.to_string())
                        },
                        _ => Error::UnexpectedErrorEncountered(rpc_error.to_string()),
                    },
                    aptos_storage_service_client::Error::StorageServiceError(err) => {
                        Error::UnexpectedErrorEncountered(err.to_string())
                    },
                    _ => Error::UnexpectedErrorEncountered(error.to_string()),
                };

                warn!(
                    (LogSchema::new(LogEntry::StorageServiceResponse)
                        .event(LogEvent::ResponseError)
                        .request_type(&request.get_label())
                        .request_id(id)
                        .peer(&peer)
                        .error(&client_error))
                );

                increment_request_counter(
                    &metrics::ERROR_RESPONSES,
                    client_error.get_label(),
                    peer,
                );

                self.notify_bad_response(id, peer, &request, ErrorType::NotUseful);
                Err(client_error)
            },
        }
    }

    /// Updates the score of the peer who sent the response with the specified id
    fn notify_bad_response(
        &self,
        _id: ResponseId,
        peer: PeerNetworkId,
        _request: &StorageServiceRequest,
        error_type: ErrorType,
    ) {
        self.peer_states
            .write()
            .update_score_error(peer, error_type);
    }

    /// Creates a storage service request using the given data request
    /// and sends it across the network
    async fn create_and_send_storage_request<T, E>(
        &self,
        request_timeout_ms: u64,
        data_request: DataRequest,
    ) -> crate::error::Result<Response<T>>
    where
        T: TryFrom<StorageServiceResponse, Error = E>,
        E: Into<Error>,
    {
        let storage_request = StorageServiceRequest::new(data_request, self.use_compression());
        self.send_request_and_decode(storage_request, request_timeout_ms)
            .await
    }

    /// Returns a copy of the peer states for testing
    #[cfg(test)]
    pub(crate) fn get_peer_states(&self) -> PeerStates {
        self.peer_states.read().clone()
    }
}

#[async_trait]
impl AptosDataClientInterface for AptosDataClient {
    fn get_global_data_summary(&self) -> GlobalDataSummary {
        self.global_summary_cache.read().clone()
    }

    async fn get_epoch_ending_ledger_infos(
        &self,
        start_epoch: Epoch,
        expected_end_epoch: Epoch,
        request_timeout_ms: u64,
    ) -> crate::error::Result<Response<Vec<LedgerInfoWithSignatures>>> {
        let data_request = DataRequest::GetEpochEndingLedgerInfos(EpochEndingLedgerInfoRequest {
            start_epoch,
            expected_end_epoch,
        });
        let response: Response<EpochChangeProof> = self
            .create_and_send_storage_request(request_timeout_ms, data_request)
            .await?;
        Ok(response.map(|epoch_change| epoch_change.ledger_info_with_sigs))
    }

    async fn get_new_transaction_outputs_with_proof(
        &self,
        known_version: Version,
        known_epoch: Epoch,
        request_timeout_ms: u64,
    ) -> crate::error::Result<Response<(TransactionOutputListWithProof, LedgerInfoWithSignatures)>>
    {
        let data_request =
            DataRequest::GetNewTransactionOutputsWithProof(NewTransactionOutputsWithProofRequest {
                known_version,
                known_epoch,
            });
        self.create_and_send_storage_request(request_timeout_ms, data_request)
            .await
    }

    async fn get_new_transactions_with_proof(
        &self,
        known_version: Version,
        known_epoch: Epoch,
        include_events: bool,
        request_timeout_ms: u64,
    ) -> crate::error::Result<Response<(TransactionListWithProof, LedgerInfoWithSignatures)>> {
        let data_request =
            DataRequest::GetNewTransactionsWithProof(NewTransactionsWithProofRequest {
                known_version,
                known_epoch,
                include_events,
            });
        self.create_and_send_storage_request(request_timeout_ms, data_request)
            .await
    }

    async fn get_new_transactions_or_outputs_with_proof(
        &self,
        known_version: Version,
        known_epoch: Epoch,
        include_events: bool,
        request_timeout_ms: u64,
    ) -> crate::error::Result<Response<(TransactionOrOutputListWithProof, LedgerInfoWithSignatures)>>
    {
        let data_request = DataRequest::GetNewTransactionsOrOutputsWithProof(
            NewTransactionsOrOutputsWithProofRequest {
                known_version,
                known_epoch,
                include_events,
                max_num_output_reductions: self.get_max_num_output_reductions(),
            },
        );
        self.create_and_send_storage_request(request_timeout_ms, data_request)
            .await
    }

    async fn get_number_of_states(
        &self,
        version: Version,
        request_timeout_ms: u64,
    ) -> crate::error::Result<Response<u64>> {
        let data_request = DataRequest::GetNumberOfStatesAtVersion(version);
        self.create_and_send_storage_request(request_timeout_ms, data_request)
            .await
    }

    async fn get_state_values_with_proof(
        &self,
        version: u64,
        start_index: u64,
        end_index: u64,
        request_timeout_ms: u64,
    ) -> crate::error::Result<Response<StateValueChunkWithProof>> {
        let data_request = DataRequest::GetStateValuesWithProof(StateValuesWithProofRequest {
            version,
            start_index,
            end_index,
        });
        self.create_and_send_storage_request(request_timeout_ms, data_request)
            .await
    }

    async fn get_transaction_outputs_with_proof(
        &self,
        proof_version: Version,
        start_version: Version,
        end_version: Version,
        request_timeout_ms: u64,
    ) -> crate::error::Result<Response<TransactionOutputListWithProof>> {
        let data_request =
            DataRequest::GetTransactionOutputsWithProof(TransactionOutputsWithProofRequest {
                proof_version,
                start_version,
                end_version,
            });
        self.create_and_send_storage_request(request_timeout_ms, data_request)
            .await
    }

    async fn get_transactions_with_proof(
        &self,
        proof_version: Version,
        start_version: Version,
        end_version: Version,
        include_events: bool,
        request_timeout_ms: u64,
    ) -> crate::error::Result<Response<TransactionListWithProof>> {
        let data_request = DataRequest::GetTransactionsWithProof(TransactionsWithProofRequest {
            proof_version,
            start_version,
            end_version,
            include_events,
        });
        self.create_and_send_storage_request(request_timeout_ms, data_request)
            .await
    }

    async fn get_transactions_or_outputs_with_proof(
        &self,
        proof_version: Version,
        start_version: Version,
        end_version: Version,
        include_events: bool,
        request_timeout_ms: u64,
    ) -> crate::error::Result<Response<TransactionOrOutputListWithProof>> {
        let data_request =
            DataRequest::GetTransactionsOrOutputsWithProof(TransactionsOrOutputsWithProofRequest {
                proof_version,
                start_version,
                end_version,
                include_events,
                max_num_output_reductions: self.get_max_num_output_reductions(),
            });
        self.create_and_send_storage_request(request_timeout_ms, data_request)
            .await
    }
}

/// The AptosNet-specific request context needed to update a peer's scoring.
struct AptosNetResponseCallback {
    data_client: AptosDataClient,
    id: ResponseId,
    peer: PeerNetworkId,
    request: StorageServiceRequest,
}

impl ResponseCallback for AptosNetResponseCallback {
    fn notify_bad_response(&self, error: ResponseError) {
        let error_type = ErrorType::from(error);
        self.data_client
            .notify_bad_response(self.id, self.peer, &self.request, error_type);
    }
}

impl fmt::Debug for AptosNetResponseCallback {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AptosNetResponseCallback")
            .field("data_client", &"..")
            .field("id", &self.id)
            .field("peer", &self.peer)
            .field("request", &self.request)
            .finish()
    }
}

/// Updates the metrics for the number of connected peers (priority and regular)
fn update_connected_peer_metrics(num_priority_peers: usize, num_regular_peers: usize) {
    // Log the number of connected peers
    info!(
        (LogSchema::new(LogEntry::PeerStates)
            .event(LogEvent::PriorityAndRegularPeers)
            .message(&format!(
                "Number of priority peers: {:?}. Number of regular peers: {:?}",
                num_priority_peers, num_regular_peers,
            )))
    );

    // Update the connected peer metrics
    set_gauge(
        &metrics::CONNECTED_PEERS,
        PRIORITIZED_PEER,
        num_priority_peers as u64,
    );
    set_gauge(
        &metrics::CONNECTED_PEERS,
        REGULAR_PEER,
        num_regular_peers as u64,
    );
}

/// Updates the metrics for the number of in-flight polls
fn update_in_flight_metrics(label: &str, num_in_flight_polls: u64) {
    sample!(
        SampleRate::Frequency(IN_FLIGHT_METRICS_SAMPLE_FREQ),
        set_gauge(
            &metrics::IN_FLIGHT_POLLS,
            label,
            num_in_flight_polls,
        );
    );
}
