// Copyright © Aptos Foundation

use aptos_block_partitioner::{
    test_utils::{create_signed_p2p_transaction, generate_test_account, TestAccount},
    BlockPartitionerConfig,
};
use aptos_types::transaction::analyzed_transaction::AnalyzedTransaction;
use clap::Parser;
use rand::rngs::OsRng;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::{sync::Mutex, time::Instant};

#[derive(Debug, Parser)]
struct Args {
    #[clap(long, default_value_t = 2000000)]
    pub num_accounts: usize,

    #[clap(long, default_value_t = 100000)]
    pub block_size: usize,

    #[clap(long, default_value_t = 10)]
    pub num_blocks: usize,

    #[clap(long, default_value_t = 12)]
    pub num_shards: usize,
}

fn main() {
    println!("Starting the block partitioning benchmark");
    let args = Args::parse();
    let num_accounts = args.num_accounts;
    println!("Creating {} accounts", num_accounts);
    let accounts: Vec<Mutex<TestAccount>> = (0..num_accounts)
        .into_par_iter()
        .map(|_i| Mutex::new(generate_test_account()))
        .collect();
    println!("Created {} accounts", num_accounts);
    println!("Creating {} transactions", args.block_size);
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

    let partitioner = BlockPartitionerConfig::default()
        .num_shards(args.num_shards)
        .max_partitioning_rounds(2)
        .cross_shard_dep_avoid_threshold(0.9)
        .partition_last_round(true)
        .build();
    for _ in 0..args.num_blocks {
        let transactions = transactions.clone();
        println!("Starting to partition");
        let now = Instant::now();
        partitioner.partition(transactions);
        let elapsed = now.elapsed();
        println!("Time taken to partition: {:?}", elapsed);
    }
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Args::command().debug_assert()
}
