// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Indexer is used to index blockchain data into Postgres
//!
//! TODO: Examples
//!
#![forbid(unsafe_code)]

use aptos_logger::info;
use clap::Parser;
use std::sync::Arc;

use aptos_indexer::{
    database::new_db_pool, default_processor::DefaultTransactionProcessor, indexer::tailer::Tailer,
    token_processor::TokenTransactionProcessor,
};

#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
struct IndexerArgs {
    /// Postgres database uri, ex: "postgresql://user:pass@localhost/postgres"
    #[clap(long)]
    pg_uri: String,

    /// URL of an Aptos node, ex: "https://fullnode.devnet.aptoslabs.com"
    #[clap(long)]
    node_url: String,

    /// If set, don't run any migrations
    #[clap(long)]
    skip_migrations: bool,

    /// If set, will exit after migrations/repairs instead of starting indexing loop
    #[clap(long)]
    dont_index: bool,

    /// If set, will ignore database contents and start processing from the specified version.
    /// This will not delete any database contents, just transactions as it reprocesses them.
    #[clap(long)]
    start_from_version: Option<u64>,

    /// How many blocks to process before logging a "processed X versions" message.
    /// Set to 0 to disable.
    #[clap(long, default_value_t = 1000)]
    emit_every: usize,

    /// Turn on the indexer to collect token, ownership, collection and metadata and store them
    /// in the postgres DB tables.
    #[clap(long)]
    index_token_data: bool,

    /// turn on the token URI fetcher
    #[clap(long)]
    index_token_uri_data: bool,
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    aptos_logger::Logger::new().init();
    let args: IndexerArgs = IndexerArgs::parse();

    info!("Starting indexer...");

    let conn_pool = new_db_pool(&args.pg_uri).unwrap();
    info!("Created the connection pool... ");

    let mut tailer = Tailer::new(&args.node_url, conn_pool.clone()).unwrap();

    if !args.skip_migrations {
        tailer.run_migrations();
    }

    tailer.check_or_update_chain_id().await.unwrap();
    let pg_transaction_processor = DefaultTransactionProcessor::new(conn_pool.clone());
    tailer.add_processor(Arc::new(pg_transaction_processor));
    if args.index_token_data {
        let token_transaction_processor =
            TokenTransactionProcessor::new(conn_pool.clone(), args.index_token_uri_data);
        tailer.add_processor(Arc::new(token_transaction_processor));
    }

    let starting_block = match args.start_from_version {
        None => tailer.set_fetcher_to_lowest_processor_block().await,
        Some(version) => {
            tailer
                .set_fetcher_to_block_from_input_version(version)
                .await
        }
    };

    if args.dont_index {
        info!("All pre-index tasks complete, exiting!");
        return Ok(());
    }

    info!("Indexing loop started!");
    let mut processed: usize = starting_block as usize;
    let mut base: usize = 0;
    loop {
        if tailer.process_next_block().await.is_ok() {
            processed += 1;
            tailer.increment_block_height().await;
        }
        if args.emit_every != 0 {
            let new_base: usize = processed / args.emit_every;
            if base != new_base {
                base = new_base;
                aptos_logger::info!("Indexer has processed {} blocks", processed);
            }
        }
    }
}
