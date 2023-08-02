// Copyright © Aptos Foundation

use std::cmp::min;
use std::time::Duration;
use std::{io, sync::Mutex, time::Instant};

use clap::Parser;
use rand::rngs::OsRng;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use aptos_block_partitioner::test_utils::{
    create_signed_p2p_transaction, generate_test_account, TestAccount,
};
use aptos_transaction_orderer::batch_orderer::SequentialDynamicAriaOrderer;
use aptos_transaction_orderer::block_orderer::BatchedBlockOrdererWithWindow;
use aptos_transaction_orderer::block_partitioner::{BlockPartitioner, OrderedRoundRobinPartitioner};
use aptos_types::transaction::analyzed_transaction::AnalyzedTransaction;

#[derive(Debug, Parser)]
struct Args {
    #[clap(long, default_value_t = 2000000)]
    pub num_accounts: usize,

    #[clap(long, default_value_t = 100000)]
    pub block_size: usize,

    #[clap(long, default_value_t = 10)]
    pub num_blocks: usize,

    #[clap(long, default_value_t = 4)]
    pub num_shards: usize,
}

fn main() {
    println!("Starting the transaction orderer benchmark");
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

    let min_ordered_transaction_before_execution = min(100, args.block_size);
    let block_orderer = BatchedBlockOrdererWithWindow::new(
        SequentialDynamicAriaOrderer::with_window(),
        min_ordered_transaction_before_execution * 5,
        1000,
    );
    let block_partitioner = OrderedRoundRobinPartitioner::new(
        block_orderer,
        args.num_shards,
        (min_ordered_transaction_before_execution + args.num_shards - 1) / args.num_shards,
    );

    for _ in 0..args.num_blocks {
        let transactions = transactions.clone();
        println!("Starting to order");
        let now = Instant::now();
        let mut latency = None;
        let mut count_ordered = 0;

        block_partitioner.partition_transactions(transactions, |sharded_txns| -> Result<(), io::Error> {
                count_ordered += sharded_txns.iter().map(|txns| txns.len()).sum::<usize>();
                if latency.is_none() && count_ordered >= min_ordered_transaction_before_execution {
                    latency = Some(now.elapsed());
                }
                // println!("Partitioned {} transactions ({} new)", count_ordered,
                //          sharded_txns.iter().map(|txns| txns.len()).sum::<usize>());
                Ok(())
            })
            .unwrap();

        let elapsed = now.elapsed();
        assert!(latency.is_some());
        println!("Time taken to order: {:?}", elapsed);
        println!(
            "Throughput: {} TPS",
            (Duration::from_secs(1).as_nanos() * (args.block_size as u128)) / elapsed.as_nanos()
        );
        println!("Latency: {:?}", latency.unwrap());
    }
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Args::command().debug_assert()
}
