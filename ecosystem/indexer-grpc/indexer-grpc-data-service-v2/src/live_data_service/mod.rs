// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

mod data_client;
mod data_manager;
mod fetch_manager;
mod in_memory_cache;

use crate::{
    config::LiveDataServiceConfig,
    connection_manager::ConnectionManager,
    live_data_service::in_memory_cache::InMemoryCache,
    metrics::{COUNTER, TIMER},
};
use aptos_protos::indexer::v1::{GetTransactionsRequest, ProcessedRange, TransactionsResponse};
use aptos_transaction_filter::BooleanTransactionFilter;
use std::{sync::Arc, time::Duration};
use tokio::sync::mpsc::{Receiver, Sender};
use tonic::{Request, Status};
use tracing::info;
use uuid::Uuid;

const MAX_BYTES_PER_BATCH: usize = 20 * (1 << 20);

pub struct LiveDataService<'a> {
    chain_id: u64,
    in_memory_cache: InMemoryCache<'a>,
    connection_manager: Arc<ConnectionManager>,
    max_transaction_filter_size_bytes: usize,
}

impl<'a> LiveDataService<'a> {
    pub fn new(
        chain_id: u64,
        config: LiveDataServiceConfig,
        connection_manager: Arc<ConnectionManager>,
        max_transaction_filter_size_bytes: usize,
    ) -> Self {
        let known_latest_version = connection_manager.known_latest_version();
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
        }
    }

    pub fn run(
        &'a self,
        mut handler_rx: Receiver<(
            Request<GetTransactionsRequest>,
            Sender<Result<TransactionsResponse, Status>>,
        )>,
    ) {
        info!("Running LiveDataService...");
        tokio_scoped::scope(|scope| {
            scope.spawn(async move {
                let _ = self
                    .in_memory_cache
                    .fetch_manager
                    .continuously_fetch_latest_data()
                    .await;
            });
            while let Some((request, response_sender)) = handler_rx.blocking_recv() {
                COUNTER
                    .with_label_values(&["live_data_service_receive_request"])
                    .inc();
                // TODO(grao): Store request metadata.
                let request = request.into_inner();
                let id = Uuid::new_v4().to_string();
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
                    match BooleanTransactionFilter::new_from_proto(
                        proto_filter,
                        Some(self.max_transaction_filter_size_bytes),
                    ) {
                        Ok(filter) => Some(filter),
                        Err(e) => {
                            let err = Err(Status::invalid_argument(format!(
                                "Invalid transaction_filter: {e:?}."
                            )));
                            info!("Client error: {err:?}.");
                            let _ = response_sender.blocking_send(err);
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
                    .map(|count| starting_version + count);

                scope.spawn(async move {
                    self.start_streaming(
                        id,
                        starting_version,
                        ending_version,
                        max_num_transactions_per_batch,
                        MAX_BYTES_PER_BATCH,
                        filter,
                        response_sender,
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
        response_sender: tokio::sync::mpsc::Sender<Result<TransactionsResponse, Status>>,
    ) {
        COUNTER
            .with_label_values(&["live_data_service_new_stream"])
            .inc();
        info!(stream_id = id, "Start streaming, starting_version: {starting_version}, ending_version: {ending_version:?}.");
        self.connection_manager
            .insert_active_stream(&id, starting_version, ending_version);
        let mut next_version = starting_version;
        let mut size_bytes = 0;
        let ending_version = ending_version.unwrap_or(u64::MAX);
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
                next_version = last_processed_version + 1;
                size_bytes += batch_size_bytes as u64;
                if response_sender.send(Ok(response)).await.is_err() {
                    info!(stream_id = id, "Client dropped.");
                    COUNTER
                        .with_label_values(&["live_data_service_client_dropped"])
                        .inc();
                    break;
                }
            } else {
                let err = Err(Status::not_found("Requested data is too old."));
                info!(stream_id = id, "Client error: {err:?}.");
                let _ = response_sender.send(err).await;
                COUNTER
                    .with_label_values(&["terminate_requested_data_too_old"])
                    .inc();
                break;
            }
        }

        self.connection_manager
            .update_stream_progress(&id, next_version, size_bytes);
        self.connection_manager.remove_active_stream(&id);
    }

    fn get_known_latest_version(&self) -> u64 {
        self.connection_manager.known_latest_version()
    }
}
