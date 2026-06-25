// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! CLI for the MonoMove-vs-MoveVM replay benchmark.
//!
//! - `capture` fetches transactions from chain into a self-contained dump (the transaction plus a
//!   read-set with the full module dependency closure).
//! - `bench` replays each entry-function transaction in a dump on both the legacy Move VM (V1) and
//!   MonoMove (V2), comparing execution time (primary) and correctness (secondary, coarse).

use anyhow::Result;
use aptos_rest_client::AptosBaseUrl;
use clap::{Parser, Subcommand};
use mono_move_replay_benchmark::{
    capture, data, report::TransactionReport, timing::TimingConfig, v1, v2, BenchmarkRun,
};
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    path::PathBuf,
    str::FromStr,
};
use url::Url;

/// The chain to capture from. Mirrors [`AptosBaseUrl`]: a named network or a custom REST endpoint.
#[derive(Clone)]
enum Network {
    Mainnet,
    Testnet,
    Devnet,
    Custom(Url),
}

impl FromStr for Network {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        Ok(match s {
            "mainnet" => Network::Mainnet,
            "testnet" => Network::Testnet,
            "devnet" => Network::Devnet,
            url => Network::Custom(Url::parse(url).map_err(|e| e.to_string())?),
        })
    }
}

impl From<Network> for AptosBaseUrl {
    fn from(network: Network) -> Self {
        match network {
            Network::Mainnet => AptosBaseUrl::Mainnet,
            Network::Testnet => AptosBaseUrl::Testnet,
            Network::Devnet => AptosBaseUrl::Devnet,
            Network::Custom(url) => AptosBaseUrl::Custom(url),
        }
    }
}

#[derive(Parser)]
#[command(about = "Replay-benchmark MonoMove (V2) against the legacy Move VM (V1).")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Replay a dump on both VMs and compare time + correctness.
    Bench(BenchArgs),
    /// Capture transactions from chain into a self-contained dump.
    Capture(CaptureArgs),
}

#[derive(Parser)]
struct BenchArgs {
    #[clap(long, help = "Path to the transactions file")]
    transactions_file: Option<String>,
    #[clap(long, help = "Path to the inputs (read-sets) file")]
    inputs_file: Option<String>,
    #[clap(
        long,
        help = "Directory of `<version>_txns` / `<version>_inputs` pairs; benchmarks them all \
                (alternative to --transactions-file/--inputs-file)"
    )]
    data_dir: Option<String>,
    #[clap(
        long,
        default_value_t = 50,
        help = "Warm-up iterations (discarded) per VM"
    )]
    warmup: usize,
    #[clap(long, default_value_t = 200, help = "Timed samples per VM")]
    samples: usize,
    #[clap(long, help = "Benchmark at most this many transactions")]
    limit: Option<usize>,
    #[clap(
        long,
        help = "Enable V1 (legacy Move VM) paranoid type checks (default: off)"
    )]
    v1_paranoid: bool,
}

#[derive(Parser)]
struct CaptureArgs {
    #[clap(
        long,
        default_value = "mainnet",
        help = "Network to capture from: mainnet, testnet, devnet, or a custom REST endpoint URL"
    )]
    network: Network,
    #[clap(long, help = "Optional API key to raise the request-rate quota")]
    api_key: Option<String>,
    #[clap(long, help = "First transaction version to capture (inclusive)")]
    begin_version: u64,
    #[clap(long, help = "Last transaction version to capture (inclusive)")]
    end_version: u64,
    #[clap(long, help = "Output directory for the captured dump")]
    out_dir: PathBuf,
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Bench(args) => bench(args),
        Command::Capture(args) => {
            if args.end_version < args.begin_version {
                anyhow::bail!("--end-version must be >= --begin-version");
            }
            let versions = (args.begin_version..=args.end_version).collect();
            capture::run(args.network.into(), args.api_key, versions, args.out_dir)
        },
    }
}

fn bench(args: BenchArgs) -> Result<()> {
    // Set once, before V1 builds its VM environment (it's a write-once global).
    aptos_vm_environment::prod_configs::set_paranoid_type_checks(args.v1_paranoid);

    let timing = TimingConfig {
        warmup: args.warmup,
        samples: args.samples,
    };

    let mut inputs = match (&args.data_dir, &args.transactions_file, &args.inputs_file) {
        (Some(dir), None, None) => data::load_inputs_from_dir(dir)?,
        (None, Some(txns), Some(inputs)) => data::load_inputs(txns, inputs)?,
        _ => anyhow::bail!(
            "provide either --data-dir, or both --transactions-file and --inputs-file"
        ),
    };
    if let Some(limit) = args.limit {
        inputs.truncate(limit);
    }
    if inputs.is_empty() {
        anyhow::bail!("No entry-function transactions found in the provided files");
    }

    println!(
        "Benchmarking {} entry-function transaction(s): warmup={}, samples={}\n",
        inputs.len(),
        timing.warmup,
        timing.samples
    );

    for input in &inputs {
        let function = format!("{}::{}", input.entry.module(), input.entry.function());
        // Isolate each VM run: a panic (e.g. a MonoMove feature not yet implemented) becomes that
        // VM's failure reason and never aborts the whole benchmark.
        let v1 = run_vm(|| v1::run(input, &timing));
        let v2 = run_vm(|| v2::run(input, &timing));
        TransactionReport::new(input.version, function, v1, v2).print();
    }
    Ok(())
}

fn run_vm(f: impl FnOnce() -> Result<BenchmarkRun>) -> Result<BenchmarkRun, String> {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(run)) => Ok(run),
        Ok(Err(err)) => Err(format!("{:#}", err)),
        Err(panic) => Err(format!("panicked: {}", panic_message(&panic))),
    }
}

fn panic_message(panic: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = panic.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = panic.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic".to_string()
    }
}
