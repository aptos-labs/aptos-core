// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_config::config::{
    EpochSnapshotPrunerConfig, LedgerPrunerConfig, PrunerConfig, StateKvPrunerConfig,
    StateMerklePrunerConfig,
};
use aptos_executor::block_executor::TransactionBlockExecutor;
use aptos_executor_benchmark::{
    benchmark_transaction::BenchmarkTransaction, fake_executor::FakeExecutor,
};
use aptos_metrics_core::{register_int_gauge, IntGauge};
use aptos_push_metrics::MetricsPusher;
use aptos_vm::AptosVM;
use once_cell::sync::Lazy;
use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use structopt::StructOpt;

#[cfg(unix)]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

/// This is needed for filters on the Grafana dashboard working as its used to populate the filter
/// variables.
pub static START_TIME: Lazy<IntGauge> =
    Lazy::new(|| register_int_gauge!("node_process_start_time", "Start time").unwrap());

#[derive(Debug, StructOpt)]
struct PrunerOpt {
    #[structopt(long)]
    enable_state_pruner: bool,

    #[structopt(long)]
    enable_epoch_snapshot_pruner: bool,

    #[structopt(long)]
    enable_ledger_pruner: bool,

    #[structopt(long)]
    enable_state_kv_pruner: bool,

    #[structopt(long, default_value = "100000")]
    state_prune_window: u64,

    #[structopt(long, default_value = "100000")]
    epoch_snapshot_prune_window: u64,

    #[structopt(long, default_value = "100000")]
    ledger_prune_window: u64,

    #[structopt(long, default_value = "100000")]
    state_kv_prune_window: u64,

    #[structopt(long, default_value = "500")]
    ledger_pruning_batch_size: usize,

    #[structopt(long, default_value = "500")]
    state_pruning_batch_size: usize,

    #[structopt(long, default_value = "500")]
    epoch_snapshot_pruning_batch_size: usize,

    #[structopt(long, default_value = "500")]
    state_kv_pruning_batch_size: usize,
}

impl PrunerOpt {
    fn pruner_config(&self) -> PrunerConfig {
        PrunerConfig {
            state_merkle_pruner_config: StateMerklePrunerConfig {
                enable: self.enable_state_pruner,
                prune_window: self.state_prune_window,
                batch_size: self.state_pruning_batch_size,
            },
            epoch_snapshot_pruner_config: EpochSnapshotPrunerConfig {
                enable: self.enable_epoch_snapshot_pruner,
                prune_window: self.epoch_snapshot_prune_window,
                batch_size: self.epoch_snapshot_pruning_batch_size,
            },
            ledger_pruner_config: LedgerPrunerConfig {
                enable: self.enable_ledger_pruner,
                prune_window: self.ledger_prune_window,
                batch_size: self.ledger_pruning_batch_size,
                user_pruning_window_offset: 0,
            },
            state_kv_pruner_config: StateKvPrunerConfig {
                enable: self.enable_state_kv_pruner,
                prune_window: self.state_kv_prune_window,
                batch_size: self.state_kv_pruning_batch_size,
            },
        }
    }
}

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(long, default_value = "10000")]
    block_size: usize,

    #[structopt(long, default_value = "5")]
    transactions_per_sender: usize,

    #[structopt(long)]
    concurrency_level: Option<usize>,

    #[structopt(flatten)]
    pruner_opt: PrunerOpt,

    #[structopt(long)]
    use_state_kv_db: bool,

    #[structopt(subcommand)]
    cmd: Command,

    #[structopt(
        long,
        about = "Verify sequence number of all the accounts after execution finishes"
    )]
    verify_sequence_numbers: bool,

    #[structopt(long)]
    use_fake_executor: bool,
}

impl Opt {
    fn concurrency_level(&self) -> usize {
        match self.concurrency_level {
            None => {
                let level = num_cpus::get();
                println!(
                    "\nVM concurrency level defaults to num of cpus: {}\n",
                    level
                );
                level
            },
            Some(level) => level,
        }
    }
}

#[derive(Debug, StructOpt)]
enum Command {
    CreateDb {
        #[structopt(long, parse(from_os_str))]
        data_dir: PathBuf,

        #[structopt(long, default_value = "1000000")]
        num_accounts: usize,

        #[structopt(long, default_value = "10000000000")]
        init_account_balance: u64,
    },
    RunExecutor {
        #[structopt(
            long,
            default_value = "1000",
            about = "number of transfer blocks to run"
        )]
        blocks: usize,

        #[structopt(long, parse(from_os_str))]
        data_dir: PathBuf,

        #[structopt(long, parse(from_os_str))]
        checkpoint_dir: PathBuf,
    },
    AddAccounts {
        #[structopt(long, parse(from_os_str))]
        data_dir: PathBuf,

        #[structopt(long, parse(from_os_str))]
        checkpoint_dir: PathBuf,

        #[structopt(long, default_value = "1000000")]
        num_new_accounts: usize,

        #[structopt(long, default_value = "1000000")]
        init_account_balance: u64,
    },
}

fn run<E>(opt: Opt)
where
    E: TransactionBlockExecutor<BenchmarkTransaction> + 'static,
{
    match opt.cmd {
        Command::CreateDb {
            data_dir,
            num_accounts,
            init_account_balance,
        } => {
            aptos_executor_benchmark::db_generator::run::<E>(
                num_accounts,
                init_account_balance,
                opt.block_size,
                data_dir,
                opt.pruner_opt.pruner_config(),
                opt.verify_sequence_numbers,
                opt.use_state_kv_db,
            );
        },
        Command::RunExecutor {
            blocks,
            data_dir,
            checkpoint_dir,
        } => {
            aptos_executor_benchmark::run_benchmark::<E>(
                opt.block_size,
                blocks,
                opt.transactions_per_sender,
                data_dir,
                checkpoint_dir,
                opt.verify_sequence_numbers,
                opt.pruner_opt.pruner_config(),
                opt.use_state_kv_db,
            );
        },
        Command::AddAccounts {
            data_dir,
            checkpoint_dir,
            num_new_accounts,
            init_account_balance,
        } => {
            aptos_executor_benchmark::add_accounts::<E>(
                num_new_accounts,
                init_account_balance,
                opt.block_size,
                data_dir,
                checkpoint_dir,
                opt.pruner_opt.pruner_config(),
                opt.verify_sequence_numbers,
                opt.use_state_kv_db,
            );
        },
    }
}

fn main() {
    let opt = Opt::from_args();
    aptos_logger::Logger::new().init();
    START_TIME.set(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64,
    );
    let _mp = MetricsPusher::start_for_local_run("executor-benchmark");

    rayon::ThreadPoolBuilder::new()
        .thread_name(|index| format!("rayon-global-{}", index))
        .build_global()
        .expect("Failed to build rayon global thread pool.");
    AptosVM::set_concurrency_level_once(opt.concurrency_level());
    FakeExecutor::set_concurrency_level_once(opt.concurrency_level());

    if opt.use_fake_executor {
        run::<FakeExecutor>(opt);
    } else {
        run::<AptosVM>(opt);
    }
}
