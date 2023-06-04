// Copyright © Aptos Foundation

use aptos_block_partitioner::{
    sharded_block_partitioner::ShardedBlockPartitioner,
    test_utils::{create_signed_p2p_transaction, generate_test_account, TestAccount},
};
use aptos_types::transaction::analyzed_transaction::AnalyzedTransaction;
use clap::Parser;
use rand::{rngs::OsRng, Rng};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::{sync::Mutex, time::Instant};

#[derive(Debug, Parser)]
struct Args {
    #[clap(long, default_value = "10")]
    pub num_accounts: usize,

    #[clap(long, default_value = "9")]
    pub block_size: usize,

    #[clap(long, default_value = "1")]
    pub num_blocks: usize,

    #[clap(long, default_value = "3")]
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
        .into_iter()
        .map(|_| {
            // randomly select a sender and receiver from accounts
            let mut rng = OsRng;
            let sender_index = rng.gen_range(0, num_accounts);
            let mut receiver_index = 0;
            loop {
                receiver_index = rng.gen_range(0, num_accounts);
                if receiver_index != sender_index {
                    break;
                }
            }
            let receiver = accounts[receiver_index].lock().unwrap();
            let mut sender = accounts[sender_index].lock().unwrap();
            create_signed_p2p_transaction(&mut sender, vec![&receiver]).remove(0)
        })
        .collect();

    let partitioner = ShardedBlockPartitioner::new(args.num_shards);
    for _ in 0..args.num_blocks {
        let transactions = transactions.clone();
        println!("Starting to partition");
        let now = Instant::now();
        let sub_blocks = partitioner.partition(transactions, 3);
        for (i, sub_block) in sub_blocks.into_iter().enumerate() {
            let shard_id = i % args.num_shards;
            let b = sub_block.start_index;
            for (j, txn) in sub_block.transactions.into_iter().enumerate() {
                let txn_id = b + j;
                let sender = txn.txn.sender().as_ref().unwrap().brief();
                let recipient = txn.txn.recipient.as_ref().unwrap().brief();
                let deps = txn.cross_shard_dependencies;
                println!("sub_block_id={i}, shard_id={shard_id}, txn_id={txn_id}, sender={sender}, recipient={recipient}, deps={deps:?}");
            }
        }
        let elapsed = now.elapsed();
        println!("Time taken to partition: {:?}", elapsed);
    }
}
