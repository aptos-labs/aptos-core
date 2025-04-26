// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Context, Result};
use aptos_experimental_bulk_txn_submit::{
    coordinator::{
        create_sample_addresses, execute_return_worker_funds, execute_submit,
        CreateSampleAddresses, SanitizeAddresses, SubmitArgs,
    },
    workloads::{
        create_account_addresses_work, create_job_params_work,
        CreateAndTransferAptSignedTransactionBuilder, MigrateCoinStoreSignedTransactionBuilder,
        MigrationJobParams, TransferAptSignedTransactionBuilder,
    },
};
use aptos_logger::{Level, Logger};
use aptos_sdk::{
    move_types::{account_address::AccountAddress, language_storage::TypeTag},
    types::APTOS_COIN_TYPE_STR,
};
use aptos_transaction_emitter_lib::Cluster;
use clap::{Parser, Subcommand};
use rand::{seq::SliceRandom, thread_rng};
use std::{collections::HashSet, str::FromStr};

lazy_static::lazy_static! {
    pub static ref APT_COIN_TYPE: TypeTag = TypeTag::from_str(APTOS_COIN_TYPE_STR).unwrap();
}

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
    /// Migrate CoinStore to PrimaryFungibleStore
    MigrateCoinStore(DestinationsArg),
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
        WorkTypeSubcommand::MigrateCoinStore(destinations_arg) => {
            let work_input =
                create_job_params_work(&destinations_arg.destinations_file, |words| {
                    let mut items = words.iter();
                    let address_string = items
                        .next()
                        .ok_or_else(|| anyhow!("no source account provided"))?;
                    let source_account = AccountAddress::from_str_strict(address_string)
                        .map_err(|e| anyhow!("failed to parse {}, {:?}", address_string, e))?;
                    let coin_type = if let Some(coin_type_str) = items.next() {
                        TypeTag::from_str(coin_type_str.trim_matches('"'))
                            .map_err(|e| anyhow!("failed to parse {}, {:?}", coin_type_str, e))?
                    } else {
                        APT_COIN_TYPE.clone()
                    };
                    Ok((source_account, coin_type))
                })?;
            let work = work_input.into_iter().fold(
                Vec::<MigrationJobParams>::new(),
                |mut acc, (source_account, coin_type)| {
                    if let Some(last) = acc.last_mut() {
                        if last.coin_type() == &coin_type && last.source_accounts().len() < 100 { /* cannot migrate more than 400 accounts in a single txn, IO_LIMIT_REACHED otherwise */
                            last.add_source_account(source_account);
                            return acc;
                        }
                    }
                    acc.push(MigrationJobParams::new(vec![source_account], coin_type));
                    acc
                },
            );

            execute_submit(
                work,
                args.submit_args,
                MigrateCoinStoreSignedTransactionBuilder,
                cluster,
                coin_source_account,
                true,
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
