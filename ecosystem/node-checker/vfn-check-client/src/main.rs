// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

mod big_query;
mod check;
mod helpers;

use anyhow::{Context, Result};
use big_query::BigQueryArgs;
use check::CheckArgs;
use clap::Parser;
use log::info;
use reqwest::Client as ReqwestClient;
use std::time::Duration;

use crate::{
    big_query::write_to_big_query,
    check::{check_vfns, get_validator_info},
};

#[derive(Clone, Debug, clap::ValueEnum)]
pub enum OutputStyle {
    Stdout,
    BigQuery,
}

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(flatten)]
    pub check_args: CheckArgs,

    #[clap(flatten)]
    big_query_args: BigQueryArgs,

    /// How to output the results.
    #[clap(long, value_enum, default_value = "stdout", case_insensitive = true)]
    output_style: OutputStyle,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();
    info!("Running with args: {:#?}", args);

    let nhc_client = ReqwestClient::builder()
        .timeout(Duration::from_secs(args.check_args.nhc_timeout_secs))
        .build()
        .unwrap();

    let validator_infos = get_validator_info(args.check_args.node_address.clone())
        .await
        .context("Failed to get on chain validator info")?;

    let nhc_responses = check_vfns(&nhc_client, &args.check_args, validator_infos).await;

    info!(
        "Got {} responses from NHC, will now output to {:?}",
        nhc_responses.len(),
        args.output_style
    );

    match args.output_style {
        OutputStyle::Stdout => {
            println!(
                "{}",
                serde_json::to_string(&nhc_responses).context("Failed to encode data as JSON")?
            );
        }
        OutputStyle::BigQuery => {
            write_to_big_query(&args.big_query_args, nhc_responses)
                .await
                .context("Failed to write to BigQuery")?;
        }
    }

    info!("Done!");

    Ok(())
}
