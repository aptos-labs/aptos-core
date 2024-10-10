// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{config::HistoricalDataServiceConfig, connection_manager::ConnectionManager};
use aptos_indexer_grpc_utils::file_store_operator_v2::file_store_reader::FileStoreReader;
use aptos_protos::indexer::v1::{GetTransactionsRequest, TransactionsResponse};
use futures::executor::block_on;
use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tonic::{Request, Status};
use tracing::info;
use uuid::Uuid;

const DEFAULT_MAX_NUM_TRANSACTIONS_PER_BATCH: usize = 10000;

pub struct HistoricalDataService {
    chain_id: u64,
    connection_manager: Arc<ConnectionManager>,
    file_store_reader: Arc<FileStoreReader>,
}

impl HistoricalDataService {
    pub fn new(
        chain_id: u64,
        config: HistoricalDataServiceConfig,
        connection_manager: Arc<ConnectionManager>,
    ) -> Self {
        let file_store = block_on(config.file_store_config.create_filestore());
        let file_store_reader = Arc::new(block_on(FileStoreReader::new(chain_id, file_store)));
        Self {
            chain_id,
            connection_manager: connection_manager.clone(),
            file_store_reader,
        }
    }

    pub fn run(
        &self,
        mut handler_rx: Receiver<(
            Request<GetTransactionsRequest>,
            Sender<Result<TransactionsResponse, Status>>,
        )>,
    ) {
        info!("Running HistoricalDataService...");
        tokio_scoped::scope(|scope| {
            while let Some((request, response_sender)) = handler_rx.blocking_recv() {
                // TODO(grao): Store request metadata.
                let request = request.into_inner();
                // TODO(grao): We probably should have a more stable id from the client side.
                let id = Uuid::new_v4().to_string();
                info!("Received request: {request:?}.");

                if request.starting_version.is_none() {
                    let err = Err(Status::invalid_argument("Must provide starting_version."));
                    info!("Client error: {err:?}.");
                    let _ = response_sender.blocking_send(err);
                    continue;
                }
                let starting_version = request.starting_version.unwrap();

                let max_num_transactions_per_batch = if let Some(batch_size) = request.batch_size {
                    batch_size as usize
                } else {
                    DEFAULT_MAX_NUM_TRANSACTIONS_PER_BATCH
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
                        response_sender,
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
        response_sender: tokio::sync::mpsc::Sender<Result<TransactionsResponse, Status>>,
    ) {
        info!(stream_id = id, "Start streaming, starting_version: {starting_version}, ending_version: {ending_version:?}.");
        self.connection_manager
            .insert_active_stream(&id, starting_version, ending_version);
        let mut next_version = starting_version;
        let ending_version = ending_version.unwrap_or(u64::MAX);
        let mut size_bytes = 0;
        'out: loop {
            self.connection_manager
                .update_stream_progress(&id, next_version, size_bytes);
            if next_version >= ending_version {
                break;
            }

            if !self.file_store_reader.can_serve(next_version).await {
                info!(stream_id = id, "next_version {next_version} is larger or equal than file store version, terminate the stream.");
                break;
            }

            // TODO(grao): Pick a better channel size here, and consider doing parallel fetching
            // inside the `get_transaction_batch` call based on the channel size.
            let (tx, mut rx) = channel(1);

            let file_store_reader = self.file_store_reader.clone();
            tokio::spawn(async move {
                file_store_reader
                    .get_transaction_batch(
                        next_version,
                        /*retries=*/ 3,
                        /*max_files=*/ None,
                        tx,
                    )
                    .await;
            });

            let mut close_to_latest = false;
            while let Some((transactions, batch_size_bytes)) = rx.recv().await {
                next_version += transactions.len() as u64;
                size_bytes += batch_size_bytes as u64;
                let timestamp = transactions.first().unwrap().timestamp.unwrap();
                let timestamp_since_epoch =
                    Duration::new(timestamp.seconds as u64, timestamp.nanos as u32);
                let now_since_epoch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
                let delta = now_since_epoch.saturating_sub(timestamp_since_epoch);

                if delta < Duration::from_secs(60) {
                    close_to_latest = true;
                }
                let responses = transactions
                    .chunks(max_num_transactions_per_batch)
                    .map(|chunk| TransactionsResponse {
                        transactions: chunk.to_vec(),
                        chain_id: Some(self.chain_id),
                    });
                for response in responses {
                    if response_sender.send(Ok(response)).await.is_err() {
                        // NOTE: We are not recalculating the version and size_bytes for the stream
                        // progress since nobody cares about the accurate if client has dropped the
                        // connection.
                        info!(stream_id = id, "Client dropped.");
                        break 'out;
                    }
                }
            }
            if close_to_latest {
                info!(
                    stream_id = id,
                    "Stream is approaching to the latest transactions, terminate."
                );
                break;
            }
        }

        self.connection_manager
            .update_stream_progress(&id, next_version, size_bytes);
        self.connection_manager.remove_active_stream(&id);
    }
}
