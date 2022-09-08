// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

mod big_query;
mod check;
mod get_pfns;
mod get_vfns;
mod helpers;

use anyhow::{Context, Result};
use big_query::BigQueryArgs;
use check::NodeHealthCheckerArgs;
use clap::{Parser, Subcommand};
use log::info;

use crate::big_query::write_to_big_query;

#[derive(Debug, Parser)]
#[clap(author, version, about)]
pub struct Args {
    #[clap(subcommand)]
    pub command: Command,

    #[clap(flatten)]
    pub node_health_checker_args: NodeHealthCheckerArgs,

    #[clap(flatten)]
    big_query_args: BigQueryArgs,

    /// Only run against these account addresses.
    #[clap(long)]
    pub account_address_allowlist: Vec<String>,
}

// We only use this subcommand to configure how to get the node information,
// but we name them as if they're running everything for the sake of the
// resulting CLI subcommand names.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Check Validator FullNodes, reading the VFNs from chain.
    CheckValidatorFullNodes(get_vfns::GetValidatorFullNodes),

    /// Check Public FullNodes, reading the data from a file.
    CheckPublicFullNodes(get_pfns::GetPublicFullNodes),
}

impl Command {
    pub fn data_source_description(&self) -> String {
        match self {
            Command::CheckValidatorFullNodes(vfn_args) => {
                format!("the on-chain validator set at {}", vfn_args.node_address)
            }
            Command::CheckPublicFullNodes(pfn_args) => {
                format!("the data in {}", pfn_args.input_file.to_string_lossy())
            }
        }
    }

    pub fn get_node_type(&self) -> &str {
        match self {
            Command::CheckValidatorFullNodes(_) => "validator_fullnode",
            Command::CheckPublicFullNodes(_) => "public_fullnode",
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();
    info!("Running with args: {:#?}", args);

    let (node_infos, failed_node_results) = match &args.command {
        Command::CheckValidatorFullNodes(get_vfns) => get_vfns
            .get_node_infos(&args.account_address_allowlist)
            .await
            .context("Failed to get VFNs")?,
        Command::CheckPublicFullNodes(get_pfns) => get_pfns
            .get_node_infos(&args.account_address_allowlist)
            .await
            .context("Failed to get PFNs")?,
    };

    let num_valid_node_addresses: usize = node_infos.values().map(|values| values.len()).sum();
    let num_invalid_node_addresses: usize = failed_node_results
        .values()
        .map(|values| values.len())
        .sum();
    let num_account_addresses = node_infos.len() + failed_node_results.len();
    let num_node_addresses = num_valid_node_addresses + num_invalid_node_addresses;

    info!(
        "Discovered {} nodes (under {} account addresses) from {}",
        num_node_addresses,
        num_account_addresses,
        args.command.data_source_description()
    );
    info!(
        "Of those, {} node addresses were valid and {} were invalid",
        num_valid_node_addresses, num_invalid_node_addresses
    );
    info!("Checking health of those valid nodes with NHC now");

    let mut nhc_responses = args.node_health_checker_args.check_nodes(node_infos).await;

    // Merge failed results into the successful results.
    for (account_address, node_results) in failed_node_results {
        for node_result in node_results {
            nhc_responses
                .entry(account_address)
                .or_insert_with(Vec::new)
                .push(node_result);
        }
    }

    info!("Printing JSON representation of output to stdout");

    println!(
        "{}",
        serde_json::to_string(&nhc_responses).context("Failed to encode data as JSON")?
    );

    if !args.big_query_args.big_query_dry_run {
        info!(
            "Got responses for {} account addresses from NHC. We will merge \
            them with the invalid results from earlier and output them to BigQuery",
            nhc_responses.len(),
        );
        write_to_big_query(
            &args.big_query_args,
            nhc_responses,
            args.command.get_node_type(),
        )
        .await
        .context("Failed to write results to BigQuery")?;
    }

    info!("Done!");
    Ok(())
}
