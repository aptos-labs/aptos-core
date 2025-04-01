// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use aptos_experimental_bulk_txn_submit::{
    coordinator::{
        create_sample_addresses, execute_return_worker_funds, execute_submit,
        CreateSampleAddresses, SanitizeAddresses, SubmitArgs,
    },
    workloads::{
        create_account_addresses_work, CreateAndTransferAptSignedTransactionBuilder,
        TransferAptSignedTransactionBuilder,
    },
};
use aptos_logger::{Level, Logger};
use aptos_sdk::move_types::account_address::AccountAddress;
use aptos_transaction_emitter_lib::Cluster;
use clap::{Parser, Subcommand};
use rand::{seq::SliceRandom, thread_rng};
use std::collections::HashSet;

#[derive(Parser, Debug)]
struct Args {
    #[clap(subcommand)]
    command: DemoCommand,
}

#[derive(Subcommand, Debug)]
enum DemoCommand {
    /// Submits set of transactions.
    Submit(Submit),
    /// Create a file with sample addresses, for testing.
    CreateSampleAddresses(CreateSampleAddresses),
    /// Sanitizes the addresses file
    /// Removes all duplicates, and shuffles the result.
    SanitizeAddresses(SanitizeAddresses),
}

#[derive(Parser, Debug)]
pub struct Submit {
    #[clap(flatten)]
    submit_args: SubmitArgs,
    #[clap(subcommand)]
    work_args: WorkTypeSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum WorkTypeSubcommand {
    /// Executes coin::transfer<AptosCoin> with given file providing list of destinations
    TransferApt(TransferArg),
    /// Executes aptos_account::transfer with given file providing list of destinations
    CreateAndTransferApt(TransferArg),
    /// Returns all leftover funds on the workers to the main source account
    ReturnWorkerFunds,
}

#[derive(Parser, Debug)]
pub struct TransferArg {
    #[clap(long, default_value_t = 1)]
    amount_to_send: u64,

    #[clap(long)]
    destinations_file: String,
}

#[derive(Parser, Debug)]
pub struct DestinationsArg {
    #[clap(long)]
    destinations_file: String,
}

#[tokio::main]
pub async fn main() -> Result<()> {
    Logger::builder().level(Level::Info).build();

    let args = Args::parse();

    match args.command {
        DemoCommand::Submit(args) => create_work_and_execute(args).await,
        DemoCommand::CreateSampleAddresses(args) => create_sample_addresses(args),
        DemoCommand::SanitizeAddresses(args) => sanitize_addresses(args),
    }
}

async fn create_work_and_execute(args: Submit) -> Result<()> {
    let cluster = Cluster::try_from_cluster_args(&args.submit_args.cluster_args)
        .await
        .context("Failed to build cluster")?;
    let coin_source_account = cluster
        .load_coin_source_account(&cluster.random_instance().rest_client())
        .await?;

    match &args.work_args {
        WorkTypeSubcommand::TransferApt(transfer_args) => {
            let work = create_account_addresses_work(&transfer_args.destinations_file, false)?;
            execute_submit(
                work,
                args.submit_args,
                TransferAptSignedTransactionBuilder {
                    amount_to_send: transfer_args.amount_to_send,
                },
                cluster,
                coin_source_account,
                false,
            )
            .await
        },
        WorkTypeSubcommand::CreateAndTransferApt(transfer_args) => {
            let work = create_account_addresses_work(&transfer_args.destinations_file, false)?;
            execute_submit(
                work,
                args.submit_args,
                CreateAndTransferAptSignedTransactionBuilder {
                    amount_to_send: transfer_args.amount_to_send,
                },
                cluster,
                coin_source_account,
                false,
            )
            .await
        },
        WorkTypeSubcommand::ReturnWorkerFunds => {
            execute_return_worker_funds(
                args.submit_args.transaction_factory_args,
                args.submit_args.accounts_args,
                cluster,
                &coin_source_account,
            )
            .await
        },
    }
}

fn sanitize_addresses(args: SanitizeAddresses) -> Result<()> {
    let work = create_account_addresses_work(&args.destinations_file, false)?;
    println!("Sanitizing addresses, {} in input.", work.len());
    let mut unique = work
        .into_iter()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    unique.shuffle(&mut thread_rng());
    println!("Sanitized addresses, {} left in the output", unique.len());
    std::fs::write(
        args.output_file,
        unique
            .iter()
            .map(AccountAddress::to_standard_string)
            .collect::<Vec<_>>()
            .join("\n"),
    )?;
    Ok(())
}
