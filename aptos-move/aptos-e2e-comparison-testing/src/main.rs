// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_comparison_testing::{
    prepare_aptos_packages, DataCollection, Execution, ExecutionMode, OnlineExecutor,
    APTOS_COMMONS, DISABLE_SPEC_CHECK,
};
use aptos_rest_client::Client;
use clap::{Parser, Subcommand};
use move_command_line_common::env::OVERRIDE_EXP_CACHE;
use move_compiler_v2::Experiment;
use move_core_types::account_address::AccountAddress;
use std::{env, path::PathBuf};
use url::Url;

const BATCH_SIZE: u64 = 500;

#[derive(Subcommand)]
pub enum Cmd {
    /// Collect and dump the data
    Dump {
        /// Endpoint url to obtain the txn data, e.g. `https://api.mainnet.aptoslabs.com/v1` for mainnet.
        /// To avoid rate limiting, users need to apply for API key from `https://developers.aptoslabs.com/`
        /// and set the env variable X_API_KEY using the obtained key
        endpoint: String,
        /// Path to the dumped data
        output_path: Option<PathBuf>,
        /// Do not dump failed txns
        #[clap(long, default_value_t = false)]
        skip_failed_txns: bool,
        /// Do not dump publish txns
        #[clap(long, default_value_t = false)]
        skip_publish_txns: bool,
        /// Collect txns regardless whether the source code is available
        #[clap(long, default_value_t = false)]
        skip_source_code_check: bool,
        /// Dump the write set of txns
        #[clap(long, default_value_t = false)]
        dump_write_set: bool,
        /// With this set, only dump transactions that are sent to this account
        #[clap(long)]
        target_account: Option<AccountAddress>,
    },
    /// Collect and execute txns without dumping the state data
    Online {
        /// Endpoint url to obtain the txn data,
        /// e.g. `https://api.mainnet.aptoslabs.com/v1` for mainnet.
        /// To avoid rate limiting, users need to apply for API key from `https://developers.aptoslabs.com/`
        /// and set the env variable X_API_KEY using the obtained key
        endpoint: String,
        /// Path to the dumped data
        output_path: Option<PathBuf>,
        /// Do not dump failed txns
        #[clap(long, default_value_t = false)]
        skip_failed_txns: bool,
        /// Do not dump publish txns
        #[clap(long, default_value_t = false)]
        skip_publish_txns: bool,
        /// Whether to execute against V1, V2 alone or both compilers for comparison
        /// Used when execution_only is true
        #[clap(long)]
        execution_mode: Option<ExecutionMode>,
        /// Packages to be skipped for reference safety check
        #[clap(long)]
        skip_ref_packages: Option<String>,
    },
    /// Execution of txns
    Execute {
        /// Path to the data
        input_path: Option<PathBuf>,
        /// Whether to execute against V1, V2 alone or both compilers for comparison
        #[clap(long)]
        execution_mode: Option<ExecutionMode>,
        /// Packages to be skipped for reference safety check
        #[clap(long)]
        skip_ref_packages: Option<String>,
    },
}

#[derive(Parser)]
pub struct Argument {
    #[clap(subcommand)]
    cmd: Cmd,

    /// Scan/execute from the txn of this version
    #[clap(long)]
    begin_version: u64,

    /// Number of txns to scan/execute
    #[clap(long)]
    limit: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Argument::parse();
    env::set_var(
        OVERRIDE_EXP_CACHE,
        format!(
            "{},{}",
            Experiment::SPEC_CHECK,
            Experiment::REFERENCE_SAFETY
        ),
    );
    env::set_var("MOVE_COMPILER_EXP", DISABLE_SPEC_CHECK);
    match args.cmd {
        Cmd::Dump {
            endpoint,
            output_path,
            skip_failed_txns,
            skip_publish_txns,
            skip_source_code_check: skip_source_code,
            dump_write_set,
            target_account,
        } => {
            let batch_size = BATCH_SIZE;
            let output = if let Some(path) = output_path {
                path
            } else {
                PathBuf::from(".")
            };
            if !output.exists() {
                std::fs::create_dir_all(output.as_path()).unwrap();
            }
            if !skip_source_code {
                prepare_aptos_packages(output.join(APTOS_COMMONS)).await;
            }
            let data_collector = DataCollection::new_with_rest_client(
                Client::new(Url::parse(&endpoint)?),
                output.clone(),
                batch_size,
                skip_failed_txns,
                skip_publish_txns,
                dump_write_set,
                skip_source_code,
                target_account,
            )?;
            data_collector
                .dump_data(args.begin_version, args.limit)
                .await?;
        },
        Cmd::Online {
            endpoint,
            output_path,
            skip_failed_txns,
            skip_publish_txns,
            execution_mode,
            skip_ref_packages,
        } => {
            let batch_size = BATCH_SIZE;
            let output = if let Some(path) = output_path {
                path
            } else {
                PathBuf::from(".")
            };
            if !output.exists() {
                std::fs::create_dir_all(output.as_path()).unwrap();
            }
            prepare_aptos_packages(output.join(APTOS_COMMONS)).await;
            let online = OnlineExecutor::new_with_rest_client(
                Client::new(Url::parse(&endpoint)?),
                output.clone(),
                batch_size,
                skip_failed_txns,
                skip_publish_txns,
                execution_mode.unwrap_or_default(),
                endpoint,
                skip_ref_packages,
            )?;
            online.execute(args.begin_version, args.limit).await?;
        },
        Cmd::Execute {
            input_path,
            execution_mode,
            skip_ref_packages,
        } => {
            let input = if let Some(path) = input_path {
                path
            } else {
                PathBuf::from(".")
            };
            prepare_aptos_packages(input.join(APTOS_COMMONS)).await;
            let executor =
                Execution::new(input, execution_mode.unwrap_or_default(), skip_ref_packages);
            executor
                .execute_txns(args.begin_version, args.limit)
                .await?;
        },
    };
    Ok(())
}
