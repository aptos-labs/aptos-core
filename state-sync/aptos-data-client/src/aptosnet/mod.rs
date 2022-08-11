// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    aptosnet::{
        logging::{LogEntry, LogEvent, LogSchema},
        metrics::{
            increment_request_counter, set_gauge, start_request_timer, DataType, PRIORITIZED_PEER,
            REGULAR_PEER,
        },
        state::{ErrorType, PeerStates},
    },
    AptosDataClient, Error, GlobalDataSummary, Response, ResponseCallback, ResponseContext,
    ResponseError, ResponseId, Result,
};
use aptos_config::{
    config::{AptosDataClientConfig, BaseConfig, StorageServiceConfig},
    network_id::PeerNetworkId,
};
use aptos_id_generator::{IdGenerator, U64IdGenerator};
use aptos_infallible::RwLock;
use aptos_logger::prelude::*;
use aptos_time_service::{TimeService, TimeServiceTrait};
use aptos_types::{
    epoch_change::EpochChangeProof,
    ledger_info::LedgerInfoWithSignatures,
    state_store::state_value::StateValueChunkWithProof,
    transaction::{TransactionListWithProof, TransactionOutputListWithProof, Version},
};
use async_trait::async_trait;
use futures::StreamExt;
use network::{
    application::interface::NetworkInterface,
    protocols::{rpc::error::RpcError, wire::handshake::v1::ProtocolId},
};
use rand::seq::SliceRandom;
use std::{convert::TryFrom, fmt, sync::Arc, time::Duration};
use storage_service_client::StorageServiceClient;
use storage_service_types::requests::{
    DataRequest, EpochEndingLedgerInfoRequest, NewTransactionOutputsWithProofRequest,
    NewTransactionsWithProofRequest, StateValuesWithProofRequest, StorageServiceRequest,
    TransactionOutputsWithProofRequest, TransactionsWithProofRequest,
};
use storage_service_types::responses::{StorageServerSummary, StorageServiceResponse};
use storage_service_types::Epoch;
use tokio::{runtime::Handle, task::JoinHandle};

mod logging;
mod metrics;
mod state;
#[cfg(test)]
mod tests;

// TODO(joshlind): this code needs to be restructured. There are no clear APIs
// and little separation between components.

// Useful constants for the Aptos Data Client
const GLOBAL_DATA_LOG_FREQ_SECS: u64 = 10;
const GLOBAL_DATA_METRIC_FREQ_SECS: u64 = 1;
const IN_FLIGHT_METRICS_SAMPLE_FREQ: u64 = 5;
const PEER_LOG_FREQ_SECS: u64 = 10;
const POLLER_LOG_FREQ_SECS: u64 = 2;
const REGULAR_PEER_SAMPLE_FREQ: u64 = 3;

/// An [`AptosDataClient`] that fulfills requests from remote peers' Storage Service
/// over AptosNet.
///
/// The `AptosNetDataClient`:
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
pub struct AptosNetDataClient {
    /// Config for AptosNet data client.
    data_client_config: AptosDataClientConfig,
    /// The underlying AptosNet storage service client.
    network_client: StorageServiceClient,
    /// All of the data-client specific data we have on each network peer.
    peer_states: Arc<RwLock<PeerStates>>,
    /// A cached, aggregate data summary of all unbanned peers' data summaries.
    global_summary_cache: Arc<RwLock<GlobalDataSummary>>,
    /// Used for generating the next request/response id.
    response_id_generator: Arc<U64IdGenerator>,
}

impl AptosNetDataClient {
    pub fn new(
        data_client_config: AptosDataClientConfig,
        base_config: BaseConfig,
        storage_service_config: StorageServiceConfig,
        time_service: TimeService,
        network_client: StorageServiceClient,
        runtime: Option<Handle>,
    ) -> (Self, DataSummaryPoller) {
        let client = Self {
            data_client_config,
            network_client: network_client.clone(),
            peer_states: Arc::new(RwLock::new(PeerStates::new(
                base_config,
                storage_service_config,
                network_client.get_peer_metadata_storage(),
            ))),
            global_summary_cache: Arc::new(RwLock::new(GlobalDataSummary::empty())),
            response_id_generator: Arc::new(U64IdGenerator::new()),
        };
        let poller = DataSummaryPoller::new(
            client.clone(),
            Duration::from_millis(client.data_client_config.summary_poll_interval_ms),
            runtime,
            time_service,
        );
        (client, poller)
    }

    /// Returns true iff compression should be requested
    fn use_compression(&self) -> bool {
        self.data_client_config.use_compression
    }

    /// Generates a new response id
    fn next_response_id(&self) -> u64 {
        self.response_id_generator.next()
    }

    /// Update a peer's data summary.
    fn update_summary(&self, peer: PeerNetworkId, summary: StorageServerSummary) {
        self.peer_states.write().update_summary(peer, summary)
    }

    /// Recompute and update the global data summary cache.
    fn update_global_summary_cache(&self) {
        let aggregate = self.peer_states.read().calculate_aggregate_summary();
        *self.global_summary_cache.write() = aggregate;
    }

    /// Choose a connected peer that can service the given request.
    /// Returns an error if no such peer can be found.
    fn choose_peer_for_request(
        &self,
        request: &StorageServiceRequest,
    ) -> Result<PeerNetworkId, Error> {
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
    fn fetch_prioritized_peer_to_poll(&self) -> Result<Option<PeerNetworkId>, Error> {
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
    fn fetch_regular_peer_to_poll(&self) -> Result<Option<PeerNetworkId>, Error> {
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
    ) -> Result<Option<PeerNetworkId>, Error> {
        // Identify the peers who do not already have in-flight requests.
        peers.retain(|peer| !self.peer_states.read().existing_in_flight_request(peer));

        // Select a peer at random for polling
        let peer_to_poll = peers.choose(&mut rand::thread_rng());
        Ok(peer_to_poll.cloned())
    }

    /// Marks the given peers as having an in-flight poll request
    fn in_flight_request_started(&self, peer: &PeerNetworkId) {
        self.peer_states.write().new_in_flight_request(peer);
    }

    /// Marks the given peers as polled
    fn in_flight_request_complete(&self, peer: &PeerNetworkId) {
        self.peer_states
            .write()
            .mark_in_flight_request_complete(peer);
    }

    /// Returns all peers connected to us
    fn get_all_connected_peers(&self) -> Result<Vec<PeerNetworkId>, Error> {
        let network_peer_metadata = self.network_client.peer_metadata_storage();
        let connected_peers = network_peer_metadata
            .networks()
            .flat_map(|network_id| {
                network_peer_metadata
                    .read_filtered(network_id, |(_, peer_metadata)| {
                        peer_metadata.is_connected()
                            && peer_metadata.supports_protocol(ProtocolId::StorageServiceRpc)
                    })
                    .into_keys()
            })
            .collect::<Vec<_>>();

        // Ensure connected peers is not empty
        if connected_peers.is_empty() {
            return Err(Error::DataIsUnavailable(
                "No connected AptosNet peers!".to_owned(),
            ));
        }
        Ok(connected_peers)
    }

    /// Returns all priority and regular peers
    fn get_priority_and_regular_peers(
        &self,
    ) -> Result<(Vec<PeerNetworkId>, Vec<PeerNetworkId>), Error> {
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
    ) -> Result<Response<T>>
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
        self.send_request_to_peer_and_decode(peer, request).await
    }

    /// Sends a request to a specific peer and decodes the response
    async fn send_request_to_peer_and_decode<T, E>(
        &self,
        peer: PeerNetworkId,
        request: StorageServiceRequest,
    ) -> Result<Response<T>>
    where
        T: TryFrom<StorageServiceResponse, Error = E>,
        E: Into<Error>,
    {
        let response = self.send_request_to_peer(peer, request.clone()).await?;

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
            }
        }
    }

    /// Sends a request to a specific peer
    async fn send_request_to_peer(
        &self,
        peer: PeerNetworkId,
        request: StorageServiceRequest,
    ) -> Result<Response<StorageServiceResponse>, Error> {
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

        let result = self
            .network_client
            .send_request(
                peer,
                request.clone(),
                Duration::from_millis(self.data_client_config.response_timeout_ms),
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
            }
            Err(error) => {
                // Convert network error and storage service error types into
                // data client errors. Also categorize the error type for scoring
                // purposes.
                let client_error = match error {
                    storage_service_client::Error::RpcError(err) => match err {
                        RpcError::NotConnected(_) => Error::DataIsUnavailable(err.to_string()),
                        RpcError::TimedOut => Error::TimeoutWaitingForResponse(err.to_string()),
                        _ => Error::UnexpectedErrorEncountered(err.to_string()),
                    },
                    storage_service_client::Error::StorageServiceError(err) => {
                        Error::UnexpectedErrorEncountered(err.to_string())
                    }
                };

                error!(
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
            }
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
}

#[async_trait]
impl AptosDataClient for AptosNetDataClient {
    fn get_global_data_summary(&self) -> GlobalDataSummary {
        self.global_summary_cache.read().clone()
    }

    async fn get_epoch_ending_ledger_infos(
        &self,
        start_epoch: Epoch,
        expected_end_epoch: Epoch,
    ) -> Result<Response<Vec<LedgerInfoWithSignatures>>> {
        let data_request = DataRequest::GetEpochEndingLedgerInfos(EpochEndingLedgerInfoRequest {
            start_epoch,
            expected_end_epoch,
        });
        let storage_request = StorageServiceRequest::new(data_request, self.use_compression());
        let response: Response<EpochChangeProof> =
            self.send_request_and_decode(storage_request).await?;
        Ok(response.map(|epoch_change| epoch_change.ledger_info_with_sigs))
    }

    async fn get_new_transaction_outputs_with_proof(
        &self,
        known_version: Version,
        known_epoch: Epoch,
    ) -> Result<Response<(TransactionOutputListWithProof, LedgerInfoWithSignatures)>> {
        let data_request =
            DataRequest::GetNewTransactionOutputsWithProof(NewTransactionOutputsWithProofRequest {
                known_version,
                known_epoch,
            });
        let storage_request = StorageServiceRequest::new(data_request, self.use_compression());
        self.send_request_and_decode(storage_request).await
    }

    async fn get_new_transactions_with_proof(
        &self,
        known_version: Version,
        known_epoch: Epoch,
        include_events: bool,
    ) -> Result<Response<(TransactionListWithProof, LedgerInfoWithSignatures)>> {
        let data_request =
            DataRequest::GetNewTransactionsWithProof(NewTransactionsWithProofRequest {
                known_version,
                known_epoch,
                include_events,
            });
        let storage_request = StorageServiceRequest::new(data_request, self.use_compression());
        self.send_request_and_decode(storage_request).await
    }

    async fn get_number_of_states(&self, version: Version) -> Result<Response<u64>> {
        let data_request = DataRequest::GetNumberOfStatesAtVersion(version);
        let storage_request = StorageServiceRequest::new(data_request, self.use_compression());
        self.send_request_and_decode(storage_request).await
    }

    async fn get_state_values_with_proof(
        &self,
        version: u64,
        start_index: u64,
        end_index: u64,
    ) -> Result<Response<StateValueChunkWithProof>> {
        let data_request = DataRequest::GetStateValuesWithProof(StateValuesWithProofRequest {
            version,
            start_index,
            end_index,
        });
        let storage_request = StorageServiceRequest::new(data_request, self.use_compression());
        self.send_request_and_decode(storage_request).await
    }

    async fn get_transaction_outputs_with_proof(
        &self,
        proof_version: Version,
        start_version: Version,
        end_version: Version,
    ) -> Result<Response<TransactionOutputListWithProof>> {
        let data_request =
            DataRequest::GetTransactionOutputsWithProof(TransactionOutputsWithProofRequest {
                proof_version,
                start_version,
                end_version,
            });
        let storage_request = StorageServiceRequest::new(data_request, self.use_compression());
        self.send_request_and_decode(storage_request).await
    }

    async fn get_transactions_with_proof(
        &self,
        proof_version: Version,
        start_version: Version,
        end_version: Version,
        include_events: bool,
    ) -> Result<Response<TransactionListWithProof>> {
        let data_request = DataRequest::GetTransactionsWithProof(TransactionsWithProofRequest {
            proof_version,
            start_version,
            end_version,
            include_events,
        });
        let storage_request = StorageServiceRequest::new(data_request, self.use_compression());
        self.send_request_and_decode(storage_request).await
    }
}

/// The AptosNet-specific request context needed to update a peer's scoring.
struct AptosNetResponseCallback {
    data_client: AptosNetDataClient,
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

/// A poller for storage summaries that is responsible for periodically refreshing
/// the view of advertised data in the network.
pub struct DataSummaryPoller {
    data_client: AptosNetDataClient, // The data client through which to poll peers
    poll_interval: Duration,         // The interval between polling rounds
    runtime: Option<Handle>,         // An optional runtime on which to spawn the poller threads
    time_service: TimeService,       // The service to monitor elapsed time
}

impl DataSummaryPoller {
    fn new(
        data_client: AptosNetDataClient,
        poll_interval: Duration,
        runtime: Option<Handle>,
        time_service: TimeService,
    ) -> Self {
        Self {
            data_client,
            poll_interval,
            runtime,
            time_service,
        }
    }

    /// Runs the poller that continuously updates the global data summary
    pub async fn start_poller(self) {
        info!(
            (LogSchema::new(LogEntry::DataSummaryPoller)
                .message("Starting the Aptos data poller!"))
        );
        let ticker = self.time_service.interval(self.poll_interval);
        futures::pin_mut!(ticker);

        loop {
            // Wait for next round before polling
            ticker.next().await;

            // Update the global storage summary
            self.data_client.update_global_summary_cache();

            // Fetch the prioritized and regular peers to poll (if any)
            let prioritized_peer = self.try_fetch_peer(true);
            let regular_peer = self.fetch_regular_peer(prioritized_peer.is_none());

            // Ensure the peers to poll exist
            if prioritized_peer.is_none() && regular_peer.is_none() {
                sample!(
                    SampleRate::Duration(Duration::from_secs(POLLER_LOG_FREQ_SECS)),
                    debug!(
                        (LogSchema::new(LogEntry::StorageSummaryRequest)
                            .event(LogEvent::NoPeersToPoll)
                            .message("No prioritized or regular peers to poll this round!"))
                    );
                );
                continue;
            }

            // Go through each peer and poll them individually
            if let Some(prioritized_peer) = prioritized_peer {
                poll_peer(
                    self.data_client.clone(),
                    prioritized_peer,
                    self.runtime.clone(),
                );
            }
            if let Some(regular_peer) = regular_peer {
                poll_peer(self.data_client.clone(), regular_peer, self.runtime.clone());
            }
        }
    }

    /// Fetches the next regular peer to poll based on the sample frequency
    fn fetch_regular_peer(&self, always_poll: bool) -> Option<PeerNetworkId> {
        if always_poll {
            self.try_fetch_peer(false)
        } else {
            sample!(SampleRate::Frequency(REGULAR_PEER_SAMPLE_FREQ), {
                return self.try_fetch_peer(false);
            });
            None
        }
    }

    /// Attempts to fetch the next peer to poll from the data client.
    /// If an error is encountered, the error is logged and None is returned.
    fn try_fetch_peer(&self, is_priority_peer: bool) -> Option<PeerNetworkId> {
        let result = if is_priority_peer {
            self.data_client.fetch_prioritized_peer_to_poll()
        } else {
            self.data_client.fetch_regular_peer_to_poll()
        };
        result.unwrap_or_else(|error| {
            log_poller_error(error);
            None
        })
    }
}

/// Logs the given poller error based on the logging frequency
fn log_poller_error(error: Error) {
    sample!(
        SampleRate::Duration(Duration::from_secs(POLLER_LOG_FREQ_SECS)),
        warn!(
            (LogSchema::new(LogEntry::StorageSummaryRequest)
                .event(LogEvent::PeerPollingError)
                .message("Unable to fetch peers to poll!")
                .error(&error))
        );
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

/// Spawns a dedicated poller for the given peer.
pub(crate) fn poll_peer(
    data_client: AptosNetDataClient,
    peer: PeerNetworkId,
    runtime: Option<Handle>,
) -> JoinHandle<()> {
    // Mark the in-flight poll as started. We do this here to prevent
    // the main polling loop from selecting the same peer concurrently.
    data_client.in_flight_request_started(&peer);

    // Create the poller for the peer
    let poller = async move {
        // Construct the request for polling
        let data_request = DataRequest::GetStorageServerSummary;
        let storage_request =
            StorageServiceRequest::new(data_request, data_client.use_compression());

        // Start the peer polling timer
        let timer = start_request_timer(
            &metrics::REQUEST_LATENCIES,
            &storage_request.get_label(),
            peer,
        );

        // Fetch the storage summary for the peer and stop the timer
        let result: Result<StorageServerSummary> = data_client
            .send_request_to_peer_and_decode(peer, storage_request)
            .await
            .map(Response::into_payload);
        drop(timer);

        // Mark the in-flight poll as now complete
        data_client.in_flight_request_complete(&peer);

        // Check the storage summary response
        let storage_summary = match result {
            Ok(storage_summary) => storage_summary,
            Err(error) => {
                error!(
                    (LogSchema::new(LogEntry::StorageSummaryResponse)
                        .event(LogEvent::PeerPollingError)
                        .message("Error encountered when polling peer!")
                        .error(&error)
                        .peer(&peer))
                );
                return;
            }
        };

        // Update the summary for the peer
        data_client.update_summary(peer, storage_summary);

        // Log the new global data summary and update the metrics
        sample!(
            SampleRate::Duration(Duration::from_secs(GLOBAL_DATA_LOG_FREQ_SECS)),
            info!(
                (LogSchema::new(LogEntry::PeerStates)
                    .event(LogEvent::AggregateSummary)
                    .message(&format!(
                        "Global data summary: {}",
                        data_client.get_global_data_summary()
                    )))
            );
        );
        sample!(
            SampleRate::Duration(Duration::from_secs(GLOBAL_DATA_METRIC_FREQ_SECS)),
            let global_data_summary = data_client.get_global_data_summary();
            update_advertised_data_metrics(global_data_summary);
        );
    };

    // Spawn the poller
    if let Some(runtime) = runtime {
        runtime.spawn(poller)
    } else {
        tokio::spawn(poller)
    }
}

/// Updates the advertised data metrics using the given global
/// data summary.
fn update_advertised_data_metrics(global_data_summary: GlobalDataSummary) {
    // Update the optimal chunk sizes
    let optimal_chunk_sizes = &global_data_summary.optimal_chunk_sizes;
    for data_type in DataType::get_all_types() {
        let optimal_chunk_size = match data_type {
            DataType::LedgerInfos => optimal_chunk_sizes.epoch_chunk_size,
            DataType::States => optimal_chunk_sizes.state_chunk_size,
            DataType::TransactionOutputs => optimal_chunk_sizes.transaction_output_chunk_size,
            DataType::Transactions => optimal_chunk_sizes.transaction_chunk_size,
        };
        set_gauge(
            &metrics::OPTIMAL_CHUNK_SIZES,
            data_type.as_str(),
            optimal_chunk_size,
        );
    }

    // Update the highest advertised data
    let advertised_data = &global_data_summary.advertised_data;
    let highest_advertised_version = advertised_data
        .highest_synced_ledger_info()
        .map(|ledger_info| ledger_info.ledger_info().version());
    if let Some(highest_advertised_version) = highest_advertised_version {
        for data_type in DataType::get_all_types() {
            set_gauge(
                &metrics::HIGHEST_ADVERTISED_DATA,
                data_type.as_str(),
                highest_advertised_version,
            );
        }
    }

    // Update the lowest advertised data
    for data_type in DataType::get_all_types() {
        let lowest_advertised_version = match data_type {
            DataType::LedgerInfos => Some(0), // All nodes contain all epoch ending ledger infos
            DataType::States => advertised_data.lowest_state_version(),
            DataType::TransactionOutputs => advertised_data.lowest_transaction_output_version(),
            DataType::Transactions => advertised_data.lowest_transaction_version(),
        };
        if let Some(lowest_advertised_version) = lowest_advertised_version {
            set_gauge(
                &metrics::LOWEST_ADVERTISED_DATA,
                data_type.as_str(),
                lowest_advertised_version,
            );
        }
    }
}
