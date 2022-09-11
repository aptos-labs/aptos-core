// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Indexer is used to index blockchain data into Postgres
#![forbid(unsafe_code)]

use aptos_logger::{error, info};
use clap::Parser;
use std::{env, sync::Arc};

use aptos_indexer::indexer::fetcher::TransactionFetcherOptions;
use aptos_indexer::{
    counters::start_inspection_service,
    database::new_db_pool,
    indexer::{tailer::Tailer, transaction_processor::TransactionProcessor},
    processors::{
        default_processor::{DefaultTransactionProcessor, NAME as DEFAULT_PROCESSOR_NAME},
        token_processor::{TokenTransactionProcessor, NAME as TOKEN_PROCESSOR_NAME},
    },
};

#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
struct IndexerArgs {
    /// Postgres database uri, ex: "postgresql://user:pass@localhost/postgres"
    #[clap(long, env = "INDEXER_DATABASE_URL")]
    pg_uri: String,

    /// URL of an Aptos node, ex: "https://fullnode.devnet.aptoslabs.com"
    #[clap(long, env = "FULLNODE_URL")]
    node_url: String,

    #[clap(long, env = "INSPECTION_URL", default_value = "localhost")]
    inspection_url: String,

    #[clap(long, env = "INSPECTION_PORT", default_value = "9105")]
    inspection_port: u16,

    /// The specific processor that it will run, ex: "token_processor"
    #[clap(long, env = "PROCESSOR_NAME")]
    processor: String,

    /// If set, don't run any migrations
    #[clap(long)]
    skip_migrations: bool,

    /// turn on the token URI fetcher
    #[clap(long)]
    index_token_uri_data: bool,

    /// If set, will ignore database contents and start processing from the specified version.
    /// This will not delete any database contents, just transactions as it reprocesses them.
    #[clap(long)]
    start_from_version: Option<u64>,

    /// If set, will make sure that we're still indexing the right chain every 100K transactions
    #[clap(long)]
    check_chain_id: bool,

    /// How many versions to fetch and process from a node in parallel
    #[clap(long)]
    batch_size: Option<u16>,

    /// How many tasks to run for fetching the transactions
    #[clap(long)]
    fetch_tasks: Option<u8>,

    /// How many tasks to run for processing the transactions
    #[clap(long, default_value = "5")]
    processor_tasks: u8,

    /// How many versions to process before logging a "processed X versions" message.
    /// This will only be checked every `--batch-size` number of versions.
    /// Set to 0 to disable.
    #[clap(long, default_value_t = 1000)]
    emit_every: usize,
}

enum Processor {
    DefaultProcessor,
    TokenProcessor,
}

impl Processor {
    fn from_string(input_str: &String) -> Self {
        match input_str.as_str() {
            DEFAULT_PROCESSOR_NAME => Self::DefaultProcessor,
            TOKEN_PROCESSOR_NAME => Self::TokenProcessor,
            _ => panic!("Processor unsupported {}", input_str),
        }
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    aptos_logger::Logger::new().init();
    let args: IndexerArgs = IndexerArgs::parse();
    let processor_name = &args.processor;

    info!(processor_name = processor_name, "Starting indexer...");

    info!(
        processor_name = processor_name,
        "Created the inspection service... "
    );

    start_inspection_service(args.inspection_url.as_str(), args.inspection_port);

    info!(
        processor_name = processor_name,
        "Created the connection pool... "
    );
    let conn_pool = new_db_pool(&args.pg_uri).expect("Failed to create connection pool");

    info!(processor_name = processor_name, "Instantiating tailer... ");

    let processor: Arc<dyn TransactionProcessor> = match Processor::from_string(&args.processor) {
        Processor::DefaultProcessor => {
            Arc::new(DefaultTransactionProcessor::new(conn_pool.clone()))
        }
        Processor::TokenProcessor => Arc::new(TokenTransactionProcessor::new(
            conn_pool.clone(),
            args.index_token_uri_data,
        )),
    };

    let options = TransactionFetcherOptions::new(
        None,
        None,
        args.batch_size,
        None,
        args.fetch_tasks.map(|x| x as usize),
    );

    let tailer = Tailer::new(&args.node_url, conn_pool.clone(), processor, options)
        .expect("Failed to instantiate tailer");

    if !args.skip_migrations {
        info!(processor_name = processor_name, "Running migrations...");
        tailer.run_migrations();
    }

    let start_version = match args.start_from_version {
        None => tailer.get_start_version(processor_name).unwrap_or_else(|| {
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

    let start = chrono::Utc::now().naive_utc();

    info!(
        processor_name = processor_name,
        start_version = start_version,
        "Indexing loop started!"
    );

    let mut versions_processed: usize = 0;
    let mut total_processed: usize = 0;
    let mut base: usize = 0;
    let mut version_to_check_chain_id: usize = 0;

    // Check once here to avoid a boolean check every iteration
    if args.check_chain_id {
        tailer
            .check_or_update_chain_id()
            .await
            .expect("Failed to get chain ID");
        version_to_check_chain_id = versions_processed + 100_000;
    }

    let (tx, mut receiver) = tokio::sync::mpsc::channel(100);
    let mut tasks = vec![];
    for _ in 0..args.processor_tasks {
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

    loop {
        if args.check_chain_id && version_to_check_chain_id < versions_processed {
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

        total_processed += num_res as usize;
        versions_processed += num_res as usize;
        if args.emit_every != 0 {
            let new_base: usize = versions_processed / args.emit_every;
            if base != new_base {
                base = new_base;
                let num_millis =
                    (chrono::Utc::now().naive_utc() - start).num_milliseconds() as f64 / 1000.0;
                let tps = (total_processed as f64 / num_millis) as u64;
                info!(
                    processor_name = processor_name,
                    batch_start_version = processing_result.start_version,
                    batch_end_version = processing_result.end_version,
                    versions_processed = versions_processed,
                    tps = tps,
                    "Processed batch version"
                );
            }
        }
    }
}
