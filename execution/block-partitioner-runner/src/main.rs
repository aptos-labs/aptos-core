// Copyright © Aptos Foundation

use aptos_block_partitioner::{sharded_block_partitioner::ShardedBlockPartitioner, test_utils::{create_signed_p2p_transaction, generate_test_account, TestAccount}};
use aptos_types::transaction::analyzed_transaction::AnalyzedTransaction;
use clap::Parser;
use rand::rngs::OsRng;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::{sync::Mutex, time::Instant};
use aptos_logger::info;
use aptos_block_partitioner_runner::build_partitioner;
use aptos_block_partitioner::assertions;

#[cfg(unix)]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[derive(Debug, Parser)]
struct Args {
    #[clap(long, default_value_t = 1000000)]
    pub num_accounts: usize,

    #[clap(long, default_value_t = 100000)]
    pub block_size: usize,

    #[clap(long, default_value_t = 9)]
    pub num_blocks: usize,

    #[clap(long, default_value_t = 60)]
    pub num_shards: usize,
}

fn main() {
    aptos_logger::Logger::new().init();
    info!("Starting the block partitioning benchmark");
    let args = Args::parse();
    let num_accounts = args.num_accounts;
    info!("Creating {} accounts", num_accounts);
    let accounts: Vec<Mutex<TestAccount>> = (0..num_accounts)
        .into_par_iter()
        .map(|_i| Mutex::new(generate_test_account()))
        .collect();
    info!("Created {} accounts", num_accounts);
    info!("Creating {} transactions", args.block_size);
    let partitioner = build_partitioner(Some(args.num_shards));
    for _ in 0..args.num_blocks {
        let transactions: Vec<AnalyzedTransaction> = (0..args.block_size)
            .map(|_| {
                // randomly select a sender and receiver from accounts
                let mut rng = OsRng;

                let indices = rand::seq::index::sample(&mut rng, num_accounts, 2);
                let receiver = accounts[indices.index(1)].lock().unwrap();
                let mut sender = accounts[indices.index(0)].lock().unwrap();
                create_signed_p2p_transaction(&mut sender, vec![&receiver]).remove(0)
            })
            .collect();
        let txns_clone = transactions.clone();
        info!("Starting to partition");
        let now = Instant::now();
        let partitioned = partitioner.partition(transactions, args.num_shards);
        let elapsed = now.elapsed();
        info!("Time taken to partition: {:?}", elapsed);
        assertions(&txns_clone, &partitioned);
    }
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Args::command().debug_assert()
}
