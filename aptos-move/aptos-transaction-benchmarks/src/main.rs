// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::_once_cell::sync::Lazy;
use aptos_language_e2e_tests::account_universe::P2PTransferGen;
use aptos_metrics_core::{register_int_gauge, IntGauge};
use aptos_push_metrics::MetricsPusher;
use aptos_transaction_benchmarks::transactions::TransactionBencher;
use aptos_vm_logging::disable_speculative_logging;
use clap::{Parser, Subcommand};
use proptest::prelude::*;
use std::{
    net::SocketAddr,
    time::{SystemTime, UNIX_EPOCH},
};

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
    ParamSweep(ParamSweepOpt),
    Execute(ExecuteOpt),
}

#[derive(Debug, Parser)]
struct ParamSweepOpt {
    #[clap(long, default_value = "200000")]
    pub num_accounts: Vec<usize>,

    #[clap(long)]
    pub block_sizes: Option<Vec<usize>>,

    #[clap(long)]
    pub skip_parallel: bool,

    #[clap(long)]
    pub skip_sequential: bool,

    #[clap(long, default_value_t = 2)]
    pub num_warmups: usize,

    #[clap(long, default_value_t = 10)]
    pub num_runs: usize,

    #[clap(long)]
    pub maybe_block_gas_limit: Option<u64>,

    #[clap(long, default_value_t = true)]
    pub use_txn_payload_v2_format: bool,

    #[clap(long, default_value_t = false)]
    pub use_orderless_transactions: bool,
}

#[derive(Debug, Parser)]
struct ExecuteOpt {
    #[clap(long, default_value_t = 100000)]
    pub num_accounts: usize,

    #[clap(long, default_value_t = 5)]
    pub num_warmups: usize,

    #[clap(long, default_value_t = 10000)]
    pub block_size: usize,

    #[clap(long, default_value_t = 15)]
    pub num_blocks: usize,

    #[clap(long, default_value_t = 8)]
    pub concurrency_level_per_shard: usize,

    #[clap(long, default_value_t = 1)]
    pub num_executor_shards: usize,

    #[clap(long, num_args = 1.., conflicts_with = "num_executor_shards")]
    pub remote_executor_addresses: Option<Vec<SocketAddr>>,

    #[clap(long, default_value_t = true)]
    pub no_conflict_txns: bool,

    #[clap(long)]
    pub maybe_block_gas_limit: Option<u64>,

    #[clap(long, default_value_t = false)]
    pub generate_then_execute: bool,

    #[clap(long, default_value_t = true)]
    pub use_txn_payload_v2_format: bool,

    #[clap(long, default_value_t = false)]
    pub use_orderless_transactions: bool,
}

fn param_sweep(opt: ParamSweepOpt) {
    disable_speculative_logging();

    let block_sizes = opt.block_sizes.unwrap_or_else(|| vec![1000, 10000, 50000]);
    let concurrency_level = num_cpus::get();

    let bencher = TransactionBencher::new(any_with::<P2PTransferGen>((1_000, 1_000_000)));

    let mut par_measurements: Vec<Vec<usize>> = Vec::new();
    let mut seq_measurements: Vec<Vec<usize>> = Vec::new();

    let run_parallel = !opt.skip_parallel;
    let run_sequential = !opt.skip_sequential;

    let maybe_block_gas_limit = opt.maybe_block_gas_limit;

    assert!(
        run_sequential || run_parallel,
        "Must run at least one of parallel or sequential"
    );

    for block_size in &block_sizes {
        for num_accounts in &opt.num_accounts {
            let (mut par_tps, mut seq_tps) = bencher.blockstm_benchmark(
                *num_accounts,
                *block_size,
                run_parallel,
                run_sequential,
                opt.num_warmups,
                opt.num_runs,
                1,
                concurrency_level,
                None,
                false,
                maybe_block_gas_limit,
                false,
                opt.use_txn_payload_v2_format,
                opt.use_orderless_transactions,
            );
            par_tps.sort();
            seq_tps.sort();
            par_measurements.push(par_tps);
            seq_measurements.push(seq_tps);
        }
    }

    println!("\nconcurrency_level = {}\n", concurrency_level);

    let mut i = 0;
    for block_size in &block_sizes {
        for num_accounts in &opt.num_accounts {
            println!(
                "PARAMS: num_account = {}, block_size = {}",
                *num_accounts, *block_size
            );

            let mut seq_tps = 1;
            if run_sequential {
                println!("Sequential TPS: {:?}", seq_measurements[i]);
                let mut seq_sum = 0;
                for m in &seq_measurements[i] {
                    seq_sum += m;
                }
                seq_tps = seq_sum / seq_measurements[i].len();
                println!("Avg Sequential TPS = {:?}", seq_tps,);
            }

            if run_parallel {
                println!("Parallel TPS: {:?}", par_measurements[i]);
                let mut par_sum = 0;
                for m in &par_measurements[i] {
                    par_sum += m;
                }
                let par_tps = par_sum / par_measurements[i].len();
                println!("Avg Parallel TPS = {:?}", par_tps,);
                if run_sequential {
                    println!("Speed up {}x over sequential", par_tps / seq_tps);
                }
            }
            i += 1;
        }
        println!();
    }
}

fn execute(opt: ExecuteOpt) {
    disable_speculative_logging();
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
        opt.remote_executor_addresses,
        opt.no_conflict_txns,
        opt.maybe_block_gas_limit,
        opt.generate_then_execute,
        opt.use_txn_payload_v2_format,
        opt.use_orderless_transactions,
    );

    let sum: usize = par_tps.iter().sum();
    println!("Avg Parallel TPS = {:?}", sum / par_tps.len())
}

fn main() {
    aptos_logger::Logger::new().init();
    START_TIME.set(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64,
    );
    aptos_node_resource_metrics::register_node_metrics_collector();
    let _mp = MetricsPusher::start_for_local_run("block-stm-benchmark");
    let args = Args::parse();

    // TODO: Check if I need DisplayChain here in the error case.
    match args.command {
        BenchmarkCommand::ParamSweep(opt) => param_sweep(opt),
        BenchmarkCommand::Execute(opt) => execute(opt),
    }
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Args::command().debug_assert()
}
