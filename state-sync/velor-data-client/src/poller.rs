// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    client::VelorDataClient,
    error::Error,
    global_summary::GlobalDataSummary,
    interface::{VelorDataClientInterface, Response},
    latency_monitor::LatencyMonitor,
    logging::{LogEntry, LogEvent, LogSchema},
    metrics,
    metrics::{set_gauge, DataType, PRIORITIZED_PEER, REGULAR_PEER},
    utils,
};
use velor_config::{
    config::{VelorDataClientConfig, VelorDataPollerConfig},
    network_id::PeerNetworkId,
};
use velor_logger::{debug, error, info, sample, sample::SampleRate, warn};
use velor_network::application::storage::PeersAndMetadata;
use velor_storage_interface::DbReader;
use velor_storage_service_types::{
    requests::{DataRequest, StorageServiceRequest},
    responses::StorageServerSummary,
};
use velor_time_service::{TimeService, TimeServiceTrait};
use dashmap::DashSet;
use futures::StreamExt;
use maplit::hashset;
use rand::Rng;
use std::{cmp, collections::HashSet, sync::Arc, time::Duration};
use tokio::{runtime::Handle, task::JoinHandle};

// Useful constants
const GLOBAL_DATA_LOG_FREQ_SECS: u64 = 10;
const GLOBAL_DATA_METRIC_FREQ_SECS: u64 = 1;
const IN_FLIGHT_METRICS_SAMPLE_FREQ: u64 = 5;
const NUM_MILLISECONDS_IN_SECONDS: f64 = 1000.0;
const POLLER_LOG_FREQ_SECS: u64 = 2;

/// A data summary poller that maintains state related to peer polling
#[derive(Clone)]
pub struct DataSummaryPoller {
    data_client_config: Arc<VelorDataClientConfig>, // The configuration for the data client
    data_client: VelorDataClient,                   // The data client through which to poll peers
    in_flight_priority_polls: Arc<DashSet<PeerNetworkId>>, // The set of priority peers with in-flight polls
    in_flight_regular_polls: Arc<DashSet<PeerNetworkId>>, // The set of regular peers with in-flight polls
    peers_and_metadata: Arc<PeersAndMetadata>,            // The peers and metadata
    runtime: Option<Handle>, // An optional runtime on which to spawn the poller threads
    storage: Arc<dyn DbReader>, // The reader interface to storage
    time_service: TimeService, // The service to monitor elapsed time
}

impl DataSummaryPoller {
    pub fn new(
        data_client_config: Arc<VelorDataClientConfig>,
        data_client: VelorDataClient,
        peers_and_metadata: Arc<PeersAndMetadata>,
        runtime: Option<Handle>,
        storage: Arc<dyn DbReader>,
        time_service: TimeService,
    ) -> Self {
        Self {
            data_client_config,
            data_client,
            in_flight_priority_polls: Arc::new(DashSet::new()),
            in_flight_regular_polls: Arc::new(DashSet::new()),
            peers_and_metadata,
            runtime,
            storage,
            time_service,
        }
    }

    /// Returns the next set of peers to poll based on the priorities
    pub(crate) fn identify_peers_to_poll(
        &self,
        poll_priority_peers: bool,
    ) -> Result<HashSet<PeerNetworkId>, Error> {
        // Fetch all priority and regular peers
        let (priority_peers, regular_peers) = self.data_client.get_priority_and_regular_peers()?;

        // Identify the peers to poll
        let peers_to_poll = if poll_priority_peers {
            self.get_priority_peers_to_poll(priority_peers)
        } else {
            self.get_regular_peers_to_poll(regular_peers)
        };

        Ok(peers_to_poll)
    }

    /// Identifies the next set of priority peers to poll from
    /// the given list of all priority peers.
    fn get_priority_peers_to_poll(
        &self,
        all_priority_peers: HashSet<PeerNetworkId>,
    ) -> HashSet<PeerNetworkId> {
        // Fetch the number of in-flight polls and update the metrics
        let num_in_flight_polls = self.in_flight_priority_polls.len() as u64;
        update_in_flight_metrics(PRIORITIZED_PEER, num_in_flight_polls);

        // Ensure we don't go over the maximum number of in-flight polls
        let data_poller_config = self.data_client_config.data_poller_config;
        let max_num_in_flight_polls = data_poller_config.max_num_in_flight_priority_polls;
        if num_in_flight_polls >= max_num_in_flight_polls {
            return hashset![];
        }

        // Calculate the number of peers to poll this round
        let max_num_peers_to_poll = max_num_in_flight_polls.saturating_sub(num_in_flight_polls);
        let num_peers_to_poll = calculate_num_peers_to_poll(
            &all_priority_peers,
            max_num_peers_to_poll,
            self.data_client_config.data_poller_config,
        );

        // Select a subset of the priority peers to poll
        self.select_peers_to_poll(all_priority_peers, num_peers_to_poll as usize)
    }

    /// Identifies the next set of regular peers to poll from
    /// the given list of all regular peers.
    fn get_regular_peers_to_poll(
        &self,
        all_regular_peers: HashSet<PeerNetworkId>,
    ) -> HashSet<PeerNetworkId> {
        // Fetch the number of in-flight polls and update the metrics
        let num_in_flight_polls = self.in_flight_regular_polls.len() as u64;
        update_in_flight_metrics(REGULAR_PEER, num_in_flight_polls);

        // Ensure we don't go over the maximum number of in-flight polls
        let data_poller_config = self.data_client_config.data_poller_config;
        let max_num_in_flight_polls = data_poller_config.max_num_in_flight_regular_polls;
        if num_in_flight_polls >= max_num_in_flight_polls {
            return hashset![];
        }

        // Calculate the number of peers to poll this round
        let max_num_peers_to_poll = max_num_in_flight_polls.saturating_sub(num_in_flight_polls);
        let num_peers_to_poll = calculate_num_peers_to_poll(
            &all_regular_peers,
            max_num_peers_to_poll,
            self.data_client_config.data_poller_config,
        );

        // Select a subset of the regular peers to poll
        self.select_peers_to_poll(all_regular_peers, num_peers_to_poll as usize)
    }

    /// Selects the peers to poll from the given peer
    /// list and the number of peers to poll.
    fn select_peers_to_poll(
        &self,
        mut potential_peers: HashSet<PeerNetworkId>,
        num_peers_to_poll: usize,
    ) -> HashSet<PeerNetworkId> {
        // Filter out the peers that have an in-flight request
        let peers_with_in_flight_polls = self.all_peers_with_in_flight_polls();
        potential_peers = potential_peers
            .difference(&peers_with_in_flight_polls)
            .cloned()
            .collect();

        // Select the peers to poll
        let maybe_peers_to_poll = match num_peers_to_poll {
            0 => None, // Don't poll any peers
            1 => {
                // Choose randomly from the potential peers
                utils::choose_random_peer(potential_peers).map(|peer| hashset![peer])
            },
            num_peers_to_poll => {
                // Select half the peers randomly, and the other half weighted by latency
                let num_peers_to_poll_randomly = num_peers_to_poll / 2;
                let num_peers_to_poll_by_latency = num_peers_to_poll - num_peers_to_poll_randomly;

                // Select the random peers
                let random_peers_to_poll =
                    utils::choose_random_peers(num_peers_to_poll_randomly, potential_peers.clone());

                // Remove already selected peers
                let potential_peers = potential_peers
                    .difference(&random_peers_to_poll)
                    .cloned()
                    .collect();

                // Select the latency weighted peers
                let peers_to_poll_by_latency = utils::choose_peers_by_latency(
                    self.data_client_config.clone(),
                    num_peers_to_poll_by_latency as u64,
                    potential_peers,
                    self.peers_and_metadata.clone(),
                    false,
                );

                // Return all peers to poll
                let all_peers_to_poll = random_peers_to_poll
                    .union(&peers_to_poll_by_latency)
                    .cloned()
                    .collect();
                Some(all_peers_to_poll)
            },
        };
        maybe_peers_to_poll.unwrap_or(hashset![])
    }

    /// Marks the given peers as having an in-flight poll request
    pub(crate) fn in_flight_request_started(&self, is_priority_peer: bool, peer: &PeerNetworkId) {
        // Get the current in-flight polls
        let in_flight_polls = if is_priority_peer {
            self.in_flight_priority_polls.clone()
        } else {
            self.in_flight_regular_polls.clone()
        };

        // Insert the new peer
        if !in_flight_polls.insert(*peer) {
            error!(
                (LogSchema::new(LogEntry::PeerStates)
                    .event(LogEvent::PriorityAndRegularPeers)
                    .message(&format!(
                        "Peer already found with an in-flight poll! Priority: {:?}",
                        is_priority_peer
                    ))
                    .peer(peer))
            );
        }
    }

    /// Marks the pending in-flight request as complete for the specified peer
    pub(crate) fn in_flight_request_complete(&self, peer: &PeerNetworkId) {
        // The priority of the peer might have changed since we
        // last polled it, so we attempt to remove it from both
        // the regular and priority in-flight requests.
        if self.in_flight_priority_polls.remove(peer).is_none()
            && self.in_flight_regular_polls.remove(peer).is_none()
        {
            error!(
                (LogSchema::new(LogEntry::PeerStates)
                    .event(LogEvent::PriorityAndRegularPeers)
                    .message("Peer not found with an in-flight poll!")
                    .peer(peer))
            );
        }
    }

    /// Returns all peers with in-flight polls (both priority and regular peers)
    pub(crate) fn all_peers_with_in_flight_polls(&self) -> HashSet<PeerNetworkId> {
        // Add the priority peers with in-flight polls
        let mut peers_with_in_flight_polls = hashset![];
        for peer in self.in_flight_priority_polls.iter() {
            peers_with_in_flight_polls.insert(*peer);
        }

        // Add the regular peers with in-flight polls
        for peer in self.in_flight_regular_polls.iter() {
            peers_with_in_flight_polls.insert(*peer);
        }

        peers_with_in_flight_polls
    }
}

/// Runs a thread that continuously polls peers and updates
/// the global data summary.
pub async fn start_poller(poller: DataSummaryPoller) {
    // Create and start the latency monitor
    start_latency_monitor(
        poller.data_client_config.clone(),
        poller.data_client.clone(),
        poller.storage.clone(),
        poller.time_service.clone(),
        poller.runtime.clone(),
    );

    // Create the poll loop ticker
    let data_poller_config = poller.data_client_config.data_poller_config;
    let data_polling_interval = Duration::from_millis(data_poller_config.poll_loop_interval_ms);
    let poll_loop_ticker = poller.time_service.interval(data_polling_interval);
    futures::pin_mut!(poll_loop_ticker);

    // Start the poller
    let mut polling_round: u64 = 0;
    info!((LogSchema::new(LogEntry::DataSummaryPoller).message("Starting the Velor data poller!")));
    loop {
        // Wait for the next round before polling
        poll_loop_ticker.next().await;

        // Increment the round counter
        polling_round = polling_round.wrapping_add(1);

        // Update the global storage summary
        if let Err(error) = poller.data_client.update_global_summary_cache() {
            sample!(
                SampleRate::Duration(Duration::from_secs(POLLER_LOG_FREQ_SECS)),
                warn!(
                    (LogSchema::new(LogEntry::DataSummaryPoller)
                        .event(LogEvent::AggregateSummary)
                        .message("Unable to update global summary cache!")
                        .error(&error))
                );
            );
        }

        // Update the metrics and logs for the peer states
        poller.data_client.update_peer_metrics_and_logs();

        // Determine the peers to poll this round. If the round is even, poll
        // the priority peers. Otherwise, poll the regular peers. This allows
        // us to alternate between peer types and load balance requests.
        let poll_priority_peers = polling_round % 2 == 0;

        // Identify the peers to poll (if any)
        let peers_to_poll = match poller.identify_peers_to_poll(poll_priority_peers) {
            Ok(peers_to_poll) => peers_to_poll,
            Err(error) => {
                sample!(
                    SampleRate::Duration(Duration::from_secs(POLLER_LOG_FREQ_SECS)),
                    warn!(
                        (LogSchema::new(LogEntry::DataSummaryPoller)
                            .event(LogEvent::PeerPollingError)
                            .message("Unable to identify peers to poll!")
                            .error(&error))
                    );
                );
                continue;
            },
        };

        // Verify that we have at least one peer to poll
        if peers_to_poll.is_empty() {
            sample!(
                SampleRate::Duration(Duration::from_secs(POLLER_LOG_FREQ_SECS)),
                debug!(
                    (LogSchema::new(LogEntry::DataSummaryPoller)
                        .event(LogEvent::NoPeersToPoll)
                        .message("No peers to poll this round!"))
                );
            );
            continue;
        }

        // Go through each peer and poll them individually
        for peer in peers_to_poll {
            poll_peer(poller.clone(), poll_priority_peers, peer);
        }
    }
}

/// Calculates the number of peers to poll this round
pub(crate) fn calculate_num_peers_to_poll(
    potential_peers: &HashSet<PeerNetworkId>,
    max_num_peers_to_poll: u64,
    data_poller_config: VelorDataPollerConfig,
) -> u64 {
    // Calculate the total number of peers to poll (per second)
    let min_polls_per_second = data_poller_config.min_polls_per_second;
    let peer_bucket_sizes = data_poller_config.peer_bucket_size;
    let additional_polls_per_bucket = data_poller_config.additional_polls_per_peer_bucket;
    let total_polls_per_second = min_polls_per_second
        + (additional_polls_per_bucket * (potential_peers.len() as u64 / peer_bucket_sizes));

    // Bound the number of polls per second by the maximum configurable value
    let polls_per_second = cmp::min(
        total_polls_per_second,
        data_poller_config.max_polls_per_second,
    );

    // Calculate the number of loop executions per second
    let mut loops_per_second =
        NUM_MILLISECONDS_IN_SECONDS / (data_poller_config.poll_loop_interval_ms as f64);
    loops_per_second /= 2.0; // Divide by 2 because we poll priority and regular peers in alternating loops

    // Calculate the number of peers to poll (per round)
    let num_peers_to_poll = (polls_per_second as f64) / loops_per_second;

    // Convert the number of peers to poll to a u64. To do this, we round the
    // fractional part up to the nearest integer with an equal probability. For
    // example, if the fractional part is 0.7, then we round up to 1 with 70%
    // probability. This ensures that we poll the correct number of peers on average.
    let round_up = rand::thread_rng().gen_bool(num_peers_to_poll.fract());
    let num_peers_to_poll = if round_up {
        num_peers_to_poll.ceil() as u64
    } else {
        num_peers_to_poll.floor() as u64
    };

    // Bound the number of peers to poll by the given maximum
    cmp::min(num_peers_to_poll, max_num_peers_to_poll)
}

/// Spawns a dedicated poller for the given peer.
pub(crate) fn poll_peer(
    data_summary_poller: DataSummaryPoller,
    is_priority_peer: bool,
    peer: PeerNetworkId,
) -> JoinHandle<()> {
    // Mark the in-flight poll as started. We do this here to prevent
    // the main polling loop from selecting the same peer concurrently.
    data_summary_poller.in_flight_request_started(is_priority_peer, &peer);

    // Create the poller for the peer
    let runtime = data_summary_poller.runtime.clone();
    let poller = async move {
        // Construct the request for polling
        let data_request = DataRequest::GetStorageServerSummary;
        let use_compression = data_summary_poller.data_client_config.use_compression;
        let storage_request = StorageServiceRequest::new(data_request, use_compression);

        // Fetch the storage summary for the peer and stop the timer
        let request_timeout = data_summary_poller.data_client_config.response_timeout_ms;
        let result: crate::error::Result<StorageServerSummary> = data_summary_poller
            .data_client
            .send_request_to_peer_and_decode(peer, storage_request, request_timeout)
            .await
            .map(Response::into_payload);

        // Mark the in-flight poll as now complete
        data_summary_poller.in_flight_request_complete(&peer);

        // Check the storage summary response
        let storage_summary = match result {
            Ok(storage_summary) => storage_summary,
            Err(error) => {
                warn!(
                    (LogSchema::new(LogEntry::StorageSummaryResponse)
                        .event(LogEvent::PeerPollingError)
                        .message("Error encountered when polling peer!")
                        .error(&error)
                        .peer(&peer))
                );
                return;
            },
        };

        // Update the summary for the peer
        data_summary_poller
            .data_client
            .update_peer_storage_summary(peer, storage_summary);

        // Log the new global data summary and update the metrics
        sample!(
            SampleRate::Duration(Duration::from_secs(GLOBAL_DATA_LOG_FREQ_SECS)),
            info!(
                (LogSchema::new(LogEntry::PeerStates)
                    .event(LogEvent::AggregateSummary)
                    .message(&format!(
                        "Global data summary: {}",
                        data_summary_poller.data_client.get_global_data_summary()
                    )))
            );
        );
        sample!(
            SampleRate::Duration(Duration::from_secs(GLOBAL_DATA_METRIC_FREQ_SECS)),
            let global_data_summary = data_summary_poller.data_client.get_global_data_summary();
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

/// Spawns the dedicated latency monitor
fn start_latency_monitor(
    data_client_config: Arc<VelorDataClientConfig>,
    data_client: VelorDataClient,
    storage: Arc<dyn DbReader>,
    time_service: TimeService,
    runtime: Option<Handle>,
) -> JoinHandle<()> {
    // Create the latency monitor
    let latency_monitor = LatencyMonitor::new(
        data_client_config,
        Arc::new(data_client),
        storage,
        time_service,
    );

    // Spawn the latency monitor
    if let Some(runtime) = runtime {
        runtime.spawn(async move { latency_monitor.start_latency_monitor().await })
    } else {
        tokio::spawn(async move { latency_monitor.start_latency_monitor().await })
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
