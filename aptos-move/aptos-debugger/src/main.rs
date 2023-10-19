// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_db::db_debugger::examine::consensus_db::Cmd;
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

    #[clap(long, default_value_t = 1)]
    concurrency_level: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    aptos_logger::Logger::new().init();
    /*
    let args = Argument::parse();

    println!(
        "{:#?}",
        debugger
            .execute_past_transactions(args.begin_version, args.limit)
            .await?
    );*/

    AptosVM::set_concurrency_level_once(1);

    let cmd = Cmd {
        db_dir: "/Users/grao/work/data/sev".into(),
    };
    let txns = cmd.run();
    let debugger = AptosDebugger::rest_client(Client::new(Url::parse(
        &"https://fullnode.mainnet.aptoslabs.com/v1/",
    )?))?;
    let result = debugger.execute_transactions_at_version(301617677, txns);
    println!("{result:?}");

    Ok(())
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Argument::command().debug_assert()
}
