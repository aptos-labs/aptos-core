// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

mod data_client;
mod data_manager;
mod fetch_manager;
mod in_memory_cache;

use crate::{
    config::LiveDataServiceConfig,
    connection_manager::ConnectionManager,
    live_data_service::in_memory_cache::InMemoryCache,
    metrics::{COUNTER, TIMER},
    service::StreamRequest,
};
use aptos_indexer_grpc_utils::{
    constants::{get_request_metadata, IndexerGrpcRequestMetadata},
    counters::BYTES_READY_TO_TRANSFER_FROM_SERVER_AFTER_STRIPPING,
    filter_utils,
};
use aptos_protos::indexer::v1::{ProcessedRange, TransactionsResponse};
use aptos_transaction_filter::BooleanTransactionFilter;
use prost::Message;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::mpsc::Receiver;
use tonic::Status;
use tracing::info;
use tracing_opentelemetry::OpenTelemetrySpanExt;

const MAX_BYTES_PER_BATCH: usize = 20 * (1 << 20);

pub struct LiveDataService<'a> {
    chain_id: u64,
    in_memory_cache: InMemoryCache<'a>,
    connection_manager: Arc<ConnectionManager>,
    max_transaction_filter_size_bytes: usize,
    checkpoint_interval: Option<Duration>,
}

impl<'a> LiveDataService<'a> {
    pub fn new(
        chain_id: u64,
        config: LiveDataServiceConfig,
        connection_manager: Arc<ConnectionManager>,
        max_transaction_filter_size_bytes: usize,
        checkpoint_interval_secs: u64,
    ) -> Self {
        let known_latest_version = connection_manager.known_latest_version();
        let checkpoint_interval = if checkpoint_interval_secs > 0 {
            Some(Duration::from_secs(checkpoint_interval_secs))
        } else {
            None
        };
        Self {
            chain_id,
            connection_manager: connection_manager.clone(),
            in_memory_cache: InMemoryCache::new(
                connection_manager,
                known_latest_version,
                config.num_slots,
                config.size_limit_bytes,
            ),
            max_transaction_filter_size_bytes,
            checkpoint_interval,
        }
    }

    pub fn run(&'a self, mut handler_rx: Receiver<StreamRequest>) {
        info!("Running LiveDataService...");
        tokio_scoped::scope(|scope| {
            scope.spawn(async move {
                let _ = self
                    .in_memory_cache
                    .fetch_manager
                    .continuously_fetch_latest_data()
                    .await;
            });
            while let Some((request, response_sender, parent_cx)) = handler_rx.blocking_recv() {
                COUNTER
                    .with_label_values(&["live_data_service_receive_request"])
                    .inc();
                // Extract request metadata before consuming the request.
                let request_metadata = Arc::new(get_request_metadata(&request));
                let request = request.into_inner();
                let id = request_metadata.request_connection_id.clone();
                let known_latest_version = self.get_known_latest_version();
                let starting_version = request.starting_version.unwrap_or(known_latest_version);

                info!("Received request: {request:?}.");
                if starting_version > known_latest_version + 10000 {
                    let err = Err(Status::failed_precondition(
                        "starting_version cannot be set to a far future version.",
                    ));
                    info!("Client error: {err:?}.");
                    let _ = response_sender.blocking_send(err);
                    COUNTER
                        .with_label_values(&["live_data_service_requested_data_too_new"])
                        .inc();
                    continue;
                }

                let filter = if let Some(proto_filter) = request.transaction_filter {
                    match filter_utils::parse_transaction_filter(
                        proto_filter,
                        self.max_transaction_filter_size_bytes,
                    ) {
                        Ok(filter) => Some(filter),
                        Err(err) => {
                            info!("Client error: {err:?}.");
                            let _ = response_sender.blocking_send(Err(err));
                            COUNTER
                                .with_label_values(&["live_data_service_invalid_filter"])
                                .inc();
                            continue;
                        },
                    }
                } else {
                    None
                };

                let max_num_transactions_per_batch = if let Some(batch_size) = request.batch_size {
                    batch_size as usize
                } else {
                    10000
                };

                let ending_version = request
                    .transactions_count
                    .map(|count| starting_version.saturating_add(count));

                scope.spawn(async move {
                    self.start_streaming(
                        id,
                        starting_version,
                        ending_version,
                        max_num_transactions_per_batch,
                        MAX_BYTES_PER_BATCH,
                        filter,
                        request_metadata,
                        response_sender,
                        parent_cx,
                    )
                    .await
                });
            }
        });
    }

    pub(crate) async fn get_min_servable_version(&self) -> u64 {
        self.in_memory_cache.data_manager.read().await.start_version
    }

    pub(super) fn get_connection_manager(&self) -> &ConnectionManager {
        &self.connection_manager
    }

    async fn start_streaming(
        &'a self,
        id: String,
        starting_version: u64,
        ending_version: Option<u64>,
        max_num_transactions_per_batch: usize,
        max_bytes_per_batch: usize,
        filter: Option<BooleanTransactionFilter>,
        request_metadata: Arc<IndexerGrpcRequestMetadata>,
        response_sender: tokio::sync::mpsc::Sender<Result<TransactionsResponse, Status>>,
        parent_cx: opentelemetry::Context,
    ) {
        COUNTER
            .with_label_values(&["live_data_service_new_stream"])
            .inc();

        let stream_span = tracing::info_span!(
            "live_stream",
            stream_id = %id,
            stream.starting_version = starting_version,
            stream.ending_version = ?ending_version,
            stream.service_type = "live",
            stream.total_bytes_sent = tracing::field::Empty,
            stream.final_version = tracing::field::Empty,
            stream.termination_reason = tracing::field::Empty,
        );
        let _ = stream_span.set_parent(parent_cx);
        let _stream_guard = stream_span.enter();

        info!(stream_id = id, "Start streaming, starting_version: {starting_version}, ending_version: {ending_version:?}.");
        self.connection_manager
            .insert_active_stream(&id, starting_version, ending_version);
        let mut next_version = starting_version;
        let mut size_bytes: u64 = 0;
        let ending_version = ending_version.unwrap_or(u64::MAX);

        let mut last_checkpoint = Instant::now();
        let mut batches_since_checkpoint: u64 = 0;
        let mut bytes_since_checkpoint: u64 = 0;
        let mut termination_reason = "ending_version_reached";

        loop {
            if next_version >= ending_version {
                break;
            }
            self.connection_manager
                .update_stream_progress(&id, next_version, size_bytes);
            let known_latest_version = self.get_known_latest_version();
            if next_version > known_latest_version {
                info!(stream_id = id, "next_version {next_version} is larger than known_latest_version {known_latest_version}");
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }

            if let Some((transactions, batch_size_bytes, last_processed_version)) = self
                .in_memory_cache
                .get_data(
                    next_version,
                    ending_version,
                    max_num_transactions_per_batch,
                    max_bytes_per_batch,
                    &filter,
                )
                .await
            {
                let _timer = TIMER
                    .with_label_values(&["live_data_service_send_batch"])
                    .start_timer();
                let response = TransactionsResponse {
                    transactions,
                    chain_id: Some(self.chain_id),
                    processed_range: Some(ProcessedRange {
                        first_version: next_version,
                        last_version: last_processed_version,
                    }),
                };
                let bytes_ready_to_transfer_after_stripping = response
                    .transactions
                    .iter()
                    .map(|t| t.encoded_len())
                    .sum::<usize>();
                BYTES_READY_TO_TRANSFER_FROM_SERVER_AFTER_STRIPPING
                    .with_label_values(&request_metadata.get_label_values())
                    .inc_by(bytes_ready_to_transfer_after_stripping as u64);
                next_version = last_processed_version + 1;
                size_bytes += batch_size_bytes as u64;
                batches_since_checkpoint += 1;
                bytes_since_checkpoint += batch_size_bytes as u64;

                if let Some(interval) = self.checkpoint_interval {
                    if last_checkpoint.elapsed() >= interval {
                        let _checkpoint = tracing::info_span!(
                            "stream_checkpoint",
                            stream_id = %id,
                            current_version = next_version,
                            total_bytes = size_bytes,
                            batches_in_window = batches_since_checkpoint,
                            bytes_in_window = bytes_since_checkpoint,
                        )
                        .entered();
                        batches_since_checkpoint = 0;
                        bytes_since_checkpoint = 0;
                        last_checkpoint = Instant::now();
                    }
                }

                if response_sender.send(Ok(response)).await.is_err() {
                    info!(stream_id = id, "Client dropped.");
                    COUNTER
                        .with_label_values(&["live_data_service_client_dropped"])
                        .inc();
                    termination_reason = "client_dropped";
                    break;
                }
            } else {
                let err = Err(Status::not_found("Requested data is too old."));
                info!(stream_id = id, "Client error: {err:?}.");
                let _ = response_sender.send(err).await;
                COUNTER
                    .with_label_values(&["terminate_requested_data_too_old"])
                    .inc();
                termination_reason = "data_too_old";
                break;
            }
        }

        stream_span.record("stream.total_bytes_sent", size_bytes);
        stream_span.record("stream.final_version", next_version);
        stream_span.record("stream.termination_reason", termination_reason);

        self.connection_manager
            .update_stream_progress(&id, next_version, size_bytes);
        self.connection_manager.remove_active_stream(&id);
    }

    fn get_known_latest_version(&self) -> u64 {
        self.connection_manager.known_latest_version()
    }
}
