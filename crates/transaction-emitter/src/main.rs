// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

mod diag;

use anyhow::Result;
use clap::{Parser, Subcommand};
use transaction_emitter_lib::{ClusterArgs, EmitArgs};

use std::time::Duration;
use tokio_metrics::TaskMonitor;

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
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let metrics_monitor = tokio_metrics::TaskMonitor::new();

    // print task metrics every 500ms
    {
        let metrics_monitor = metrics_monitor.clone();
        tokio::spawn(async move {
            for deltas in metrics_monitor.intervals() {
                // pretty-print the metric deltas
                println!("{:?}", deltas);
                // wait 500ms
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        });
    }

    // instrument some tasks and await them
    tokio::join![
        metrics_monitor.instrument(do_work()),
        metrics_monitor.instrument(do_work()),
        metrics_monitor.instrument(do_work())
    ];

    Ok(())
}

async fn do_work() {
    for _ in 0..25 {
        tokio::task::yield_now().await;
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
