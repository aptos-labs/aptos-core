// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    database::new_db_pool,
    indexer::{
        fetcher::TransactionFetcherOptions, processing_result::ProcessingResult, tailer::Tailer,
        transaction_processor::TransactionProcessor,
    },
    processors::{
        coin_processor::CoinTransactionProcessor, default_processor::DefaultTransactionProcessor,
        stake_processor::StakeTransactionProcessor, token_processor::TokenTransactionProcessor,
        Processor,
    },
};
use velor_api::context::Context;
use velor_config::config::{IndexerConfig, NodeConfig};
use velor_logger::{error, info};
use velor_mempool::MempoolClientSender;
use velor_storage_interface::DbReader;
use velor_types::chain_id::ChainId;
use std::{collections::VecDeque, sync::Arc};
use tokio::runtime::Runtime;

pub struct MovingAverage {
    window_millis: u64,
    // (timestamp_millis, value)
    values: VecDeque<(u64, u64)>,
    sum: u64,
}

impl MovingAverage {
    pub fn new(window_millis: u64) -> Self {
        Self {
            window_millis,
            values: VecDeque::new(),
            sum: 0,
        }
    }

    pub fn tick_now(&mut self, value: u64) {
        let now = chrono::Utc::now().timestamp_millis() as u64;
        self.tick(now, value);
    }

    pub fn tick(&mut self, timestamp_millis: u64, value: u64) -> f64 {
        self.values.push_back((timestamp_millis, value));
        self.sum += value;
        loop {
            match self.values.front() {
                None => break,
                Some((ts, val)) => {
                    if timestamp_millis - ts > self.window_millis {
                        self.sum -= val;
                        self.values.pop_front();
                    } else {
                        break;
                    }
                },
            }
        }
        self.avg()
    }

    pub fn avg(&self) -> f64 {
        if self.values.len() < 2 {
            0.0
        } else {
            let elapsed = self.values.back().unwrap().0 - self.values.front().unwrap().0;
            self.sum as f64 / elapsed as f64
        }
    }
}

/// Creates a runtime which creates a thread pool which reads from storage and writes to postgres
/// Returns corresponding Tokio runtime
pub fn bootstrap(
    config: &NodeConfig,
    chain_id: ChainId,
    db: Arc<dyn DbReader>,
    mp_sender: MempoolClientSender,
) -> Option<anyhow::Result<Runtime>> {
    if !config.indexer.enabled {
        return None;
    }

    let runtime = velor_runtimes::spawn_named_runtime("indexer".into(), None);

    let indexer_config = config.indexer.clone();
    let node_config = config.clone();

    runtime.spawn(async move {
        let context = Arc::new(Context::new(
            chain_id,
            db,
            mp_sender,
            node_config,
            None, /* table info reader */
        ));
        run_forever(indexer_config, context).await;
    });

    Some(Ok(runtime))
}

pub async fn run_forever(config: IndexerConfig, context: Arc<Context>) {
    // All of these options should be filled already with defaults
    let processor_name = config.processor.clone().unwrap();
    let check_chain_id = config.check_chain_id.unwrap();
    let skip_migrations = config.skip_migrations.unwrap();
    let fetch_tasks = config.fetch_tasks.unwrap();
    let processor_tasks = config.processor_tasks.unwrap();
    let emit_every = config.emit_every.unwrap();
    let batch_size = config.batch_size.unwrap();
    let lookback_versions = config.gap_lookback_versions.unwrap() as i64;

    info!(processor_name = processor_name, "Starting indexer...");

    let db_uri = &config.postgres_uri.unwrap();
    info!(
        processor_name = processor_name,
        "Creating connection pool..."
    );
    let conn_pool = new_db_pool(db_uri).expect("Failed to create connection pool");
    info!(
        processor_name = processor_name,
        "Created the connection pool... "
    );

    info!(processor_name = processor_name, "Instantiating tailer... ");

    let processor_enum = Processor::from_string(&processor_name);
    let processor: Arc<dyn TransactionProcessor> = match processor_enum {
        Processor::DefaultProcessor => {
            Arc::new(DefaultTransactionProcessor::new(conn_pool.clone()))
        },
        Processor::TokenProcessor => Arc::new(TokenTransactionProcessor::new(
            conn_pool.clone(),
            config.ans_contract_address,
            config.nft_points_contract,
        )),
        Processor::CoinProcessor => Arc::new(CoinTransactionProcessor::new(conn_pool.clone())),
        Processor::StakeProcessor => Arc::new(StakeTransactionProcessor::new(conn_pool.clone())),
    };

    let options =
        TransactionFetcherOptions::new(None, None, Some(batch_size), None, fetch_tasks as usize);

    let tailer = Tailer::new(context, conn_pool.clone(), processor, options)
        .expect("Failed to instantiate tailer");

    if !skip_migrations {
        info!(processor_name = processor_name, "Running migrations...");
        tailer.run_migrations();
    }

    info!(
        processor_name = processor_name,
        lookback_versions = lookback_versions,
        "Fetching starting version from db..."
    );
    // For now this is not being used but we'd want to track it anyway
    let starting_version_from_db_short = tailer
        .get_start_version(&processor_name)
        .unwrap_or_else(|e| panic!("Failed to get starting version: {:?}", e))
        .unwrap_or_else(|| {
            info!(
                processor_name = processor_name,
                "No starting version from db so starting from version 0"
            );
            0
        }) as u64;
    let start_version = match config.starting_version {
        None => starting_version_from_db_short,
        Some(version) => version,
    };

    info!(
        processor_name = processor_name,
        final_start_version = start_version,
        start_version_from_config = config.starting_version,
        starting_version_from_db = starting_version_from_db_short,
        "Setting starting version..."
    );
    tailer.set_fetcher_version(start_version).await;

    info!(processor_name = processor_name, "Starting fetcher...");
    tailer.transaction_fetcher.lock().await.start().await;

    info!(
        processor_name = processor_name,
        start_version = start_version,
        "Indexing loop started!"
    );

    let mut versions_processed: u64 = 0;
    let mut base: u64 = 0;

    // Check once here to avoid a boolean check every iteration
    if check_chain_id {
        tailer
            .check_or_update_chain_id()
            .await
            .expect("Failed to get chain ID");
    }

    let mut ma = MovingAverage::new(10_000);

    loop {
        let mut tasks = vec![];
        for _ in 0..processor_tasks {
            let other_tailer = tailer.clone();
            let task = tokio::spawn(async move { other_tailer.process_next_batch().await });
            tasks.push(task);
        }
        let batches = match futures::future::try_join_all(tasks).await {
            Ok(res) => res,
            Err(err) => panic!("Error processing transaction batches: {:?}", err),
        };

        let mut batch_start_version = u64::MAX;
        let mut batch_end_version = 0;
        let mut num_res = 0;

        for (num_txn, res) in batches {
            let processed_result: ProcessingResult = match res {
                // When the batch is empty b/c we're caught up, continue to next batch
                None => continue,
                Some(Ok(res)) => res,
                Some(Err(tpe)) => {
                    let (err, start_version, end_version, _) = tpe.inner();
                    error!(
                        processor_name = processor_name,
                        start_version = start_version,
                        end_version = end_version,
                        error =? err,
                        "Error processing batch!"
                    );
                    panic!(
                        "Error in '{}' while processing batch: {:?}",
                        processor_name, err
                    );
                },
            };
            batch_start_version =
                std::cmp::min(batch_start_version, processed_result.start_version);
            batch_end_version = std::cmp::max(batch_end_version, processed_result.end_version);
            num_res += num_txn;
        }

        tailer
            .update_last_processed_version(&processor_name, batch_end_version)
            .unwrap_or_else(|e| {
                error!(
                    processor_name = processor_name,
                    end_version = batch_end_version,
                    error = format!("{:?}", e),
                    "Failed to update last processed version!"
                );
                panic!("Failed to update last processed version: {:?}", e);
            });

        ma.tick_now(num_res);

        versions_processed += num_res;
        if emit_every != 0 {
            let new_base: u64 = versions_processed / emit_every;
            if base != new_base {
                base = new_base;
                info!(
                    processor_name = processor_name,
                    batch_start_version = batch_start_version,
                    batch_end_version = batch_end_version,
                    versions_processed = versions_processed,
                    tps = (ma.avg() * 1000.0) as u64,
                    "Processed batch version"
                );
            }
        }
    }
}
