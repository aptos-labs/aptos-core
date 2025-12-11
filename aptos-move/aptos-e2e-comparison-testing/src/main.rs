// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::Result;
use aptos_comparison_testing::{
    prepare_aptos_packages, DataCollection, Execution, ExecutionMode, OnlineExecutor,
    APTOS_COMMONS, DISABLE_SPEC_CHECK,
};
use aptos_rest_client::Client;
use clap::{Parser, Subcommand};
use itertools::Itertools;
use move_core_types::account_address::AccountAddress;
use std::path::PathBuf;
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
        /// Branch of framework
        #[clap(long)]
        branch: Option<String>,
        /// force override the framework,
        #[clap(long, default_value_t = false)]
        force_override_framework: bool,
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
        /// Branch of framework
        #[clap(long)]
        branch: Option<String>,
        /// Base experiments
        #[clap(long)]
        base_experiments: Option<String>,
        /// Compared experiments
        #[clap(long)]
        compared_experiments: Option<String>,
        /// disable aa and fa related features for handling old txn data which is incompatible with new VM
        #[clap(long, default_value_t = false)]
        disable_aa_fa: bool,
        /// force override the framework,
        #[clap(long, default_value_t = false)]
        force_override_framework: bool,
    },
    /// Execution of txns
    Execute {
        /// Path to the data
        input_path: Option<PathBuf>,
        /// Whether to execute against V1, V2 alone or both compilers for comparison
        #[clap(long)]
        execution_mode: Option<ExecutionMode>,
        /// Branch of framework
        #[clap(long)]
        branch: Option<String>,
        /// Base experiments
        #[clap(long)]
        base_experiments: Option<String>,
        /// Compared experiments
        #[clap(long)]
        compared_experiments: Option<String>,
        /// disable aa and fa related features for handling old txn data which is incompatible with new VM
        #[clap(long, default_value_t = false)]
        disable_aa_fa: bool,
        /// force override the framework,
        #[clap(long, default_value_t = false)]
        force_override_framework: bool,
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
    let parse_experiments = |opt: Option<String>| {
        let mut experiments = opt
            .map(|s| {
                s.split(',')
                    .map(|part| part.trim().to_string())
                    .collect_vec()
            })
            .unwrap_or_default();
        // disable spec check by default
        experiments.push(DISABLE_SPEC_CHECK.to_string());
        experiments
    };
    let args = Argument::parse();
    match args.cmd {
        Cmd::Dump {
            endpoint,
            output_path,
            skip_failed_txns,
            skip_publish_txns,
            skip_source_code_check: skip_source_code,
            dump_write_set,
            target_account,
            branch,
            force_override_framework,
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
                prepare_aptos_packages(
                    output.join(APTOS_COMMONS),
                    branch,
                    force_override_framework,
                )
                .await;
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
            let experiments = parse_experiments(None);
            data_collector
                .dump_data(args.begin_version, args.limit, experiments)
                .await?;
        },
        Cmd::Online {
            endpoint,
            output_path,
            skip_failed_txns,
            skip_publish_txns,
            execution_mode,
            base_experiments,
            branch,
            compared_experiments,
            disable_aa_fa,
            force_override_framework,
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
            prepare_aptos_packages(output.join(APTOS_COMMONS), branch, force_override_framework)
                .await;
            let online = OnlineExecutor::new_with_rest_client(
                Client::new(Url::parse(&endpoint)?),
                output.clone(),
                batch_size,
                skip_failed_txns,
                skip_publish_txns,
                execution_mode.unwrap_or_default(),
                endpoint,
                disable_aa_fa,
            )?;
            let base_experiments = parse_experiments(base_experiments);
            let compared_experiments = parse_experiments(compared_experiments);
            online
                .execute(
                    args.begin_version,
                    args.limit,
                    base_experiments,
                    compared_experiments,
                )
                .await?;
        },
        Cmd::Execute {
            input_path,
            execution_mode,
            branch,
            base_experiments,
            compared_experiments,
            disable_aa_fa,
            force_override_framework,
        } => {
            let input = if let Some(path) = input_path {
                path
            } else {
                PathBuf::from(".")
            };
            let exec_mode = execution_mode.unwrap_or_default();
            prepare_aptos_packages(input.join(APTOS_COMMONS), branch, force_override_framework)
                .await;
            let base_experiments = parse_experiments(base_experiments);
            let compared_experiments = parse_experiments(compared_experiments);
            let executor = Execution::new(input, exec_mode, disable_aa_fa);
            executor
                .execute_txns(
                    args.begin_version,
                    args.limit,
                    base_experiments,
                    compared_experiments,
                )
                .await?;
        },
    };
    Ok(())
}
