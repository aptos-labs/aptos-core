// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    config::HistoricalDataServiceConfig,
    connection_manager::ConnectionManager,
    metrics::{COUNTER, TIMER},
    service::StreamRequest,
};
use aptos_indexer_grpc_utils::{
    constants::{get_request_metadata, IndexerGrpcRequestMetadata},
    counters::BYTES_READY_TO_TRANSFER_FROM_SERVER_AFTER_STRIPPING,
    file_store_operator_v2::file_store_reader::FileStoreReader,
    filter_utils,
};
use aptos_protos::indexer::v1::{ProcessedRange, TransactionsResponse};
use aptos_transaction_filter::BooleanTransactionFilter;
use futures::executor::block_on;
use prost::Message;
use std::{
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::sync::mpsc::{channel, Receiver};
use tonic::Status;
use tracing::info;
use tracing_opentelemetry::OpenTelemetrySpanExt;

const DEFAULT_MAX_NUM_TRANSACTIONS_PER_BATCH: usize = 10000;

pub struct HistoricalDataService {
    chain_id: u64,
    connection_manager: Arc<ConnectionManager>,
    file_store_reader: Arc<FileStoreReader>,
    max_transaction_filter_size_bytes: usize,
    checkpoint_interval: Option<Duration>,
}

impl HistoricalDataService {
    pub fn new(
        chain_id: u64,
        config: HistoricalDataServiceConfig,
        connection_manager: Arc<ConnectionManager>,
        max_transaction_filter_size_bytes: usize,
        checkpoint_interval_secs: u64,
    ) -> Self {
        let file_store = block_on(config.file_store_config.create_filestore());
        let file_store_reader = Arc::new(block_on(FileStoreReader::new(chain_id, file_store)));
        let checkpoint_interval = if checkpoint_interval_secs > 0 {
            Some(Duration::from_secs(checkpoint_interval_secs))
        } else {
            None
        };
        Self {
            chain_id,
            connection_manager: connection_manager.clone(),
            file_store_reader,
            max_transaction_filter_size_bytes,
            checkpoint_interval,
        }
    }

    pub fn run(&self, mut handler_rx: Receiver<StreamRequest>) {
        info!("Running HistoricalDataService...");
        tokio_scoped::scope(|scope| {
            while let Some((request, response_sender, parent_cx)) = handler_rx.blocking_recv() {
                COUNTER
                    .with_label_values(&["historical_data_service_receive_request"])
                    .inc();
                // Extract request metadata before consuming the request.
                let request_metadata = Arc::new(get_request_metadata(&request));
                let request = request.into_inner();
                let id = request_metadata.request_connection_id.clone();
                info!("Received request: {request:?}.");

                if request.starting_version.is_none() {
                    let err = Err(Status::invalid_argument("Must provide starting_version."));
                    info!("Client error: {err:?}.");
                    let _ = response_sender.blocking_send(err);
                    COUNTER
                        .with_label_values(&["historical_data_service_invalid_request"])
                        .inc();
                    continue;
                }
                let starting_version = request.starting_version.unwrap();

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
                                .with_label_values(&["historical_data_service_invalid_filter"])
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
                    DEFAULT_MAX_NUM_TRANSACTIONS_PER_BATCH
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

    pub(crate) fn get_connection_manager(&self) -> &ConnectionManager {
        &self.connection_manager
    }

    async fn start_streaming(
        &self,
        id: String,
        starting_version: u64,
        ending_version: Option<u64>,
        max_num_transactions_per_batch: usize,
        filter: Option<BooleanTransactionFilter>,
        request_metadata: Arc<IndexerGrpcRequestMetadata>,
        response_sender: tokio::sync::mpsc::Sender<Result<TransactionsResponse, Status>>,
        parent_cx: opentelemetry::Context,
    ) {
        COUNTER
            .with_label_values(&["historical_data_service_new_stream"])
            .inc();

        let stream_span = tracing::info_span!(
            "historical_stream",
            stream_id = %id,
            stream.starting_version = starting_version,
            stream.ending_version = ?ending_version,
            stream.service_type = "historical",
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
        let ending_version = ending_version.unwrap_or(u64::MAX);
        let mut size_bytes: u64 = 0;

        let mut last_checkpoint = Instant::now();
        let mut batches_since_checkpoint: u64 = 0;
        let mut bytes_since_checkpoint: u64 = 0;
        let mut termination_reason = "ending_version_reached";

        'out: loop {
            self.connection_manager
                .update_stream_progress(&id, next_version, size_bytes);
            if next_version >= ending_version {
                break;
            }

            if !self.file_store_reader.can_serve(next_version).await {
                info!(stream_id = id, "next_version {next_version} is larger or equal than file store version, terminate the stream.");
                termination_reason = "data_unavailable";
                break;
            }

            // TODO(grao): Pick a better channel size here, and consider doing parallel fetching
            // inside the `get_transaction_batch` call based on the channel size.
            let (tx, mut rx) = channel(1);

            let file_store_reader = self.file_store_reader.clone();
            let filter = filter.clone();
            tokio::spawn(async move {
                file_store_reader
                    .get_transaction_batch(
                        next_version,
                        /*retries=*/ 3,
                        /*max_files=*/ None,
                        filter,
                        Some(ending_version),
                        tx,
                    )
                    .await;
            });

            let mut close_to_latest = false;
            while let Some((
                transactions,
                batch_size_bytes,
                timestamp,
                (first_processed_version, last_processed_version),
            )) = rx.recv().await
            {
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

                let timestamp_since_epoch =
                    Duration::new(timestamp.seconds as u64, timestamp.nanos as u32);
                let now_since_epoch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
                let delta = now_since_epoch.saturating_sub(timestamp_since_epoch);

                // TODO(grao): Double check if this threshold makes sense.
                if delta < Duration::from_secs(60) {
                    close_to_latest = true;
                }

                let responses = if !transactions.is_empty() {
                    let mut current_version = first_processed_version;
                    let mut responses: Vec<_> = transactions
                        .chunks(max_num_transactions_per_batch)
                        .map(|chunk| {
                            let first_version = current_version;
                            let last_version = chunk.last().unwrap().version;
                            current_version = last_version + 1;
                            TransactionsResponse {
                                transactions: chunk.to_vec(),
                                chain_id: Some(self.chain_id),
                                processed_range: Some(ProcessedRange {
                                    first_version,
                                    last_version,
                                }),
                            }
                        })
                        .collect();
                    responses
                        .last_mut()
                        .unwrap()
                        .processed_range
                        .as_mut()
                        .unwrap()
                        .last_version = last_processed_version;
                    responses
                } else {
                    vec![TransactionsResponse {
                        transactions: vec![],
                        chain_id: Some(self.chain_id),
                        processed_range: Some(ProcessedRange {
                            first_version: first_processed_version,
                            last_version: last_processed_version,
                        }),
                    }]
                };

                for response in responses {
                    let _timer = TIMER
                        .with_label_values(&["historical_data_service_send_batch"])
                        .start_timer();
                    let bytes_ready_to_transfer_after_stripping = response
                        .transactions
                        .iter()
                        .map(|t| t.encoded_len())
                        .sum::<usize>();
                    BYTES_READY_TO_TRANSFER_FROM_SERVER_AFTER_STRIPPING
                        .with_label_values(&request_metadata.get_label_values())
                        .inc_by(bytes_ready_to_transfer_after_stripping as u64);
                    if response_sender.send(Ok(response)).await.is_err() {
                        info!(stream_id = id, "Client dropped.");
                        COUNTER
                            .with_label_values(&["historical_data_service_client_dropped"])
                            .inc();
                        termination_reason = "client_dropped";
                        break 'out;
                    }
                }
            }
            if close_to_latest {
                info!(
                    stream_id = id,
                    "Stream is approaching to the latest transactions, terminate."
                );
                COUNTER
                    .with_label_values(&["terminate_close_to_latest"])
                    .inc();
                termination_reason = "close_to_latest";
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
}
