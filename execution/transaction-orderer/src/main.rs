// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright © Aptos Foundation

use aptos_block_partitioner::test_utils::{
    create_signed_p2p_transaction, generate_test_account, TestAccount,
};
use aptos_transaction_orderer::{
    batch_orderer::SequentialDynamicAriaOrderer,
    batch_orderer_with_window::SequentialDynamicWindowOrderer,
    block_orderer::{
        BatchedBlockOrdererWithWindow, BatchedBlockOrdererWithoutWindow, BlockOrderer,
        IdentityBlockOrderer,
    },
    quality::{amortized_inverse_dependency_cost_function, order_total_cost},
    transaction_compressor::{compress_transactions, CompressedPTransaction},
};
use aptos_types::transaction::analyzed_transaction::AnalyzedTransaction;
use clap::{Parser, ValueEnum};
use rand::rngs::OsRng;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::{
    cmp::min,
    io,
    sync::Mutex,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Orderer {
    Aria,
    Window,
    Identity,
}

#[derive(Debug, Parser)]
struct Args {
    #[clap(long, default_value_t = 2000000)]
    pub num_accounts: usize,

    #[clap(long, default_value_t = 100000)]
    pub block_size: usize,

    #[clap(long, default_value_t = 10)]
    pub num_blocks: usize,

    #[clap(long)]
    pub orderer: Orderer,
}

fn run_benchmark<O>(args: Args, block_orderer: O)
where
    O: BlockOrderer<Txn = CompressedPTransaction<AnalyzedTransaction>>,
{
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

    let now = Instant::now();
    let (transactions, compressor) = compress_transactions(transactions);
    println!("Mapping time: {:?}", now.elapsed());

    let min_ordered_transaction_before_execution = min(100, args.block_size);

    for _ in 0..args.num_blocks {
        let transactions = transactions.clone();
        println!("Starting to order");
        let now = Instant::now();

        // Mapping state keys to u64 significantly speeds up the orderer,
        // even including the time it takes to do the mapping itself.
        // When we move to the streaming approach, compression also can (should?) be done
        // in batches instead of doing it for the whole block.

        let mut latency = None;
        let mut count_ordered = 0;
        let mut results = Vec::new();

        block_orderer
            .order_transactions(transactions, |ordered_txns| -> Result<(), io::Error> {
                count_ordered += ordered_txns.len();
                if latency.is_none() && count_ordered >= min_ordered_transaction_before_execution {
                    latency = Some(now.elapsed());
                }
                results.push(ordered_txns);
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

        let odered_txns = results.into_iter().flatten().collect::<Vec<_>>();
        println!(
            "Ordering cost (1 / D): {:?}",
            order_total_cost(&odered_txns, amortized_inverse_dependency_cost_function(0.),)
        );
        println!(
            "Ordering cost (1 / (16 + D)): {:?}",
            order_total_cost(
                &odered_txns,
                amortized_inverse_dependency_cost_function(16.),
            )
        );
        println!(
            "Ordering cost (1 / (50 + D)): {:?}",
            order_total_cost(
                &odered_txns,
                amortized_inverse_dependency_cost_function(50.),
            )
        );
    }
}

fn main() {
    let args = Args::parse();

    match args.orderer {
        Orderer::Aria => {
            let min_ordered_transaction_before_execution = min(100, args.block_size);
            let block_orderer = BatchedBlockOrdererWithoutWindow::new(
                SequentialDynamicAriaOrderer::default(),
                min_ordered_transaction_before_execution * 5,
            );

            run_benchmark(args, block_orderer);
        },
        Orderer::Window => {
            let min_ordered_transaction_before_execution = min(100, args.block_size);
            let block_orderer = BatchedBlockOrdererWithWindow::new(
                SequentialDynamicWindowOrderer::default(),
                min_ordered_transaction_before_execution * 5,
                1000,
            );

            run_benchmark(args, block_orderer);
        },
        Orderer::Identity => {
            let block_orderer = IdentityBlockOrderer::default();

            run_benchmark(args, block_orderer);
        },
    }
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Args::command().debug_assert()
}
