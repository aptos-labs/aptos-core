// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_debugger::AptosDebugger;
use aptos_rest_client::Client;
use aptos_vm::AptosVM;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use url::Url;

#[derive(Subcommand)]
pub enum Target {
    /// Use full node's rest api as query endpoint.
    Rest { endpoint: String },
    /// Use a local db instance to serve as query endpoint.
    DB { path: PathBuf },
}
#[derive(Parser)]
pub struct Argument {
    #[clap(subcommand)]
    target: Target,

    #[clap(long)]
    begin_version: u64,

    #[clap(long)]
    limit: u64,

    #[clap(long, default_value = "1")]
    concurrency_level: usize,

    /// Default 0 disables executable caching across the blocks.
    #[clap(long, default_value = "0")]
    executable_cache_size: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    aptos_logger::Logger::new().init();
    let args = Argument::parse();
    AptosVM::set_concurrency_level_once(args.concurrency_level);
    AptosVM::set_executable_cache_size_once(args.executable_cache_size as u64);

    let debugger = match args.target {
        Target::Rest { endpoint } => {
            AptosDebugger::rest_client(Client::new(Url::parse(&endpoint)?))?
        },
        Target::DB { path } => AptosDebugger::db(path)?,
    };

    println!(
        "{:#?}",
        debugger
            .execute_past_transactions(args.begin_version, args.limit)
            .await?
    );

    Ok(())
}
