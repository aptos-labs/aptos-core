// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

mod diag;

use ::aptos_logger::{Level, Logger};
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use diag::diag;
use std::time::Duration;
use transaction_emitter_lib::{emit_transactions, Cluster, ClusterArgs, EmitArgs};

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

    /// This runs the transaction emitter in diag mode, where the focus is on
    /// FullNodes instead of ValidatorNodes. This performs a simple health check.
    Diag(Diag),
}

#[derive(Parser, Debug)]
struct EmitTx {
    #[clap(flatten)]
    cluster_args: ClusterArgs,

    #[clap(flatten)]
    emit_args: EmitArgs,
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
            let stats = emit_transactions(&args.cluster_args, &args.emit_args)
                .await
                .context("Emit transactions failed")?;
            println!("Total stats: {}", stats);
            println!(
                "Average rate: {}",
                stats.rate(Duration::from_secs(args.emit_args.duration))
            );
            Ok(())
        }
        TxnEmitterCommand::Diag(args) => {
            let cluster = Cluster::try_from_cluster_args(&args.cluster_args)
                .await
                .context("Failed to build cluster")?;
            diag(&cluster).await.context("Diag failed")?;
            Ok(())
        }
    }
}
