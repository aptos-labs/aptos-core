// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Indexer is used to index blockchain data into Postgres
#![forbid(unsafe_code)]

use aptos_logger::{debug, info};
use aptos_sf_indexer::indexer::substream_processor::{
    get_start_block, run_migrations, SubstreamProcessor,
};
use aptos_sf_indexer::proto;

use anyhow::{format_err, Context, Error};
use aptos_sf_indexer::{
    counters::start_inspection_service,
    database::new_db_pool,
    substream_processors::{
        block_output_processor::BlockOutputSubstreamProcessor,
        tokens_processor::TokensSubstreamProcessor,
    },
    substreams::SubstreamsEndpoint,
    substreams_stream::{BlockResponse, SubstreamsStream},
};
use clap::Parser;
use futures::StreamExt;
use prost::Message;
use std::{env, sync::Arc};
use tokio::sync::Mutex;

#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
struct IndexerArgs {
    // URL of the firehose gRPC endpoint
    #[clap(long)]
    endpoint_url: String,

    // Relative location of the substream wasm file (.spkg)
    #[clap(long)]
    package_file: String,

    // Substream module name
    #[clap(long)]
    module_name: String,

    /// If set, don't run any migrations
    #[clap(long)]
    skip_migrations: bool,

    /// How many blocks to process before logging a "processed X blocks" message.
    /// Set to 0 to disable.
    #[clap(long, default_value_t = 10)]
    emit_every: usize,
}

enum Processor {
    BlockToBlockOutput,
    BlockOutputToToken,
}

impl Processor {
    fn from_string(input_str: &String) -> Self {
        match input_str.as_str() {
            "block_to_block_output" => Self::BlockToBlockOutput,
            "block_output_to_token" => Self::BlockOutputToToken,
            _ => panic!("Module unsupported {}", input_str),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    aptos_logger::Logger::new().init();
    let args: IndexerArgs = IndexerArgs::parse();
    info!("Starting indexer");

    let endpoint_url = &args.endpoint_url;
    let package_file = &args.package_file;
    let substream_module_name = &args.module_name;

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let inspection_url = env::var("INSPECTION_URL").unwrap_or_else(|_| "localhost".to_string());
    let inspection_port = env::var("INSPECTION_PORT")
        .map(|v| v.parse::<u16>().unwrap_or(9105))
        .unwrap_or(9105);
    start_inspection_service(inspection_url.as_str(), inspection_port);
    let conn_pool = new_db_pool(&database_url).unwrap();
    info!("Created the connection pool");

    if !args.skip_migrations {
        run_migrations(&conn_pool);
    }

    let token_env = env::var("SUBSTREAMS_API_TOKEN").unwrap_or_else(|_| "".to_string());
    let mut token: Option<String> = None;
    if !token_env.is_empty() {
        token = Some(token_env);
    }
    let content =
        std::fs::read(package_file).context(format_err!("read package {}", package_file))?;
    let package = proto::Package::decode(content.as_ref()).context("decode command")?;

    let endpoint = Arc::new(SubstreamsEndpoint::new(&endpoint_url, token).await?);

    info!(
        substream_module_name = substream_module_name,
        "Created substream endpoint"
    );
    let start_block = get_start_block(&conn_pool, substream_module_name).unwrap_or_else(|| {
        info!(
            substream_module_name = substream_module_name,
            "Could not fetch max block so starting from block 0"
        );
        0
    });
    info!(
        substream_module_name = substream_module_name,
        start_block = start_block,
        "Starting stream"
    );

    let mut stream = SubstreamsStream::new(
        endpoint.clone(),
        None, // We're using block instead of cursor currently
        package.modules.clone(),
        substream_module_name.to_string(),
        start_block,
        i64::MAX,
    );

    let mut block_height = start_block as u64;
    let processor: Arc<Mutex<dyn SubstreamProcessor>> =
        match Processor::from_string(substream_module_name) {
            Processor::BlockToBlockOutput => Arc::new(Mutex::new(
                BlockOutputSubstreamProcessor::new(conn_pool.clone()),
            )),
            Processor::BlockOutputToToken => {
                Arc::new(Mutex::new(TokensSubstreamProcessor::new(conn_pool.clone())))
            }
        };
    let start = chrono::Utc::now().naive_utc();
    let mut base: usize = 0;
    loop {
        let data = match stream.next().await {
            None => {
                info!(
                    substream_module_name = substream_module_name,
                    "Stream fully consumed"
                );
                break;
            }
            Some(event) => {
                let block_data;
                if let Ok(BlockResponse::New(data)) = event {
                    debug!(
                        substream_module_name = substream_module_name,
                        block_height = block_height,
                        cursor = data.cursor,
                        "Consuming module output",
                    );
                    block_data = data;
                } else {
                    panic!("Stream response failed");
                }
                block_data
            }
        };
        match processor
            .lock()
            .await
            .process_substream_with_status(data, block_height)
            .await
        {
            Ok(_) => {
                if args.emit_every != 0 {
                    let processed_this_session = block_height as usize - start_block as usize + 1;
                    let new_base: usize = block_height as usize / args.emit_every;
                    if base != new_base {
                        base = new_base;
                        let num_millis = (chrono::Utc::now().naive_utc() - start).num_milliseconds()
                            as f64
                            / 1000.0;
                        let bps = (processed_this_session as f64 / num_millis) as u64;
                        info!(
                            substream_module_name = substream_module_name,
                            block_height = block_height,
                            blocks_per_second = bps,
                            "Finished processing block",
                        );
                    }
                }
                block_height += 1
            }
            Err(error) => {
                panic!(
                    "[{}] Error processing block {}, error: {:?}",
                    substream_module_name, block_height, &error
                );
            }
        };
    }

    Ok(())
}
