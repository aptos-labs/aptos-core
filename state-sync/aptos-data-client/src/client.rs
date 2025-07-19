// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::Error,
    global_summary::GlobalDataSummary,
    interface::{
        AptosDataClientInterface, Response, ResponseCallback, ResponseContext, ResponseError,
        ResponseId, SubscriptionRequestMetadata,
    },
    logging::{LogEntry, LogEvent, LogSchema},
    metrics,
    metrics::{
        increment_request_counter, set_gauge, start_request_timer, PRIORITIZED_PEER, REGULAR_PEER,
    },
    peer_states::{ErrorType, PeerStates},
    poller::DataSummaryPoller,
    priority,
    priority::PeerPriority,
    utils,
};
use aptos_config::{
    config::{AptosDataClientConfig, BaseConfig},
    network_id::PeerNetworkId,
};
use aptos_id_generator::{IdGenerator, U64IdGenerator};
use aptos_infallible::Mutex;
use aptos_logger::{info, sample, sample::SampleRate, trace, warn};
use aptos_network::{
    application::{interface::NetworkClient, storage::PeersAndMetadata},
    protocols::network::RpcError,
};
use aptos_storage_interface::DbReader;
use aptos_storage_service_client::StorageServiceClient;
use aptos_storage_service_types::{
    requests::{
        DataRequest, EpochEndingLedgerInfoRequest, NewTransactionOutputsWithProofRequest,
        NewTransactionsOrOutputsWithProofRequest, NewTransactionsWithProofRequest,
        StateValuesWithProofRequest, StorageServiceRequest,
        SubscribeTransactionOutputsWithProofRequest,
        SubscribeTransactionsOrOutputsWithProofRequest, SubscribeTransactionsWithProofRequest,
        SubscriptionStreamMetadata, TransactionOutputsWithProofRequest,
        TransactionsOrOutputsWithProofRequest, TransactionsWithProofRequest,
    },
    responses::{StorageServerSummary, StorageServiceResponse, TransactionOrOutputListWithProofV2},
    Epoch, StorageServiceMessage,
};
use aptos_time_service::TimeService;
use aptos_types::{
    epoch_change::EpochChangeProof,
    ledger_info::LedgerInfoWithSignatures,
    state_store::state_value::StateValueChunkWithProof,
    transaction::{TransactionListWithProofV2, TransactionOutputListWithProofV2, Version},
};
use arc_swap::ArcSwap;
use async_trait::async_trait;
use futures::{stream::FuturesUnordered, StreamExt};
use maplit::hashset;
use std::{
    cmp::min,
    collections::{BTreeMap, HashSet},
    fmt,
    ops::Deref,
    sync::Arc,
    time::Duration,
};
use tokio::runtime::Handle;

// Useful constants
const PEER_METRICS_FREQ_SECS: u64 = 5; // The frequency to update peer metrics and logs

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
    /// The base config of the node.
    base_config: Arc<BaseConfig>,
    /// The config for the AptosNet data client.
    data_client_config: Arc<AptosDataClientConfig>,
    /// The underlying AptosNet storage service client.
    storage_service_client: StorageServiceClient<NetworkClient<StorageServiceMessage>>,
    /// The state of the active subscription stream.
    active_subscription_state: Arc<Mutex<Option<SubscriptionState>>>,
    /// All of the data-client specific data we have on each network peer.
    peer_states: Arc<PeerStates>,
    /// A cached, aggregate data summary of all unbanned peers' data summaries.
    global_summary_cache: Arc<ArcSwap<GlobalDataSummary>>,
    /// Used for generating the next request/response id.
    response_id_generator: Arc<U64IdGenerator>,
    /// Time service used for calculating peer lag
    time_service: TimeService,
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
        // Wrap the configs in an Arc (to be shared across components)
        let base_config = Arc::new(base_config);
        let data_client_config = Arc::new(data_client_config);

        // Create the data client
        let data_client = Self {
            base_config,
            data_client_config: data_client_config.clone(),
            storage_service_client: storage_service_client.clone(),
            active_subscription_state: Arc::new(Mutex::new(None)),
            peer_states: Arc::new(PeerStates::new(data_client_config.clone())),
            global_summary_cache: Arc::new(ArcSwap::from(Arc::new(GlobalDataSummary::empty()))),
            response_id_generator: Arc::new(U64IdGenerator::new()),
            time_service: time_service.clone(),
        };

        // Create the data summary poller
        let data_summary_poller = DataSummaryPoller::new(
            data_client_config,
            data_client.clone(),
            storage_service_client.get_peers_and_metadata(),
            runtime,
            storage,
            time_service,
        );

        (data_client, data_summary_poller)
    }

    /// Returns the max number of output reductions as defined by the config
    fn get_max_num_output_reductions(&self) -> u64 {
        self.data_client_config.max_num_output_reductions
    }

    /// Returns the maximum number of bytes that can be returned in a single response
    fn get_max_response_bytes(&self) -> u64 {
        self.data_client_config.max_response_bytes
    }

    /// Returns the peers and metadata struct
    pub fn get_peers_and_metadata(&self) -> Arc<PeersAndMetadata> {
        self.storage_service_client.get_peers_and_metadata()
    }

    /// Returns true iff transaction v2 is enabled, and
    /// the client should use transaction v2 data requests.
    fn is_transaction_v2_enabled(&self) -> bool {
        self.data_client_config.enable_transaction_data_v2
    }

    /// Updates the metrics and logs for peer states. This includes
    /// peer priorities and request distributions.
    pub fn update_peer_metrics_and_logs(&self) {
        // Update the peer request logs and metrics
        self.peer_states.update_peer_request_logs_and_metrics();

        // Update the peer priority metrics and logs (infrequently)
        sample!(
            SampleRate::Duration(Duration::from_secs(PEER_METRICS_FREQ_SECS)),
            {
                // Update the priority and regular peer metrics
                match self.get_priority_and_regular_peers() {
                    Ok((priority_peers, regular_peers)) => {
                        update_priority_and_regular_peer_metrics(&priority_peers, &regular_peers);
                    },
                    Err(error) => {
                        warn!(
                            (LogSchema::new(LogEntry::PeerStates)
                                .event(LogEvent::PriorityAndRegularPeers)
                                .message("Unable to update priority and regular peer metrics!")
                                .error(&error))
                        );
                    },
                };

                // Update the fine-grained peer priority metrics
                match self.get_peers_by_priorities() {
                    Ok(peers_by_priorities) => {
                        update_peer_priority_metrics(&peers_by_priorities);
                    },
                    Err(error) => {
                        warn!(
                            (LogSchema::new(LogEntry::PeerStates)
                                .event(LogEvent::PriorityPeerCategories)
                                .message("Unable to update peer priority metrics!")
                                .error(&error))
                        );
                    },
                };
            }
        );
    }

    /// Update a peer's storage summary
    pub fn update_peer_storage_summary(&self, peer: PeerNetworkId, summary: StorageServerSummary) {
        self.peer_states.update_summary(peer, summary)
    }

    /// Recompute and update the global data summary cache
    pub fn update_global_summary_cache(&self) -> crate::error::Result<(), Error> {
        // Before calculating the summary, we should garbage collect
        // the peer states (to handle disconnected peers).
        self.garbage_collect_peer_states()?;

        // Calculate the global data summary
        let global_data_summary = self.peer_states.calculate_global_data_summary();

        // Update the cached data summary
        self.global_summary_cache
            .store(Arc::new(global_data_summary));

        Ok(())
    }

    /// Garbage collects the peer states to remove data for disconnected peers
    fn garbage_collect_peer_states(&self) -> crate::error::Result<(), Error> {
        // Get all connected peers
        let all_connected_peers = self.get_all_connected_peers()?;

        // Garbage collect the disconnected peers
        self.peer_states
            .garbage_collect_peer_states(all_connected_peers);

        Ok(())
    }

    /// Chooses peers randomly weighted by distance and
    /// latency from the given set of serviceable peers.
    fn choose_random_peers_by_distance_and_latency(
        &self,
        serviceable_peers: HashSet<PeerNetworkId>,
        num_peers_to_choose: usize,
    ) -> HashSet<PeerNetworkId> {
        // Choose peers weighted by distance and latency
        let selected_peers = utils::choose_random_peers_by_distance_and_latency(
            serviceable_peers.clone(),
            self.get_peers_and_metadata(),
            num_peers_to_choose,
        );

        // Extend the selected peers with random peers (if necessary)
        utils::extend_with_random_peers(selected_peers, serviceable_peers, num_peers_to_choose)
    }

    /// Chooses several connected peers to service the given request.
    /// Returns an error if no single peer can service the request.
    pub(crate) fn choose_peers_for_request(
        &self,
        request: &StorageServiceRequest,
    ) -> crate::error::Result<HashSet<PeerNetworkId>, Error> {
        // Get all peers grouped by priorities
        let peers_by_priorities = self.get_peers_by_priorities()?;

        // Identify the peers that can service the request (ordered by priority)
        let mut serviceable_peers_by_priorities = vec![];
        for priority in PeerPriority::get_all_ordered_priorities() {
            // Identify the serviceable peers for the priority
            let peers = self.identify_serviceable(&peers_by_priorities, priority, request);

            // Add the serviceable peers to the ordered list
            serviceable_peers_by_priorities.push(peers);
        }

        // If the request is a subscription request, select a single
        // peer (as we can only subscribe to a single peer at a time).
        if request.data_request.is_subscription_request() {
            return self
                .choose_peer_for_subscription_request(request, serviceable_peers_by_priorities);
        }

        // Otherwise, determine the number of peers to select for the request
        let multi_fetch_config = self.data_client_config.data_multi_fetch_config;
        let num_peers_for_request = if multi_fetch_config.enable_multi_fetch {
            // Calculate the total number of priority serviceable peers
            let mut num_serviceable_peers = 0;
            for (index, peers) in serviceable_peers_by_priorities.iter().enumerate() {
                // Only include the lowest priority peers if no other peers are
                // available (the lowest priority peers are generally unreliable).
                if (num_serviceable_peers == 0)
                    || (index < serviceable_peers_by_priorities.len() - 1)
                {
                    num_serviceable_peers += peers.len();
                }
            }

            // Calculate the number of peers to select for the request
            let peer_ratio_for_request =
                num_serviceable_peers / multi_fetch_config.multi_fetch_peer_bucket_size;
            let mut num_peers_for_request = multi_fetch_config.min_peers_for_multi_fetch
                + (peer_ratio_for_request * multi_fetch_config.additional_requests_per_peer_bucket);

            // Bound the number of peers by the number of serviceable peers
            num_peers_for_request = min(num_peers_for_request, num_serviceable_peers);

            // Ensure the number of peers is no larger than the maximum
            min(
                num_peers_for_request,
                multi_fetch_config.max_peers_for_multi_fetch,
            )
        } else {
            1 // Multi-fetch is disabled (only select a single peer)
        };

        // Verify that we have at least one peer to service the request
        if num_peers_for_request == 0 {
            return Err(Error::DataIsUnavailable(format!(
                "No peers are available to service the given request: {:?}",
                request
            )));
        }

        // Choose the peers based on the request type
        if request.data_request.is_optimistic_fetch() {
            self.choose_peers_for_optimistic_fetch(
                request,
                serviceable_peers_by_priorities,
                num_peers_for_request,
            )
        } else {
            self.choose_peers_for_specific_data_request(
                request,
                serviceable_peers_by_priorities,
                num_peers_for_request,
            )
        }
    }

    /// Chooses several peers to service the given optimistic fetch
    /// request. Peers are selected first by priority, and then by
    /// validator distance and latency (within priority groups).
    fn choose_peers_for_optimistic_fetch(
        &self,
        request: &StorageServiceRequest,
        serviceable_peers_by_priorities: Vec<HashSet<PeerNetworkId>>,
        num_peers_for_request: usize,
    ) -> crate::error::Result<HashSet<PeerNetworkId>, Error> {
        // Select peers by priority (starting with the highest priority first)
        let mut selected_peers = HashSet::new();
        for serviceable_peers in serviceable_peers_by_priorities {
            // Select peers by distance and latency
            let num_peers_remaining = num_peers_for_request.saturating_sub(selected_peers.len());
            let peers = self.choose_random_peers_by_distance_and_latency(
                serviceable_peers,
                num_peers_remaining,
            );

            // Add the peers to the entire set
            selected_peers.extend(peers);

            // If we have selected enough peers, return early
            if selected_peers.len() >= num_peers_for_request {
                return Ok(selected_peers);
            }
        }

        // If selected peers is empty, return an error
        if !selected_peers.is_empty() {
            Ok(selected_peers)
        } else {
            Err(Error::DataIsUnavailable(format!(
                "Unable to select peers for optimistic fetch request: {:?}",
                request
            )))
        }
    }

    /// Chooses several peers to service the specific data request.
    /// Peers are selected first by priority, and then by latency
    /// (within priority groups).
    fn choose_peers_for_specific_data_request(
        &self,
        request: &StorageServiceRequest,
        serviceable_peers_by_priorities: Vec<HashSet<PeerNetworkId>>,
        num_peers_for_request: usize,
    ) -> crate::error::Result<HashSet<PeerNetworkId>, Error> {
        // Select peers by priority (starting with the highest priority first)
        let mut selected_peers = HashSet::new();
        for serviceable_peers in serviceable_peers_by_priorities {
            // Select peers by distance and latency
            let num_peers_remaining = num_peers_for_request.saturating_sub(selected_peers.len());
            let peers = self.choose_random_peers_by_latency(serviceable_peers, num_peers_remaining);

            // Add the peers to the entire set
            selected_peers.extend(peers);

            // If we have selected enough peers, return early
            if selected_peers.len() >= num_peers_for_request {
                return Ok(selected_peers);
            }
        }

        // If selected peers is empty, return an error
        if !selected_peers.is_empty() {
            Ok(selected_peers)
        } else {
            Err(Error::DataIsUnavailable(format!(
                "Unable to select peers for specific data request: {:?}",
                request
            )))
        }
    }

    /// Chooses a single peer to service the given subscription request.
    /// Peers are selected first by priority, and then by validator
    /// distance and latency (within priority groups).
    fn choose_peer_for_subscription_request(
        &self,
        request: &StorageServiceRequest,
        serviceable_peers_by_priorities: Vec<HashSet<PeerNetworkId>>,
    ) -> crate::error::Result<HashSet<PeerNetworkId>, Error> {
        // Prioritize peer selection by choosing the highest priority peer first
        for serviceable_peers in serviceable_peers_by_priorities {
            if let Some(selected_peer) =
                self.choose_serviceable_peer_for_subscription_request(request, serviceable_peers)?
            {
                return Ok(hashset![selected_peer]); // A peer was found!
            }
        }

        // Otherwise, no peer was selected, return an error
        Err(Error::DataIsUnavailable(format!(
            "Unable to select peers for subscription request: {:?}",
            request
        )))
    }

    /// Chooses a peer that can service the given subscription request.
    /// If not peer can service the request, None is returned.
    fn choose_serviceable_peer_for_subscription_request(
        &self,
        request: &StorageServiceRequest,
        serviceable_peers: HashSet<PeerNetworkId>,
    ) -> crate::error::Result<Option<PeerNetworkId>, Error> {
        // If there are no serviceable peers, return None
        if serviceable_peers.is_empty() {
            return Ok(None);
        }

        // Get the stream ID from the request
        let request_stream_id = match &request.data_request {
            DataRequest::SubscribeTransactionsWithProof(request) => {
                request.subscription_stream_metadata.subscription_stream_id
            },
            DataRequest::SubscribeTransactionOutputsWithProof(request) => {
                request.subscription_stream_metadata.subscription_stream_id
            },
            DataRequest::SubscribeTransactionsOrOutputsWithProof(request) => {
                request.subscription_stream_metadata.subscription_stream_id
            },
            DataRequest::SubscribeTransactionDataWithProof(request) => {
                request.subscription_stream_metadata.subscription_stream_id
            },
            data_request => {
                return Err(Error::UnexpectedErrorEncountered(format!(
                    "Invalid subscription request type found: {:?}",
                    data_request
                )))
            },
        };

        // Grab the lock on the active subscription state
        let mut active_subscription_state = self.active_subscription_state.lock();

        // If we have an active subscription and the request is for the same
        // stream ID, use the same peer (as long as it is still serviceable).
        if let Some(subscription_state) = active_subscription_state.take() {
            if subscription_state.subscription_stream_id == request_stream_id {
                // The stream IDs match. Verify that the request is still serviceable.
                let peer_network_id = subscription_state.peer_network_id;
                return if serviceable_peers.contains(&peer_network_id) {
                    // The previously chosen peer can still service the request
                    *active_subscription_state = Some(subscription_state);
                    Ok(Some(peer_network_id))
                } else {
                    // The previously chosen peer is either: (i) unable to service
                    // the request; or (ii) no longer the highest priority peer. So
                    // we need to return an error so the stream will be terminated.
                    Err(Error::DataIsUnavailable(format!(
                        "The peer that we were previously subscribing to should no \
                        longer service the subscriptions! Peer: {:?}, request: {:?}",
                        peer_network_id, request
                    )))
                };
            }
        }

        // Otherwise, choose a new peer to handle the subscription request
        let selected_peer = self
            .choose_random_peers_by_distance_and_latency(serviceable_peers, 1)
            .into_iter()
            .next();

        // If a peer was selected, update the active subscription state
        if let Some(selected_peer) = selected_peer {
            let subscription_state = SubscriptionState::new(selected_peer, request_stream_id);
            *active_subscription_state = Some(subscription_state);
        }

        Ok(selected_peer)
    }

    /// Chooses peers randomly weighted by latency from the given set of serviceable peers
    fn choose_random_peers_by_latency(
        &self,
        serviceable_peers: HashSet<PeerNetworkId>,
        num_peers_to_choose: usize,
    ) -> HashSet<PeerNetworkId> {
        // Choose peers weighted by latency
        let selected_peers = utils::choose_peers_by_latency(
            self.data_client_config.clone(),
            num_peers_to_choose as u64,
            serviceable_peers.clone(),
            self.get_peers_and_metadata(),
            true,
        );

        // Extend the selected peers with random peers (if necessary)
        utils::extend_with_random_peers(selected_peers, serviceable_peers, num_peers_to_choose)
    }

    /// Identifies the peers with the specified priority that can service the given request
    fn identify_serviceable(
        &self,
        peers_by_priorities: &BTreeMap<PeerPriority, HashSet<PeerNetworkId>>,
        priority: PeerPriority,
        request: &StorageServiceRequest,
    ) -> HashSet<PeerNetworkId> {
        // Get the peers for the specified priority
        let prospective_peers = peers_by_priorities
            .get(&priority)
            .unwrap_or(&hashset![])
            .clone();

        // Identify and return the serviceable peers
        prospective_peers
            .into_iter()
            .filter(|peer| {
                self.peer_states
                    .can_service_request(peer, self.time_service.clone(), request)
            })
            .collect()
    }

    /// Returns all peers connected to us
    fn get_all_connected_peers(&self) -> crate::error::Result<HashSet<PeerNetworkId>, Error> {
        let connected_peers = self.storage_service_client.get_available_peers()?;
        if connected_peers.is_empty() {
            return Err(Error::NoConnectedPeers(
                "No available peers found!".to_owned(),
            ));
        }

        Ok(connected_peers)
    }

    /// Returns all peers grouped by priorities
    fn get_peers_by_priorities(
        &self,
    ) -> crate::error::Result<BTreeMap<PeerPriority, HashSet<PeerNetworkId>>, Error> {
        // Get all connected peers
        let all_connected_peers = self.get_all_connected_peers()?;

        // Group the peers by priority
        let mut peers_by_priorities = BTreeMap::new();
        for peer in all_connected_peers {
            // Get the priority for the peer
            let priority = priority::get_peer_priority(
                self.base_config.clone(),
                self.get_peers_and_metadata(),
                &peer,
            );

            // Insert the peer into the priority map
            peers_by_priorities
                .entry(priority)
                .or_insert_with(HashSet::new)
                .insert(peer);
        }

        Ok(peers_by_priorities)
    }

    /// Returns all priority and regular peers. We define "priority peers" as
    /// high-priority peers only, and "regular peers" as all other priority categories.
    pub fn get_priority_and_regular_peers(
        &self,
    ) -> crate::error::Result<(HashSet<PeerNetworkId>, HashSet<PeerNetworkId>), Error> {
        // Get all connected peers
        let all_connected_peers = self.get_all_connected_peers()?;

        // Gather the priority and regular peers
        let mut priority_peers = hashset![];
        let mut regular_peers = hashset![];
        for peer in all_connected_peers {
            if priority::is_high_priority_peer(
                self.base_config.clone(),
                self.get_peers_and_metadata(),
                &peer,
            ) {
                priority_peers.insert(peer);
            } else {
                regular_peers.insert(peer);
            }
        }

        Ok((priority_peers, regular_peers))
    }

    /// Sends the specified storage request to a number of peers
    /// in the network and decodes the first successful response.
    async fn send_request_and_decode<T, E>(
        &self,
        request: StorageServiceRequest,
        request_timeout_ms: u64,
    ) -> crate::error::Result<Response<T>>
    where
        T: TryFrom<StorageServiceResponse, Error = E> + Send + Sync + 'static,
        E: Into<Error>,
    {
        // Select the peers to service the request
        let peers = self.choose_peers_for_request(&request)?;

        // If peers is empty, return an error
        if peers.is_empty() {
            return Err(Error::DataIsUnavailable(format!(
                "No peers were chosen to service the given request: {:?}",
                request
            )));
        }

        // Update the metrics for the number of selected peers (for the request)
        metrics::observe_value_with_label(
            &metrics::MULTI_FETCHES_PER_REQUEST,
            &request.get_label(),
            peers.len() as f64,
        );

        // Send the requests to the peers (and gather abort handles for the tasks)
        let mut sent_requests = FuturesUnordered::new();
        let mut abort_handles = vec![];
        for peer in peers {
            // Send the request to the peer
            let aptos_data_client = self.clone();
            let request = request.clone();
            let sent_request = tokio::spawn(async move {
                aptos_data_client
                    .send_request_to_peer_and_decode(peer, request, request_timeout_ms)
                    .await
            });
            let abort_handle = sent_request.abort_handle();

            // Gather the tasks and abort handles
            sent_requests.push(sent_request);
            abort_handles.push(abort_handle);
        }

        // Wait for the first successful response and abort all other tasks.
        // If all requests fail, gather the errors and return them.
        let num_sent_requests = sent_requests.len();
        let mut sent_request_errors = vec![];
        for _ in 0..num_sent_requests {
            if let Ok(response_result) = sent_requests.select_next_some().await {
                match response_result {
                    Ok(response) => {
                        // We received a valid response. Abort all pending tasks.
                        for abort_handle in abort_handles {
                            abort_handle.abort();
                        }
                        return Ok(response); // Return the response
                    },
                    Err(error) => {
                        // Gather the error and continue waiting for a response
                        sent_request_errors.push(error)
                    },
                }
            }
        }

        // Otherwise, all requests failed and we should return an error
        Err(Error::DataIsUnavailable(format!(
            "All {} attempts failed for the given request: {:?}. Errors: {:?}",
            num_sent_requests, request, sent_request_errors
        )))
    }

    /// Sends a request to a specific peer and decodes the response
    pub async fn send_request_to_peer_and_decode<T, E>(
        &self,
        peer: PeerNetworkId,
        request: StorageServiceRequest,
        request_timeout_ms: u64,
    ) -> crate::error::Result<Response<T>>
    where
        T: TryFrom<StorageServiceResponse, Error = E> + Send + 'static,
        E: Into<Error>,
    {
        // Start the timer for the request
        let timer = start_request_timer(&metrics::REQUEST_LATENCIES, &request.get_label(), peer);

        // Get the response from the peer
        let response = self
            .send_request_to_peer(peer, request.clone(), request_timeout_ms)
            .await;

        // If an error occurred, stop the timer (without updating the metrics)
        // and return the error. Otherwise, stop the timer and update the metrics.
        let storage_response = match response {
            Ok(storage_response) => {
                timer.stop_and_record(); // Update the latency metrics
                storage_response
            },
            Err(error) => {
                timer.stop_and_discard(); // Discard the timer without updating the metrics
                return Err(error);
            },
        };

        // Ensure the response obeys the compression requirements
        let (context, storage_response) = storage_response.into_parts();
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

        // Try to convert the storage service enum into the exact variant we're expecting.
        // We do this using spawn_blocking because it involves serde and compression.
        tokio::task::spawn_blocking(move || {
            match T::try_from(storage_response) {
                Ok(new_payload) => Ok(Response::new(context, new_payload)),
                // If the variant doesn't match what we're expecting, report the issue
                Err(err) => {
                    context
                        .response_callback
                        .notify_bad_response(ResponseError::InvalidPayloadDataType);
                    Err(err.into())
                },
            }
        })
        .await
        .map_err(|error| Error::UnexpectedErrorEncountered(error.to_string()))?
    }

    /// Sends a request to a specific peer
    async fn send_request_to_peer(
        &self,
        peer: PeerNetworkId,
        request: StorageServiceRequest,
        request_timeout_ms: u64,
    ) -> crate::error::Result<Response<StorageServiceResponse>, Error> {
        // Generate a unique id for the request
        let id = self.response_id_generator.next();

        // Update the sent request metrics
        trace!(
            (LogSchema::new(LogEntry::StorageServiceRequest)
                .event(LogEvent::SendRequest)
                .request_type(&request.get_label())
                .request_id(id)
                .peer(&peer)
                .request_data(&request))
        );
        self.update_sent_request_metrics(peer, &request);

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

                // Update the received response metrics
                self.update_received_response_metrics(peer, &request);

                // For now, record all responses that at least pass the data
                // client layer successfully. An alternative might also have the
                // consumer notify both success and failure via the callback.
                // On the one hand, scoring dynamics are simpler when each request
                // is successful or failed but not both; on the other hand, this
                // feels simpler for the consumer.
                self.peer_states.update_score_success(peer);

                // Package up all of the context needed to fully report an error
                // with this RPC.
                let response_callback = AptosNetResponseCallback {
                    data_client: self.clone(),
                    id,
                    peer,
                    request,
                };
                let context = ResponseContext::new(id, Box::new(response_callback));
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
        self.peer_states.update_score_error(peer, error_type);
    }

    /// Creates a storage service request using the given data request
    /// and sends it across the network
    async fn create_and_send_storage_request<T, E>(
        &self,
        request_timeout_ms: u64,
        data_request: DataRequest,
    ) -> crate::error::Result<Response<T>>
    where
        T: TryFrom<StorageServiceResponse, Error = E> + Send + Sync + 'static,
        E: Into<Error>,
    {
        let storage_request =
            StorageServiceRequest::new(data_request, self.data_client_config.use_compression);
        self.send_request_and_decode(storage_request, request_timeout_ms)
            .await
    }

    /// Updates the metrics for the responses received via the data client
    fn update_received_response_metrics(
        &self,
        peer: PeerNetworkId,
        request: &StorageServiceRequest,
    ) {
        // Update the global received response metrics
        increment_request_counter(&metrics::SUCCESS_RESPONSES, &request.get_label(), peer);

        // Update the received response counter for the specific peer
        self.peer_states
            .increment_received_response_counter(peer, request);
    }

    /// Updates the metrics for the requests sent via the data client
    fn update_sent_request_metrics(&self, peer: PeerNetworkId, request: &StorageServiceRequest) {
        // Increment the global request counter
        increment_request_counter(&metrics::SENT_REQUESTS, &request.get_label(), peer);

        // Update the sent request counter for the specific peer
        self.peer_states
            .increment_sent_request_counter(peer, request);
    }

    /// Returns the peer states
    pub fn get_peer_states(&self) -> Arc<PeerStates> {
        self.peer_states.clone()
    }
}

#[async_trait]
impl AptosDataClientInterface for AptosDataClient {
    fn get_global_data_summary(&self) -> GlobalDataSummary {
        self.global_summary_cache.load().clone().deref().clone()
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
    ) -> crate::error::Result<Response<(TransactionOutputListWithProofV2, LedgerInfoWithSignatures)>>
    {
        let data_request = if self.is_transaction_v2_enabled() {
            DataRequest::get_new_transaction_output_data_with_proof(
                known_version,
                known_epoch,
                self.get_max_response_bytes(),
            )
        } else {
            DataRequest::GetNewTransactionOutputsWithProof(NewTransactionOutputsWithProofRequest {
                known_version,
                known_epoch,
            })
        };
        self.create_and_send_storage_request(request_timeout_ms, data_request)
            .await
    }

    async fn get_new_transactions_with_proof(
        &self,
        known_version: Version,
        known_epoch: Epoch,
        include_events: bool,
        request_timeout_ms: u64,
    ) -> crate::error::Result<Response<(TransactionListWithProofV2, LedgerInfoWithSignatures)>>
    {
        let data_request = if self.is_transaction_v2_enabled() {
            DataRequest::get_new_transaction_data_with_proof(
                known_version,
                known_epoch,
                include_events,
                self.get_max_response_bytes(),
            )
        } else {
            DataRequest::GetNewTransactionsWithProof(NewTransactionsWithProofRequest {
                known_version,
                known_epoch,
                include_events,
            })
        };
        self.create_and_send_storage_request(request_timeout_ms, data_request)
            .await
    }

    async fn get_new_transactions_or_outputs_with_proof(
        &self,
        known_version: Version,
        known_epoch: Epoch,
        include_events: bool,
        request_timeout_ms: u64,
    ) -> crate::error::Result<
        Response<(TransactionOrOutputListWithProofV2, LedgerInfoWithSignatures)>,
    > {
        let data_request = if self.is_transaction_v2_enabled() {
            DataRequest::get_new_transaction_or_output_data_with_proof(
                known_version,
                known_epoch,
                include_events,
                self.get_max_response_bytes(),
            )
        } else {
            DataRequest::GetNewTransactionsOrOutputsWithProof(
                NewTransactionsOrOutputsWithProofRequest {
                    known_version,
                    known_epoch,
                    include_events,
                    max_num_output_reductions: self.get_max_num_output_reductions(),
                },
            )
        };
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
    ) -> crate::error::Result<Response<TransactionOutputListWithProofV2>> {
        let data_request = if self.is_transaction_v2_enabled() {
            DataRequest::get_transaction_output_data_with_proof(
                proof_version,
                start_version,
                end_version,
                self.get_max_response_bytes(),
            )
        } else {
            DataRequest::GetTransactionOutputsWithProof(TransactionOutputsWithProofRequest {
                proof_version,
                start_version,
                end_version,
            })
        };
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
    ) -> crate::error::Result<Response<TransactionListWithProofV2>> {
        let data_request = if self.is_transaction_v2_enabled() {
            DataRequest::get_transaction_data_with_proof(
                proof_version,
                start_version,
                end_version,
                include_events,
                self.get_max_response_bytes(),
            )
        } else {
            DataRequest::GetTransactionsWithProof(TransactionsWithProofRequest {
                proof_version,
                start_version,
                end_version,
                include_events,
            })
        };
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
    ) -> crate::error::Result<Response<TransactionOrOutputListWithProofV2>> {
        let data_request = if self.is_transaction_v2_enabled() {
            DataRequest::get_transaction_or_output_data_with_proof(
                proof_version,
                start_version,
                end_version,
                include_events,
                self.get_max_response_bytes(),
            )
        } else {
            DataRequest::GetTransactionsOrOutputsWithProof(TransactionsOrOutputsWithProofRequest {
                proof_version,
                start_version,
                end_version,
                include_events,
                max_num_output_reductions: self.get_max_num_output_reductions(),
            })
        };
        self.create_and_send_storage_request(request_timeout_ms, data_request)
            .await
    }

    async fn subscribe_to_transaction_outputs_with_proof(
        &self,
        request_metadata: SubscriptionRequestMetadata,
        request_timeout_ms: u64,
    ) -> crate::error::Result<Response<(TransactionOutputListWithProofV2, LedgerInfoWithSignatures)>>
    {
        let subscription_stream_metadata = SubscriptionStreamMetadata {
            known_version_at_stream_start: request_metadata.known_version_at_stream_start,
            known_epoch_at_stream_start: request_metadata.known_epoch_at_stream_start,
            subscription_stream_id: request_metadata.subscription_stream_id,
        };
        let data_request = if self.is_transaction_v2_enabled() {
            DataRequest::subscribe_transaction_output_data_with_proof(
                subscription_stream_metadata,
                request_metadata.subscription_stream_index,
                self.get_max_response_bytes(),
            )
        } else {
            DataRequest::SubscribeTransactionOutputsWithProof(
                SubscribeTransactionOutputsWithProofRequest {
                    subscription_stream_metadata,
                    subscription_stream_index: request_metadata.subscription_stream_index,
                },
            )
        };
        self.create_and_send_storage_request(request_timeout_ms, data_request)
            .await
    }

    async fn subscribe_to_transactions_with_proof(
        &self,
        request_metadata: SubscriptionRequestMetadata,
        include_events: bool,
        request_timeout_ms: u64,
    ) -> crate::error::Result<Response<(TransactionListWithProofV2, LedgerInfoWithSignatures)>>
    {
        let subscription_stream_metadata = SubscriptionStreamMetadata {
            known_version_at_stream_start: request_metadata.known_version_at_stream_start,
            known_epoch_at_stream_start: request_metadata.known_epoch_at_stream_start,
            subscription_stream_id: request_metadata.subscription_stream_id,
        };
        let data_request = if self.is_transaction_v2_enabled() {
            DataRequest::subscribe_transaction_data_with_proof(
                subscription_stream_metadata,
                request_metadata.subscription_stream_index,
                include_events,
                self.get_max_response_bytes(),
            )
        } else {
            DataRequest::SubscribeTransactionsWithProof(SubscribeTransactionsWithProofRequest {
                subscription_stream_metadata,
                include_events,
                subscription_stream_index: request_metadata.subscription_stream_index,
            })
        };
        self.create_and_send_storage_request(request_timeout_ms, data_request)
            .await
    }

    async fn subscribe_to_transactions_or_outputs_with_proof(
        &self,
        request_metadata: SubscriptionRequestMetadata,
        include_events: bool,
        request_timeout_ms: u64,
    ) -> crate::error::Result<
        Response<(TransactionOrOutputListWithProofV2, LedgerInfoWithSignatures)>,
    > {
        let subscription_stream_metadata = SubscriptionStreamMetadata {
            known_version_at_stream_start: request_metadata.known_version_at_stream_start,
            known_epoch_at_stream_start: request_metadata.known_epoch_at_stream_start,
            subscription_stream_id: request_metadata.subscription_stream_id,
        };
        let data_request = if self.is_transaction_v2_enabled() {
            DataRequest::subscribe_transaction_or_output_data_with_proof(
                subscription_stream_metadata,
                request_metadata.subscription_stream_index,
                include_events,
                self.get_max_response_bytes(),
            )
        } else {
            DataRequest::SubscribeTransactionsOrOutputsWithProof(
                SubscribeTransactionsOrOutputsWithProofRequest {
                    subscription_stream_metadata,
                    include_events,
                    max_num_output_reductions: self.get_max_num_output_reductions(),
                    subscription_stream_index: request_metadata.subscription_stream_index,
                },
            )
        };
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

/// A struct that holds a subscription state, including
/// the subscription stream ID and the peer serving the requests.
#[derive(Clone, Debug)]
struct SubscriptionState {
    peer_network_id: PeerNetworkId,
    subscription_stream_id: u64,
}

impl SubscriptionState {
    fn new(peer_network_id: PeerNetworkId, subscription_stream_id: u64) -> Self {
        Self {
            peer_network_id,
            subscription_stream_id,
        }
    }
}

/// Updates the metrics for the number of connected peers (priority and regular)
fn update_priority_and_regular_peer_metrics(
    priority_peers: &HashSet<PeerNetworkId>,
    regular_peers: &HashSet<PeerNetworkId>,
) {
    // Log the number of connected peers
    let num_priority_peers = priority_peers.len();
    let num_regular_peers = regular_peers.len();
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

/// Updates the metrics for the number of connected peers by priority
fn update_peer_priority_metrics(
    peers_by_priority: &BTreeMap<PeerPriority, HashSet<PeerNetworkId>>,
) {
    // Calculate the number of peers by priority
    let mut num_peers_by_priority = BTreeMap::new();
    for (priority, peers) in peers_by_priority {
        num_peers_by_priority.insert(priority, peers.len());
    }

    // Log the number of connected peers by priority
    info!(
        (LogSchema::new(LogEntry::PeerStates)
            .event(LogEvent::PriorityPeerCategories)
            .message(&format!(
                "Number of connected peers by priority: {:?}",
                num_peers_by_priority,
            )))
    );

    // Update the connected peer priority metrics
    for (priority, num_peers) in num_peers_by_priority {
        set_gauge(
            &metrics::CONNECTED_PEERS_AND_PRIORITIES,
            &priority.get_label(),
            num_peers as u64,
        );
    }
}
