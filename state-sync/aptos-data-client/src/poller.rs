// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    client::AptosDataClient,
    error::Error,
    global_summary::GlobalDataSummary,
    interface::{AptosDataClientInterface, Response},
    latency_monitor::LatencyMonitor,
    logging::{LogEntry, LogEvent, LogSchema},
    metrics,
    metrics::{set_gauge, start_request_timer, DataType},
};
use aptos_config::{config::AptosDataClientConfig, network_id::PeerNetworkId};
use aptos_logger::{debug, info, sample, sample::SampleRate, warn};
use aptos_storage_interface::DbReader;
use aptos_storage_service_types::{
    requests::{DataRequest, StorageServiceRequest},
    responses::StorageServerSummary,
};
use aptos_time_service::{TimeService, TimeServiceTrait};
use futures::StreamExt;
use std::{sync::Arc, time::Duration};
use tokio::{runtime::Handle, task::JoinHandle};

// Useful constants
const GLOBAL_DATA_LOG_FREQ_SECS: u64 = 10;
const GLOBAL_DATA_METRIC_FREQ_SECS: u64 = 1;
const POLLER_LOG_FREQ_SECS: u64 = 2;
const REGULAR_PEER_SAMPLE_FREQ: u64 = 3;

/// A poller for storage summaries that is responsible for periodically refreshing
/// the view of advertised data in the network.
pub struct DataSummaryPoller {
    data_client_config: AptosDataClientConfig, // The configuration for the data client
    data_client: AptosDataClient,              // The data client through which to poll peers
    poll_loop_interval: Duration,              // The interval between polling loop executions
    runtime: Option<Handle>, // An optional runtime on which to spawn the poller threads
    storage: Arc<dyn DbReader>, // The reader interface to storage
    time_service: TimeService, // The service to monitor elapsed time
}

impl DataSummaryPoller {
    pub fn new(
        data_client_config: AptosDataClientConfig,
        data_client: AptosDataClient,
        poll_loop_interval: Duration,
        runtime: Option<Handle>,
        storage: Arc<dyn DbReader>,
        time_service: TimeService,
    ) -> Self {
        Self {
            data_client_config,
            data_client,
            poll_loop_interval,
            runtime,
            storage,
            time_service,
        }
    }

    /// Runs the poller that continuously updates the global data summary
    pub async fn start_poller(self) {
        // Create and start the latency monitor
        start_latency_monitor(
            self.data_client_config,
            self.data_client.clone(),
            self.storage.clone(),
            self.time_service.clone(),
            self.runtime.clone(),
        );

        // Start the poller
        info!(
            (LogSchema::new(LogEntry::DataSummaryPoller)
                .message("Starting the Aptos data poller!"))
        );
        let poll_loop_ticker = self.time_service.interval(self.poll_loop_interval);
        futures::pin_mut!(poll_loop_ticker);

        loop {
            // Wait for next round before polling
            poll_loop_ticker.next().await;

            // Update the global storage summary
            if let Err(error) = self.data_client.update_global_summary_cache() {
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

            // Fetch the prioritized and regular peers to poll (if any)
            let prioritized_peer = self.try_fetch_peer(true);
            let regular_peer = self.fetch_regular_peer(prioritized_peer.is_none());

            // Ensure the peers to poll exist
            if prioritized_peer.is_none() && regular_peer.is_none() {
                sample!(
                    SampleRate::Duration(Duration::from_secs(POLLER_LOG_FREQ_SECS)),
                    debug!(
                        (LogSchema::new(LogEntry::DataSummaryPoller)
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
    pub(crate) fn fetch_regular_peer(&self, always_poll: bool) -> Option<PeerNetworkId> {
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
    pub(crate) fn try_fetch_peer(&self, is_priority_peer: bool) -> Option<PeerNetworkId> {
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

/// Spawns a dedicated poller for the given peer.
pub(crate) fn poll_peer(
    data_client: AptosDataClient,
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
        let request_timeout = data_client.get_response_timeout_ms();

        // Start the peer polling timer
        let timer = start_request_timer(
            &metrics::REQUEST_LATENCIES,
            &storage_request.get_label(),
            peer,
        );

        // Fetch the storage summary for the peer and stop the timer
        let result: crate::error::Result<StorageServerSummary> = data_client
            .send_request_to_peer_and_decode(peer, storage_request, request_timeout)
            .await
            .map(Response::into_payload);
        drop(timer);

        // Mark the in-flight poll as now complete
        data_client.in_flight_request_complete(&peer);

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

/// Spawns the dedicated latency monitor
fn start_latency_monitor(
    data_client_config: AptosDataClientConfig,
    data_client: AptosDataClient,
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
