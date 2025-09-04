// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

mod diag;

use anyhow::{Context, Result};
use velor_logger::{Level, Logger};
use velor_transaction_emitter_lib::{
    create_accounts_command, emit_transactions, Cluster, ClusterArgs, CreateAccountsArgs, EmitArgs,
};
use velor_transaction_workloads_lib::args::EmitWorkloadArgs;
use clap::{Parser, Subcommand};
use diag::diag;

#[derive(Parser, Debug)]
struct Args {
    #[clap(subcommand)]
    command: TxnEmitterCommand,
}

#[derive(Subcommand, Debug)]
enum TxnEmitterCommand {
    /// This is the primary use of the transaction emitter, specifically where
    /// we mint many accounts and then hit the target peer(s) with transactions,
    /// recording stats as we go.
    EmitTx(EmitTx),

    /// Create test accounts, for use with EmitTx
    CreateAccounts(CreateAccounts),

    /// This runs the transaction emitter in diag mode, where the focus is on
    /// FullNodes instead of ValidatorNodes. This performs a simple health check.
    Diag(Diag),

    /// Just pings a set of end points and determines if they are reachable and have
    /// up to date ledger information
    PingEndPoints(PingEndPoints),
}

#[derive(Parser, Debug)]
struct EmitTx {
    #[clap(flatten)]
    cluster_args: ClusterArgs,

    #[clap(flatten)]
    emit_args: EmitArgs,

    #[clap(flatten)]
    emit_workload_args: EmitWorkloadArgs,
}

#[derive(Parser, Debug)]
struct CreateAccounts {
    #[clap(flatten)]
    cluster_args: ClusterArgs,

    #[clap(flatten)]
    create_accounts_args: CreateAccountsArgs,
}

#[derive(Parser, Debug)]
struct PingEndPoints {
    #[clap(flatten)]
    cluster_args: ClusterArgs,
}

#[derive(Parser, Debug)]
struct Diag {
    #[clap(flatten)]
    cluster_args: ClusterArgs,
}

#[tokio::main]
pub async fn main() -> Result<()> {
    Logger::builder().level(Level::Info).build();

    let args = Args::parse();

    // TODO: Check if I need DisplayChain here in the error case.
    match args.command {
        TxnEmitterCommand::EmitTx(args) => {
            let stats = emit_transactions(
                &args.cluster_args,
                &args.emit_args,
                args.emit_workload_args.args_to_transaction_mix_per_phase(),
            )
            .await
            .map_err(|e| panic!("Emit transactions failed {:?}", e))
            .unwrap();
            println!("Total stats: {}", stats);
            println!("Average rate: {}", stats.rate());
            Ok(())
        },
        TxnEmitterCommand::CreateAccounts(args) => {
            create_accounts_command(&args.cluster_args, &args.create_accounts_args)
                .await
                .map_err(|e| panic!("Create accounts failed {:?}", e))
                .unwrap();
            Ok(())
        },
        TxnEmitterCommand::Diag(args) => {
            let cluster = Cluster::try_from_cluster_args(&args.cluster_args)
                .await
                .context("Failed to build cluster")?;
            diag(&cluster).await.context("Diag failed")?;
            Ok(())
        },
        TxnEmitterCommand::PingEndPoints(args) => {
            Cluster::try_from_cluster_args(&args.cluster_args)
                .await
                .context("Failed to build cluster")?;
            Ok(())
        },
    }
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Args::command().debug_assert()
}
