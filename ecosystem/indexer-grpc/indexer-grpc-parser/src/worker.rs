// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    processors::{
        coin_processor::CoinTransactionProcessor,
        default_processor::DefaultTransactionProcessor,
        processor_trait::{ProcessingResult, ProcessorTrait},
        stake_processor::StakeTransactionProcessor,
        token_processor::TokenTransactionProcessor,
        Processor,
    },
    utils::{
        counters::{
            LATEST_PROCESSED_VERSION, PROCESSOR_ERRORS, PROCESSOR_INVOCATIONS, PROCESSOR_SUCCESSES,
        },
        database::new_db_pool,
    },
};
use aptos_logger::{error, info};
use aptos_moving_average::MovingAverage;
use aptos_protos::{
    datastream::v1::{
        indexer_stream_client::IndexerStreamClient, raw_datastream_response::Response,
        RawDatastreamRequest,
    },
    transaction::testing1::v1::Transaction as TransactionProto,
};
use diesel::{
    pg::PgConnection,
    r2d2::{ConnectionManager, PooledConnection},
};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use futures::StreamExt;
use prost::Message;
use std::sync::Arc;

pub type PgPool = diesel::r2d2::Pool<ConnectionManager<PgConnection>>;
pub type PgDbPool = Arc<PgPool>;
pub type PgPoolConnection = PooledConnection<ConnectionManager<PgConnection>>;
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

// TODO: Will replace these with yaml config
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

fn get_processor_name() -> String {
    std::env::var("PROCESSOR_NAME").expect("PROCESSOR_NAME is required.")
}

fn get_ans_address() -> Option<String> {
    std::env::var("ANS_ADDRESS").ok()
}

pub struct Worker {
    pub db_pool: PgDbPool,
    pub datastream_service_address: String,
    pub postgres_uri: String,
}

impl Worker {
    pub async fn new() -> Self {
        let processor_name = get_processor_name();
        info!(processor_name = processor_name, "[Parser] Kicking off");

        let postgres_uri = get_postgres_connection_string();
        info!(
            processor_name = processor_name,
            "[Parser] Creating connection pool"
        );
        let conn_pool = new_db_pool(&postgres_uri).expect("Failed to create connection pool");
        info!(
            processor_name = processor_name,
            "[Parser] Finish creating the connection pool"
        );

        Self {
            db_pool: conn_pool,
            datastream_service_address: get_datastream_service_address(),
            postgres_uri,
        }
    }

    pub async fn run(&self) {
        let processor_name = get_processor_name();
        info!(
            processor_name = processor_name,
            "[Parser] Running migrations"
        );
        self.run_migrations();
        info!(
            processor_name = processor_name,
            "[Parser] Finished migrations"
        );
        // Connecting once to the RPC client.
        // TODO: What happens if the connection is lost?
        info!(
            processor_name = processor_name,
            stream_address = self.datastream_service_address.clone(),
            "[Parser] Connecting to GRPC endpoint",
        );
        let mut rpc_client =
            match IndexerStreamClient::connect(self.datastream_service_address.clone()).await {
                Ok(client) => client,
                Err(e) => {
                    error!(
                        processor_name = processor_name,
                        stream_address = self.datastream_service_address.clone(),
                        error = ?e,
                        "[Parser] Error connecting to grpc_stream"
                    );
                    panic!();
                },
            };
        info!(
            processor_name = processor_name,
            stream_address = self.datastream_service_address.clone(),
            "[Parser] Connected to GRPC endpoint",
        );
        let starting_version = get_starting_version();
        info!(
            processor_name = processor_name,
            stream_address = self.datastream_service_address.clone(),
            starting_version = starting_version,
            "[Parser] Making request to GRPC endpoint",
        );
        // TODO: CHECK CHAIN ID.
        // TODO: Loads from the recent successful starting version.
        let request = tonic::Request::new(RawDatastreamRequest {
            starting_version,
            transactions_count: None,
        });
        let response = rpc_client.raw_datastream(request).await.unwrap();
        let mut resp_stream = response.into_inner();

        let concurrent_tasks = get_concurrent_tasks();
        info!(
            processor_name = processor_name,
            stream_address = self.datastream_service_address.clone(),
            starting_version = starting_version,
            concurrent_tasks = concurrent_tasks,
            "[Parser] Successfully connected to GRPC endpoint. Now instantiating processor",
        );

        // Instantiates correct processor based on config
        let processor_enum = Processor::from_string(&processor_name);
        let processor: Arc<dyn ProcessorTrait> = match processor_enum {
            Processor::CoinProcessor => {
                Arc::new(CoinTransactionProcessor::new(self.db_pool.clone()))
            },
            Processor::DefaultProcessor => {
                Arc::new(DefaultTransactionProcessor::new(self.db_pool.clone()))
            },
            Processor::TokenProcessor => Arc::new(TokenTransactionProcessor::new(
                self.db_pool.clone(),
                get_ans_address(),
            )),
            Processor::StakeProcessor => {
                Arc::new(StakeTransactionProcessor::new(self.db_pool.clone()))
            },
        };
        let processor_name = processor.name();

        let mut ma = MovingAverage::new(10_000);
        info!(processor_name = processor_name, "[Parser] Starting stream");
        loop {
            let mut transactions_batches = vec![];
            // Gets a batch of transactions from the stream. Batch size is set in the grpc server.
            // The number of batches depends on our config
            for _ in 0..concurrent_tasks {
                let next_stream = match resp_stream.next().await {
                    Some(Ok(r)) => r,
                    Some(Err(e)) => {
                        // TODO: If the connection is lost, reconnect.
                        error!(
                            processor_name = processor_name,
                            error = ?e,
                            "[Parser] Error receiving datastream response"
                        );
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
                info!(
                    processor_name = processor_name,
                    "[Parser] Channel is empty now."
                );
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                continue;
            }

            // Process the transactions in parallel
            let mut tasks = vec![];
            for transactions in transactions_batches {
                let processor_clone = processor.clone();
                let task = tokio::spawn(async move {
                    let start_version = transactions.as_slice().first().unwrap().version;
                    let end_version = transactions.as_slice().last().unwrap().version;

                    PROCESSOR_INVOCATIONS
                        .with_label_values(&[processor_name])
                        .inc();
                    processor_clone
                        .process_transactions(transactions, start_version, end_version)
                        .await
                });
                tasks.push(task);
            }
            let batches = match futures::future::try_join_all(tasks).await {
                Ok(res) => res,
                Err(err) => panic!("[Parser] Error processing transaction batches: {:?}", err),
            };

            // Update states depending on results of the batch processing
            let mut processed_versions = vec![];
            for res in batches {
                let processed: ProcessingResult = match res {
                    Ok(versions) => {
                        PROCESSOR_SUCCESSES
                            .with_label_values(&[processor_name])
                            .inc();
                        versions
                    },
                    Err(e) => {
                        error!(
                            processor_name = processor_name,
                            stream_address = self.datastream_service_address.clone(),
                            error = ?e,
                            "[Parser] Error processing transactions"
                        );
                        PROCESSOR_ERRORS.with_label_values(&[processor_name]).inc();
                        panic!();
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
                        error!(
                            processor_name = processor_name,
                            stream_address = self.datastream_service_address.clone(),
                            processed_versions = processed_versions_sorted,
                            "[Parser] Gaps in processing stream"
                        );
                        panic!();
                    }
                    prev_start = Some(start);
                    prev_end = Some(end);
                }
            }
            let batch_start = processed_versions_sorted.first().unwrap().0;
            let batch_end = processed_versions_sorted.last().unwrap().1;

            LATEST_PROCESSED_VERSION
                .with_label_values(&[processor_name])
                .set(batch_end as i64);
            processor
                .update_last_processed_version(batch_end)
                .await
                .unwrap();

            ma.tick_now(batch_end - batch_start + 1);
            info!(
                processor_name = processor_name,
                start_version = batch_start,
                end_version = batch_end,
                batch_size = batch_end - batch_start + 1,
                tps = (ma.avg() * 1000.0) as u64,
                "[Parser] Processed transactions.",
            );
        }
    }

    fn run_migrations(&self) {
        let _ = &self
            .db_pool
            .get()
            .expect("Could not get connection for migrations")
            .run_pending_migrations(MIGRATIONS)
            .expect("migrations failed!");
    }
}
