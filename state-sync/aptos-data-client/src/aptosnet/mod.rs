// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    aptosnet::{
        logging::{LogEntry, LogEvent, LogSchema},
        metrics::{increment_counter, start_timer, DataType},
        state::{ErrorType, PeerStates},
    },
    AptosDataClient, Error, GlobalDataSummary, Response, ResponseCallback, ResponseContext,
    ResponseError, ResponseId, Result,
};
use aptos_config::{
    config::{AptosDataClientConfig, StorageServiceConfig},
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
use storage_service_types::{
    AccountStatesChunkWithProofRequest, Epoch, EpochEndingLedgerInfoRequest, StorageServerSummary,
    StorageServiceRequest, StorageServiceResponse, TransactionOutputsWithProofRequest,
    TransactionsWithProofRequest,
};

mod logging;
mod metrics;
mod state;
#[cfg(test)]
mod tests;

// Useful constants for the Aptos Data Client
const GLOBAL_DATA_LOG_FREQ_SECS: u64 = 5;
const GLOBAL_DATA_METRIC_FREQ_SECS: u64 = 1;
const POLLER_ERROR_LOG_FREQ_SECS: u64 = 1;

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
        storage_service_config: StorageServiceConfig,
        time_service: TimeService,
        network_client: StorageServiceClient,
    ) -> (Self, DataSummaryPoller) {
        let client = Self {
            data_client_config,
            network_client,
            peer_states: Arc::new(RwLock::new(PeerStates::new(storage_service_config))),
            global_summary_cache: Arc::new(RwLock::new(GlobalDataSummary::empty())),
            response_id_generator: Arc::new(U64IdGenerator::new()),
        };
        let poller = DataSummaryPoller::new(
            time_service,
            client.clone(),
            Duration::from_millis(client.data_client_config.summary_poll_interval_ms),
        );
        (client, poller)
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

    /// Choose a connected peer that can service the given request. Returns an
    /// error if no such peer can be found.
    fn choose_peer_for_request(
        &self,
        request: &StorageServiceRequest,
    ) -> Result<PeerNetworkId, Error> {
        let all_connected_peers = self.get_all_connected_peers()?;

        // Identify the peers that can service this request
        let internal_peer_states = self.peer_states.read();
        let serviceable_peers = all_connected_peers
            .into_iter()
            .filter(|peer| internal_peer_states.can_service_request(peer, request))
            .collect::<Vec<_>>();

        // Choose a random peer from those that can service the request
        serviceable_peers
            .choose(&mut rand::thread_rng())
            .copied()
            .ok_or_else(|| {
                Error::DataIsUnavailable(
                    format!("No connected peers are advertising that they can serve this data! Request: {:?}",request),
                )
            })
    }

    /// Fetches the next group of peers to poll. The group will contain: (i) the peer who was last
    /// polled (i.e., contains the oldest data); and (ii) any (new) peers that have connected
    /// since the last time this method was called (i.e., the peers that have not been polled yet).
    fn fetch_peers_to_poll(&self) -> Result<Vec<PeerNetworkId>, Error> {
        // Fetch all (new) unpolled peers
        let mut peers_to_poll = self
            .get_all_connected_peers()?
            .into_iter()
            .filter(|peer| !self.peer_states.read().already_polled_peer(peer))
            .collect::<Vec<_>>();

        // Fetch the last polled peer
        if let Some(peer) = self.peer_states.write().oldest_polled_peer() {
            peers_to_poll.push(peer);
        }

        // Mark these peers as now polled
        for peer in &peers_to_poll {
            self.peer_states.write().add_polled_peer(*peer);
        }

        Ok(peers_to_poll)
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
        let _timer = start_timer(&metrics::REQUEST_LATENCIES, request.get_label().into());
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
        let response = self.send_request_to_peer(peer, request).await?;

        let (context, payload) = response.into_parts();

        // try to convert the storage service enum into the exact variant we're expecting.
        match T::try_from(payload) {
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

        debug!(
            (LogSchema::new(LogEntry::StorageServiceRequest)
                .event(LogEvent::SendRequest)
                .request_type(request.get_label())
                .request_id(id)
                .peer(&peer)
                .request_data(&request))
        );

        increment_counter(&metrics::SENT_REQUESTS, request.get_label().into());

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
                debug!(
                    (LogSchema::new(LogEntry::StorageServiceResponse)
                        .event(LogEvent::ResponseSuccess)
                        .request_type(request.get_label())
                        .request_id(id)
                        .peer(&peer))
                );

                increment_counter(&metrics::SUCCESS_RESPONSES, request.get_label().into());

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
            Err(err) => {
                // Convert network error and storage service error types into
                // data client errors. Also categorize the error type for scoring
                // purposes.
                let client_err = match err {
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
                        .request_type(request.get_label())
                        .request_id(id)
                        .peer(&peer)
                        .error(&client_err))
                );

                increment_counter(&metrics::ERROR_RESPONSES, request.get_label().into());

                self.notify_bad_response(id, peer, &request, ErrorType::NotUseful);
                Err(client_err)
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

    async fn get_account_states_with_proof(
        &self,
        version: u64,
        start_account_index: u64,
        end_account_index: u64,
    ) -> Result<Response<StateValueChunkWithProof>> {
        let request = StorageServiceRequest::GetAccountStatesChunkWithProof(
            AccountStatesChunkWithProofRequest {
                version,
                start_account_index,
                end_account_index,
            },
        );
        self.send_request_and_decode(request).await
    }

    async fn get_epoch_ending_ledger_infos(
        &self,
        start_epoch: Epoch,
        expected_end_epoch: Epoch,
    ) -> Result<Response<Vec<LedgerInfoWithSignatures>>> {
        let request =
            StorageServiceRequest::GetEpochEndingLedgerInfos(EpochEndingLedgerInfoRequest {
                start_epoch,
                expected_end_epoch,
            });
        let response: Response<EpochChangeProof> = self.send_request_and_decode(request).await?;
        Ok(response.map(|epoch_change| epoch_change.ledger_info_with_sigs))
    }

    async fn get_number_of_account_states(&self, version: Version) -> Result<Response<u64>> {
        let request = StorageServiceRequest::GetNumberOfAccountsAtVersion(version);
        self.send_request_and_decode(request).await
    }

    async fn get_transaction_outputs_with_proof(
        &self,
        proof_version: Version,
        start_version: Version,
        end_version: Version,
    ) -> Result<Response<TransactionOutputListWithProof>> {
        let request = StorageServiceRequest::GetTransactionOutputsWithProof(
            TransactionOutputsWithProofRequest {
                proof_version,
                start_version,
                end_version,
            },
        );
        self.send_request_and_decode(request).await
    }

    async fn get_transactions_with_proof(
        &self,
        proof_version: Version,
        start_version: Version,
        end_version: Version,
        include_events: bool,
    ) -> Result<Response<TransactionListWithProof>> {
        let request =
            StorageServiceRequest::GetTransactionsWithProof(TransactionsWithProofRequest {
                proof_version,
                start_version,
                end_version,
                include_events,
            });
        self.send_request_and_decode(request).await
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

pub struct DataSummaryPoller {
    time_service: TimeService,
    data_client: AptosNetDataClient,
    poll_interval: Duration,
}

impl DataSummaryPoller {
    fn new(
        time_service: TimeService,
        data_client: AptosNetDataClient,
        poll_interval: Duration,
    ) -> Self {
        Self {
            time_service,
            data_client,
            poll_interval,
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

            // Fetch the peers to poll
            let peers_to_poll = match self.data_client.fetch_peers_to_poll() {
                Ok(peers_to_poll) => peers_to_poll,
                Err(error) => {
                    sample!(
                        SampleRate::Duration(Duration::from_secs(POLLER_ERROR_LOG_FREQ_SECS)),
                        error!(
                            (LogSchema::new(LogEntry::StorageSummaryRequest)
                                .event(LogEvent::NoPeersToPoll)
                                .message("Unable to fetch any peers to poll!")
                                .error(&error))
                        );
                    );
                    continue;
                }
            };

            // Ensure peers to poll is not empty
            if peers_to_poll.is_empty() {
                sample!(
                    SampleRate::Duration(Duration::from_secs(POLLER_ERROR_LOG_FREQ_SECS)),
                    error!(
                        (LogSchema::new(LogEntry::StorageSummaryRequest)
                            .event(LogEvent::NoPeersToPoll)
                            .message("Peers to poll is empty!"))
                    );
                );
            }

            // Go through each peer and poll individually
            for peer in peers_to_poll {
                // Start the peer polling timer
                let timer = start_timer(
                    &metrics::REQUEST_LATENCIES,
                    StorageServiceRequest::GetStorageServerSummary
                        .get_label()
                        .into(),
                );

                // Fetch the storage summary for the peer
                let result: Result<StorageServerSummary> = self
                    .data_client
                    .send_request_to_peer_and_decode(
                        peer,
                        StorageServiceRequest::GetStorageServerSummary,
                    )
                    .await
                    .map(Response::into_payload);
                drop(timer);

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
                        continue;
                    }
                };

                // Update the global storage summary and the summary for the peer
                self.data_client.update_summary(peer, storage_summary);
                self.data_client.update_global_summary_cache();

                // Log the new global data summary and update the metrics
                sample!(
                    SampleRate::Duration(Duration::from_secs(GLOBAL_DATA_LOG_FREQ_SECS)),
                    info!(
                        (LogSchema::new(LogEntry::PeerStates)
                            .event(LogEvent::AggregateSummary)
                            .message(&format!(
                                "Global data summary: {:?}",
                                self.data_client.get_global_data_summary()
                            )))
                    );
                );
                sample!(
                    SampleRate::Duration(Duration::from_secs(GLOBAL_DATA_METRIC_FREQ_SECS)),
                    let global_data_summary = self.data_client.global_summary_cache.read().clone();
                    update_advertised_data_metrics(global_data_summary);
                );
            }
        }
    }
}

/// Updates the advertised data metrics using the given global
/// data summary.
fn update_advertised_data_metrics(global_data_summary: GlobalDataSummary) {
    // Update the optimal chunk sizes
    let optimal_chunk_sizes = &global_data_summary.optimal_chunk_sizes;
    for data_type in DataType::get_all_types() {
        let optimal_chunk_size = match data_type {
            DataType::AccountStates => optimal_chunk_sizes.account_states_chunk_size,
            DataType::LedgerInfos => optimal_chunk_sizes.epoch_chunk_size,
            DataType::TransactionOutputs => optimal_chunk_sizes.transaction_output_chunk_size,
            DataType::Transactions => optimal_chunk_sizes.transaction_chunk_size,
        };
        metrics::set_gauge(
            &metrics::OPTIMAL_CHUNK_SIZES,
            data_type.as_str().into(),
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
            metrics::set_gauge(
                &metrics::HIGHEST_ADVERTISED_DATA,
                data_type.as_str().into(),
                highest_advertised_version,
            );
        }
    }

    // Update the lowest advertised data
    for data_type in DataType::get_all_types() {
        let lowest_advertised_version = match data_type {
            DataType::AccountStates => advertised_data.lowest_account_states_version(),
            DataType::LedgerInfos => Some(0), // All nodes contain all epoch ending ledger infos
            DataType::TransactionOutputs => advertised_data.lowest_transaction_output_version(),
            DataType::Transactions => advertised_data.lowest_transaction_version(),
        };
        if let Some(lowest_advertised_version) = lowest_advertised_version {
            metrics::set_gauge(
                &metrics::LOWEST_ADVERTISED_DATA,
                data_type.as_str().into(),
                lowest_advertised_version,
            );
        }
    }
}
