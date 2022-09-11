// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    database::new_db_pool,
    indexer::{
        fetcher::TransactionFetcherOptions, tailer::Tailer,
        transaction_processor::TransactionProcessor,
    },
    processors::{
        default_processor::DefaultTransactionProcessor, token_processor::TokenTransactionProcessor,
        Processor,
    },
};

use aptos_api::context::Context;
use aptos_config::config::{IndexerConfig, NodeConfig};
use aptos_logger::{error, info};
use aptos_mempool::MempoolClientSender;
use aptos_types::chain_id::ChainId;
use std::collections::VecDeque;
use std::sync::Arc;
use storage_interface::DbReader;
use tokio::runtime::{Builder, Runtime};

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
        let now = chrono::Utc::now().naive_utc().timestamp_millis() as u64;
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
                }
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

    let runtime = Builder::new_multi_thread()
        .thread_name("indexer")
        .disable_lifo_slot()
        .enable_all()
        .build()
        .expect("[indexer] failed to create runtime");

    let indexer_config = config.indexer.clone();
    let node_config = config.clone();

    runtime.spawn(async move {
        let context = Arc::new(Context::new(chain_id, db, mp_sender, node_config));
        run_forever(indexer_config, context).await;
    });

    Some(Ok(runtime))
}

pub async fn run_forever(config: IndexerConfig, context: Arc<Context>) {
    let processor_name = config.processor.clone();

    info!(processor_name = processor_name, "Starting indexer...");

    let conn_pool = new_db_pool(&config.postgres_uri).expect("Failed to create connection pool");
    info!(
        processor_name = processor_name,
        "Created the connection pool... "
    );

    info!(processor_name = processor_name, "Instantiating tailer... ");

    let processor_enum = Processor::from_string(&processor_name);
    let processor: Arc<dyn TransactionProcessor> = match processor_enum {
        Processor::DefaultProcessor => {
            Arc::new(DefaultTransactionProcessor::new(conn_pool.clone()))
        }
        Processor::TokenProcessor => Arc::new(TokenTransactionProcessor::new(
            conn_pool.clone(),
            config.index_token_uri_data,
        )),
    };

    let options = TransactionFetcherOptions::new(
        None,
        None,
        Some(config.batch_size),
        None,
        config.fetch_tasks as usize,
    );

    let tailer = Tailer::new(context, conn_pool.clone(), processor, options)
        .expect("Failed to instantiate tailer");

    if !config.skip_migrations {
        info!(processor_name = processor_name, "Running migrations...");
        tailer.run_migrations();
    }

    let start_version = match config.starting_version {
        None => tailer
            .get_start_version(&processor_name)
            .unwrap_or_else(|| {
                info!(
                    processor_name = processor_name,
                    "Could not fetch version from db so starting from version 0"
                );
                0
            }),
        Some(version) => version,
    };

    info!(
        processor_name = processor_name,
        start_version = start_version,
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
    let mut version_to_check_chain_id: u64 = 0;

    // Check once here to avoid a boolean check every iteration
    if config.check_chain_id {
        tailer
            .check_or_update_chain_id()
            .await
            .expect("Failed to get chain ID");
        version_to_check_chain_id = versions_processed + 100_000;
    }

    let (tx, mut receiver) = tokio::sync::mpsc::channel(100);
    let mut tasks = vec![];
    for _ in 0..config.processor_tasks {
        let other_tx = tx.clone();
        let other_tailer = tailer.clone();
        let task = tokio::task::spawn(async move {
            loop {
                let (num_res, res) = other_tailer.process_next_batch().await;
                other_tx.send((num_res, res)).await.unwrap();
            }
        });
        tasks.push(task);
    }

    let emit_every = config.emit_every as u64;

    let mut ma = MovingAverage::new(10_000);

    loop {
        if config.check_chain_id && version_to_check_chain_id < versions_processed {
            tailer
                .check_or_update_chain_id()
                .await
                .expect("Failed to get chain ID");
            version_to_check_chain_id = versions_processed + 100_000;
        }

        let (num_res, result) = receiver
            .recv()
            .await
            .expect("Failed to receive batch results: got None!");

        let processing_result = match result {
            Ok(res) => res,
            Err(tpe) => {
                let (err, start_version, end_version, _) = tpe.inner();
                error!(
                    processor_name = processor_name,
                    start_version = start_version,
                    end_version = end_version,
                    error = format!("{:?}", err),
                    "Error processing batch!"
                );
                panic!(
                    "Error in '{}' while processing batch: {:?}",
                    processor_name, err
                );
            }
        };

        ma.tick_now(num_res);

        versions_processed += num_res;
        if emit_every != 0 {
            let new_base: u64 = versions_processed / emit_every;
            if base != new_base {
                base = new_base;
                info!(
                    processor_name = processor_name,
                    batch_start_version = processing_result.start_version,
                    batch_end_version = processing_result.end_version,
                    versions_processed = versions_processed,
                    tps = (ma.avg() * 1000.0) as u64,
                    "Processed batch version"
                );
            }
        }
    }
}
