// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, Result};
use aptos_debugger::AptosDebugger;
use aptos_rest_client::Client;
use aptos_types::transaction::Transaction;
use aptos_vm::AptosVM;
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
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
    let args = Argument::parse();
    AptosVM::set_concurrency_level_once(args.concurrency_level);

    let debugger = match args.target {
        Target::Rest { endpoint } => {
            AptosDebugger::rest_client(Client::new(Url::parse(&endpoint)?))?
        },
        Target::DB { path } => AptosDebugger::db(path)?,
    };

    /*
    println!(
        "{:#?}",
        debugger
            .execute_past_transactions(args.begin_version, args.limit)
            .await?
    );*/

    let version = args.begin_version;
    let (txn, _) = debugger
        .get_committed_transaction_at_version(version)
        .await?;

    let txn = match txn {
        Transaction::UserTransaction(txn) => txn,
        _ => bail!("not a user transaction"),
    };

    let (_status, output, gas_log) =
        debugger.execute_transaction_at_version_with_gas_profiler(version, txn)?;

    let txn_output =
        output.try_into_transaction_output(&debugger.state_view_at_version(version))?;

    // Show results to the user
    println!("{:#?}", txn_output);

    let report_path = Path::new("gas-profiling").join(format!("txn-{}", version));
    gas_log.generate_html_report(
        &report_path,
        format!("Gas Report - Transaction {}", version),
    )?;

    println!("Gas profiling report saved to {}.", report_path.display());

    Ok(())
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Argument::command().debug_assert()
}
