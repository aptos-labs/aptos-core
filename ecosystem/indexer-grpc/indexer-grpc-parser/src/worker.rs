// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::processors::{
    default_processor::DefaultTransactionProcessor,
    processor_trait::{ProcessingResult, ProcessorTrait},
};
use diesel::{
    pg::PgConnection,
    r2d2::{ConnectionManager, PooledConnection},
};
use std::sync::Arc;

pub type PgPool = diesel::r2d2::Pool<ConnectionManager<PgConnection>>;
pub type PgDbPool = Arc<PgPool>;
pub type PgPoolConnection = PooledConnection<ConnectionManager<PgConnection>>;

use aptos_logger::{error, info};
use aptos_moving_average::MovingAverage;
use aptos_protos::{
    datastream::v1::{
        indexer_stream_client::IndexerStreamClient, raw_datastream_response::Response,
        RawDatastreamRequest,
    },
    transaction::testing1::v1::Transaction as TransactionProto,
};
use futures::StreamExt;
use prost::Message;

// Will replace these with yaml config
fn get_datastream_service_address() -> String {
    std::env::var("GRPC_ADDRESS").expect("GRPC_ADDRESS is required.")
}

fn get_postgres_connection_string() -> String {
    std::env::var("DATABASE_URI").expect("DATABASE_URI is required.")
}

fn get_starting_version() -> u64 {
    std::env::var("STARTING_VERSION")
        .expect("STARTING_VERSION is required.")
        .parse::<u64>()
        .unwrap()
}

fn get_concurrent_tasks() -> u64 {
    std::env::var("NUM_CONCURRENT_TASKS")
        .expect("NUM_CONCURRENT_TASKS is required.")
        .parse::<u64>()
        .unwrap()
}

pub struct Worker {
    pub db_pool: PgDbPool,
    pub datastream_service_address: String,
    pub postgres_uri: String,
}

impl Worker {
    pub async fn new() -> Self {
        let postgres_uri = get_postgres_connection_string();
        let manager = ConnectionManager::<PgConnection>::new(postgres_uri.clone());
        let pg_pool = PgPool::builder().build(manager).map(Arc::new);
        Self {
            db_pool: pg_pool.unwrap(),
            datastream_service_address: get_datastream_service_address(),
            postgres_uri,
        }
    }

    pub async fn run(&self) {
        // Connecting once to the RPC client.
        // TODO: What happens if the connection is lost?
        let mut rpc_client =
            match IndexerStreamClient::connect(self.datastream_service_address.clone()).await {
                Ok(client) => client,
                Err(e) => {
                    panic!(
                        "[Parser] Error connecting to grpc_stream: {}. Error: {:?}",
                        self.datastream_service_address.clone(),
                        e,
                    );
                },
            };
        info!(
            "Connected to GRPC endpoint at {}.",
            self.datastream_service_address.clone(),
        );
        let starting_version = get_starting_version();
        let request = tonic::Request::new(RawDatastreamRequest {
            // Loads from the recent successful starting version.
            starting_version,
            transactions_count: None,
        });
        let response = rpc_client.raw_datastream(request).await.unwrap();
        let mut resp_stream = response.into_inner();

        // TODO: Add code for other processors
        let concurrent_tasks = get_concurrent_tasks();
        let default_transaction_processor: Arc<dyn ProcessorTrait> =
            Arc::new(DefaultTransactionProcessor::new(self.db_pool.clone()));

        let mut ma = MovingAverage::new(10_000);
        loop {
            let mut transactions_batches = vec![];
            // Gets a batch of transactions from the stream. Batch size is set in the grpc server.
            // The number of batches depends on our config
            for _ in 0..concurrent_tasks {
                let next_stream = match resp_stream.next().await {
                    Some(Ok(r)) => r,
                    Some(Err(e)) => {
                        // TODO: If the connection is lost, reconnect.
                        error!("[Parser] Error receiving datastream response: {}", e);
                        break;
                    },
                    None => {
                        // If no next stream wait a bit and try again
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        continue;
                    },
                };
                // We only care about stream with transactions
                let transactions = if let Response::Data(txns) = next_stream.response.unwrap() {
                    txns.transactions
                        .into_iter()
                        .map(|e| {
                            let txn_raw = base64::decode(e.encoded_proto_data).unwrap();
                            TransactionProto::decode(&*txn_raw).unwrap()
                        })
                        .collect::<Vec<TransactionProto>>()
                } else {
                    continue;
                };
                // If stream is somehow empty wait a bit and try again
                if !transactions.is_empty() {
                    transactions_batches.push(transactions);
                }
            }

            // If stream is somehow empty wait a bit and try again
            if transactions_batches.is_empty() {
                info!("[Parser] Channel is empty now.");
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                continue;
            }

            // Process the transactions in parallel
            let mut tasks = vec![];
            for transactions in transactions_batches {
                let processor_clone = default_transaction_processor.clone();
                let task = tokio::spawn(async move {
                    let start_version = transactions.as_slice().first().unwrap().version;
                    let end_version = transactions.as_slice().last().unwrap().version;

                    processor_clone
                        .process_transactions(transactions, start_version, end_version)
                        .await
                });
                tasks.push(task);
            }
            let batches = match futures::future::try_join_all(tasks).await {
                Ok(res) => res,
                Err(err) => panic!("Error processing transaction batches: {:?}", err),
            };

            // Update states depending on results of the batch processing
            let mut processed_versions = vec![];
            for res in batches {
                let processed: ProcessingResult = match res {
                    Ok(versions) => versions,
                    Err(error) => {
                        panic!("[Parser] Error processing transactions. Error: {:?}", error);
                    },
                };
                processed_versions.push(processed);
            }

            // Make sure there are no gaps and advance states
            processed_versions.sort();
            let mut prev_start = None;
            let mut prev_end = None;
            let processed_versions_sorted = processed_versions.clone();
            for (start, end) in processed_versions {
                if prev_start.is_none() {
                    prev_start = Some(start);
                    prev_end = Some(end);
                } else {
                    if prev_end.unwrap() + 1 != start {
                        panic!(
                            "[Parser] Gaps with processed versions {:?}",
                            processed_versions_sorted
                        );
                    }
                    prev_start = Some(start);
                    prev_end = Some(end);
                }
            }
            let batch_start = processed_versions_sorted.first().unwrap().0;
            let batch_end = processed_versions_sorted.last().unwrap().1;

            default_transaction_processor
                .update_last_processed_version(batch_end)
                .await
                .unwrap();

            ma.tick_now(batch_end - batch_start + 1);
            info!(
                start_version = batch_start,
                end_version = batch_end,
                batch_size = batch_end - batch_start + 1,
                tps = (ma.avg() * 1000.0) as u64,
                "[Parser] Processed transactions.",
            );
        }
    }
}
