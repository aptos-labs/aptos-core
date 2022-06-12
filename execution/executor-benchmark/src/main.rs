// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_config::config::StoragePrunerConfig;
use aptos_secure_push_metrics::MetricsPusher;
use aptos_vm::AptosVM;
use std::path::PathBuf;
use structopt::StructOpt;

#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[derive(Debug, StructOpt)]
struct PrunerOpt {
    #[structopt(long, default_value = "100000", help = "Set to -1 to disable.")]
    state_prune_window: i64,

    #[structopt(long, default_value = "100000", help = "Set to -1 to disable.")]
    ledger_prune_window: i64,

    #[structopt(long, default_value = "500")]
    pruning_batch_size: usize,
}

impl PrunerOpt {
    fn pruner_config(&self) -> StoragePrunerConfig {
        StoragePrunerConfig {
            state_store_prune_window: if self.state_prune_window == -1 {
                None
            } else {
                Some(self.state_prune_window as u64)
            },
            ledger_prune_window: if self.ledger_prune_window == -1 {
                None
            } else {
                Some(self.ledger_prune_window as u64)
            },
            pruning_batch_size: self.pruning_batch_size,
        }
    }
}

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(long, default_value = "500")]
    block_size: usize,

    #[structopt(long)]
    concurrency_level: Option<usize>,

    #[structopt(flatten)]
    pruner_opt: PrunerOpt,

    #[structopt(subcommand)]
    cmd: Command,

    #[structopt(
        long,
        about = "Verify sequence number of all the accounts after execution finishes"
    )]
    verify_sequence_numbers: bool,
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
            }
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

        #[structopt(long, default_value = "1000000")]
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
}

fn main() {
    let _mp = MetricsPusher::start();
    let opt = Opt::from_args();

    rayon::ThreadPoolBuilder::new()
        .thread_name(|index| format!("rayon-global-{}", index))
        .build_global()
        .expect("Failed to build rayon global thread pool.");
    AptosVM::set_concurrency_level_once(opt.concurrency_level());

    match opt.cmd {
        Command::CreateDb {
            data_dir,
            num_accounts,
            init_account_balance,
        } => {
            executor_benchmark::db_generator::run(
                num_accounts,
                init_account_balance,
                opt.block_size,
                data_dir,
                opt.pruner_opt.pruner_config(),
                opt.verify_sequence_numbers,
            );
        }
        Command::RunExecutor {
            blocks,
            data_dir,
            checkpoint_dir,
        } => {
            aptos_logger::Logger::new().init();
            executor_benchmark::run_benchmark(
                opt.block_size,
                blocks,
                data_dir,
                checkpoint_dir,
                opt.verify_sequence_numbers,
                opt.pruner_opt.pruner_config(),
            );
        }
    }
}
