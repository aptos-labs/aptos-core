// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::_once_cell::sync::Lazy;
use aptos_language_e2e_tests::account_universe::P2PTransferGen;
use aptos_metrics_core::{register_int_gauge, IntGauge};
use aptos_push_metrics::MetricsPusher;
use aptos_transaction_benchmarks::transactions::TransactionBencher;
use clap::{Parser, Subcommand};
use proptest::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};

/// This is needed for filters on the Grafana dashboard working as its used to populate the filter
/// variables.
pub static START_TIME: Lazy<IntGauge> =
    Lazy::new(|| register_int_gauge!("node_process_start_time", "Start time").unwrap());

#[derive(Parser, Debug)]
struct Args {
    #[clap(subcommand)]
    command: BenchmarkCommand,
}

#[derive(Subcommand, Debug)]
enum BenchmarkCommand {
    CompareParallelAndSeq(ParallelAndSeqOpt),
    ParallelExecution(ParallelExecutionOpt),
}

#[derive(Debug, Parser)]
struct ParallelAndSeqOpt {
    #[clap(long, default_value = "100000")]
    pub num_accounts: Vec<usize>,

    #[clap(long)]
    pub num_txns: Option<Vec<usize>>,

    #[clap(long)]
    pub run_parallel: bool,

    #[clap(long)]
    pub run_sequential: bool,

    #[clap(long, default_value = "2")]
    pub num_warmups: usize,

    #[clap(long, default_value = "10")]
    pub num_runs: usize,
}

#[derive(Debug, Parser)]
struct ParallelExecutionOpt {
    #[clap(long, default_value = "10000")]
    pub num_accounts: usize,

    #[clap(long, default_value = "2")]
    pub num_warmups: usize,

    #[clap(long, default_value = "50000")]
    pub block_size: usize,

    #[clap(long, default_value = "10")]
    pub num_blocks: usize,

    #[clap(long, default_value = "8")]
    pub concurrency_level_per_shard: usize,

    #[clap(long, default_value = "1")]
    pub num_executor_shards: usize,

    #[clap(long, default_value = "true")]
    pub no_conflict_txns: bool,
}

fn compare_parallel_and_seq(opt: ParallelAndSeqOpt) {
    let num_txns = opt.num_txns.unwrap_or_else(|| vec![1000, 10000, 50000]);
    let concurrency_level = num_cpus::get();

    let bencher = TransactionBencher::new(any_with::<P2PTransferGen>((1_000, 1_000_000)));

    let mut par_measurements: Vec<Vec<usize>> = Vec::new();
    let mut seq_measurements: Vec<Vec<usize>> = Vec::new();

    for block_size in &num_txns {
        for num_accounts in &opt.num_accounts {
            let (mut par_tps, mut seq_tps) = bencher.blockstm_benchmark(
                *num_accounts,
                *block_size,
                opt.run_parallel,
                opt.run_sequential,
                opt.num_warmups,
                opt.num_runs,
                1,
                concurrency_level,
                false,
            );
            par_tps.sort();
            seq_tps.sort();
            par_measurements.push(par_tps);
            seq_measurements.push(seq_tps);
        }
    }

    println!("\nconcurrency_level = {}\n", concurrency_level);

    let mut i = 0;
    for block_size in &num_txns {
        for num_accounts in &opt.num_accounts {
            println!(
                "PARAMS: num_account = {}, block_size = {}",
                *num_accounts, *block_size
            );

            let mut seq_tps = 1;
            if opt.run_sequential {
                println!("Sequential TPS: {:?}", seq_measurements[i]);
                let mut seq_sum = 0;
                for m in &seq_measurements[i] {
                    seq_sum += m;
                }
                seq_tps = seq_sum / seq_measurements[i].len();
                println!("Avg Sequential TPS = {:?}", seq_tps,);
            }

            if opt.run_parallel {
                println!("Parallel TPS: {:?}", par_measurements[i]);
                let mut par_sum = 0;
                for m in &par_measurements[i] {
                    par_sum += m;
                }
                let par_tps = par_sum / par_measurements[i].len();
                println!("Avg Parallel TPS = {:?}", par_tps,);
                if opt.run_sequential {
                    println!("Speed up {}x over sequential", par_tps / seq_tps);
                }
            }
            i += 1;
        }
        println!();
    }
}

fn parallel_execution(opt: ParallelExecutionOpt) {
    aptos_logger::Logger::new().init();
    START_TIME.set(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64,
    );
    let _mp = MetricsPusher::start_for_local_run("blockstm-benchmark");

    let bencher = TransactionBencher::new(any_with::<P2PTransferGen>((1_000, 1_000_000)));

    let (par_tps, _) = bencher.blockstm_benchmark(
        opt.num_accounts,
        opt.block_size,
        true,
        false,
        opt.num_warmups,
        opt.num_blocks,
        opt.num_executor_shards,
        opt.concurrency_level_per_shard,
        opt.no_conflict_txns,
    );

    let sum: usize = par_tps.iter().sum();
    println!("Avg Parallel TPS = {:?}", sum / par_tps.len())
}

fn main() {
    let args = Args::parse();

    // TODO: Check if I need DisplayChain here in the error case.
    match args.command {
        BenchmarkCommand::CompareParallelAndSeq(opt) => compare_parallel_and_seq(opt),
        BenchmarkCommand::ParallelExecution(opt) => parallel_execution(opt),
    }
}
