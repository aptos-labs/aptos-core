// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    models::{ledger_info::LedgerInfo, processor_status::ProcessorStatusQuery},
    processors::{
        coin_processor::CoinTransactionProcessor,
        default_processor::DefaultTransactionProcessor,
        processor_trait::{ProcessingResult, ProcessorTrait},
        stake_processor::StakeTransactionProcessor,
        token_processor::TokenTransactionProcessor,
        Processor,
    },
    schema::ledger_infos,
    utils::{
        counters::{
            LATEST_PROCESSED_VERSION, PROCESSOR_ERRORS, PROCESSOR_INVOCATIONS, PROCESSOR_SUCCESSES,
        },
        database::{execute_with_better_error, new_db_pool},
    },
};
use anyhow::Context;
use aptos_indexer_grpc_utils::config::IndexerGrpcProcessorConfig;
use aptos_logger::{error, info};
use aptos_moving_average::MovingAverage;
use aptos_protos::{
    datastream::v1::{
        indexer_stream_client::IndexerStreamClient,
        raw_datastream_response::{self, Response},
        RawDatastreamRequest, RawDatastreamResponse,
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

pub struct Worker {
    pub db_pool: PgDbPool,
    pub config: IndexerGrpcProcessorConfig,
}

impl Worker {
    pub async fn new(config: IndexerGrpcProcessorConfig) -> Self {
        let processor_name = config.processor_name.clone();
        info!(processor_name = processor_name, "[Parser] Kicking off");

        let postgres_uri = config.postgres_connection_string.clone();
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
            config,
        }
    }

    pub async fn run(&self) {
        let processor_name = self.config.processor_name.clone();

        info!(
            processor_name = processor_name,
            stream_address = self.config.indexer_grpc_address.clone(),
            "[Parser] Connecting to GRPC endpoint",
        );

        let mut rpc_client = match IndexerStreamClient::connect(format!(
            "http://{}",
            self.config.indexer_grpc_address.clone()
        ))
        .await
        {
            Ok(client) => client,
            Err(e) => {
                error!(
                    processor_name = processor_name,
                    stream_address = self.config.indexer_grpc_address.clone(),
                    error = ?e,
                    "[Parser] Error connecting to grpc_stream"
                );
                panic!();
            },
        };
        info!(
            processor_name = processor_name,
            stream_address = self.config.indexer_grpc_address.clone(),
            "[Parser] Connected to GRPC endpoint",
        );

        info!(
            processor_name = processor_name,
            "[Parser] Running migrations"
        );
        self.run_migrations();
        info!(
            processor_name = processor_name,
            "[Parser] Finished migrations"
        );

        let starting_version_from_db = self
            .get_start_version()
            .expect("Database error when getting starting version")
            .unwrap_or_else(|| {
                info!(
                    processor_name = processor_name,
                    "No starting version from db so starting from version 0"
                );
                0
            });

        let starting_version = match self.config.starting_version {
            None => starting_version_from_db,
            Some(version) => version,
        };

        info!(
            processor_name = processor_name,
            stream_address = self.config.indexer_grpc_address.clone(),
            final_start_version = starting_version,
            start_version_from_config = self.config.starting_version,
            start_version_from_db = starting_version_from_db,
            "[Parser] Making request to GRPC endpoint",
        );

        let request = tonic::Request::new(RawDatastreamRequest {
            starting_version,
            transactions_count: None,
        });
        let response = rpc_client.raw_datastream(request).await.unwrap();
        let mut resp_stream = response.into_inner();

        let concurrent_tasks = self.config.number_concurrent_processing_tasks;
        info!(
            processor_name = processor_name,
            stream_address = self.config.indexer_grpc_address.clone(),
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
                self.config.ans_address.clone(),
            )),
            Processor::StakeProcessor => {
                Arc::new(StakeTransactionProcessor::new(self.db_pool.clone()))
            },
        };
        let processor_name = processor.name();

        let mut ma = MovingAverage::new(10_000);
        info!(processor_name = processor_name, "[Parser] Starting stream");
        match resp_stream.next().await {
            Some(Ok(r)) => {
                self.validate_grpc_chain_id(r)
                    .await
                    .expect("Invalid grpc response with INIT frame.");
            },
            _ => {
                error!(
                    processor_name = processor_name,
                    "[Parser] Error receiving datastream response"
                );
                panic!();
            },
        }

        loop {
            let mut transactions_batches = vec![];
            // Gets a batch of transactions from the stream. Batch size is set in the grpc server.
            // The number of batches depends on our config
            for _ in 0..concurrent_tasks {
                // TODO(larry): do not block here to wait for consumer items.
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
                            stream_address = self.config.indexer_grpc_address.clone(),
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
                            stream_address = self.config.indexer_grpc_address.clone(),
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

    /// Gets the start version for the processor. If not found, start from 0.
    pub fn get_start_version(&self) -> anyhow::Result<Option<u64>> {
        let mut conn = self.db_pool.get()?;

        match ProcessorStatusQuery::get_by_processor(&self.config.processor_name, &mut conn)? {
            Some(status) => Ok(Some(status.last_success_version as u64 + 1)),
            None => Ok(None),
        }
    }

    /// Verify the chain id from GRPC against the database.
    pub async fn check_or_update_chain_id(&self, grpc_chain_id: i64) -> anyhow::Result<u64> {
        info!(
            processor_name = self.config.processor_name.as_str(),
            "Checking if chain id is correct"
        );
        let mut conn = self.db_pool.get()?;

        let maybe_existing_chain_id = LedgerInfo::get(&mut conn)?.map(|li| li.chain_id);

        match maybe_existing_chain_id {
            Some(chain_id) => {
                anyhow::ensure!(chain_id == grpc_chain_id, "Wrong chain detected! Trying to index chain {} now but existing data is for chain {}", grpc_chain_id, chain_id);
                info!(
                    processor_name = self.config.processor_name.as_str(),
                    chain_id = chain_id,
                    "Chain id matches! Continue to index...",
                );
                Ok(chain_id as u64)
            },
            None => {
                info!(
                    processor_name = self.config.processor_name.as_str(),
                    chain_id = grpc_chain_id,
                    "Adding chain id to db, continue to index.."
                );
                execute_with_better_error(
                    &mut conn,
                    diesel::insert_into(ledger_infos::table).values(LedgerInfo {
                        chain_id: grpc_chain_id,
                    }),
                    None,
                )
                .context(r#"Error updating chain_id!"#)
                .map(|_| grpc_chain_id as u64)
            },
        }
    }

    /// GRPC validation
    pub async fn validate_grpc_chain_id(
        &self,
        init_signal: RawDatastreamResponse,
    ) -> anyhow::Result<()> {
        match init_signal.response {
            Some(raw_datastream_response::Response::Status(_)) => {
                let grpc_chain_id = init_signal.chain_id;
                let _chain_id = self.check_or_update_chain_id(grpc_chain_id as i64).await?;
                Ok(())
            },
            _ => anyhow::bail!("Grpc first response is not a init signal"),
        }
    }
}
