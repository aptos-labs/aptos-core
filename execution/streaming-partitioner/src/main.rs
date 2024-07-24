// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright © Aptos Foundation

use aptos_block_partitioner::test_utils::{
    create_signed_p2p_transaction, generate_test_account, TestAccount,
};
#[cfg(feature = "metis-partitioner")]
use aptos_graphs::partitioning::{metis::MetisGraphPartitioner, WholeGraphStreamingPartitioner};
use aptos_graphs::{
    graph::{EdgeWeight, NodeWeight},
    partitioning::{
        fennel::{AlphaComputationMode, BalanceConstraintMode, FennelGraphPartitioner},
        PartitionId,
        random::RandomPartitioner,
    },
};
use aptos_streaming_partitioner::{PartitionedTransaction, SerializationIdx, StreamingTransactionPartitioner, transaction_graph_partitioner, transaction_graph_partitioner::TransactionGraphPartitioner};
use aptos_transaction_orderer::transaction_compressor::{compress_transactions, CompressedPTransaction, CompressedPTransactionInner};
use aptos_types::{
    batched_stream::{Batched, BatchedStream},
    transaction::analyzed_transaction::AnalyzedTransaction,
};
use clap::{Parser, ValueEnum};
use rand::rngs::OsRng;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::{
    fmt::Debug,
    sync::Mutex,
    time::{Duration, Instant},
};
use std::collections::BTreeSet;
use std::rc::Rc;
use aptos_block_partitioner::BlockPartitioner;
use aptos_types::block_executor::partitioner::PartitionedTransactions;

#[derive(Debug, Copy, Clone, ValueEnum)]
enum Partitioner {
    Fennel,
    #[cfg(feature = "metis-partitioner")]
    Metis,
    Random,
}

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

    #[clap(long, value_enum, default_value_t = Partitioner::Fennel)]
    pub partitioner: Partitioner,
}

fn run_benchmark<S, P>(transactions: S, mut partitioner: P, args: Args)
where
    S: BatchedStream<StreamItem = CompressedPTransaction<AnalyzedTransaction>> + Clone,
    P: StreamingTransactionPartitioner<S>,
    P::Error: Debug,
{
    for _ in 0..args.num_blocks {
        let transactions = transactions.clone();
        println!("Starting to order");
        let start = Instant::now();

        let mut latency = None;
        let stream = partitioner.partition_transactions(transactions).unwrap();

        let mut txns_by_partition = vec![vec![]; args.num_shards];
        let mut partition_by_txn = vec![0; args.block_size];

        for batch in stream.unwrap_batches().into_no_error_batch_iter() {
            if latency.is_none() {
                latency = Some(start.elapsed());
            }

            for tx in batch {
                partition_by_txn[tx.serialization_idx as usize] = tx.partition;
                txns_by_partition[tx.partition as usize].push(tx);
            }
        }

        let elapsed = start.elapsed();
        assert!(latency.is_some());
        println!("Time taken to partition: {:?}", elapsed);
        println!(
            "Throughput: {} TPS",
            (Duration::from_secs(1).as_nanos() * (args.block_size as u128)) / elapsed.as_nanos()
        );
        println!("Latency: {:?}", latency.unwrap());

        let mut cut_edges_weight = 0 as EdgeWeight;
        let mut total_edges_weight = 0 as EdgeWeight;

        // Compute the cut edges weight and the total edges weight.
        for (partition_idx, partition) in txns_by_partition.iter().enumerate() {
            for tx in partition {
                for &dep in tx.dependencies.keys() {
                    let edge_weight = edge_weight_function(tx.serialization_idx, dep);
                    total_edges_weight += edge_weight;
                    if partition_by_txn[dep as usize] != partition_idx as PartitionId {
                        cut_edges_weight += edge_weight;
                    }
                }
            }
        }

        println!(
            "Cut edges weight: {} / {} ({:.2})",
            cut_edges_weight,
            total_edges_weight,
            cut_edges_weight as f64 / total_edges_weight as f64
        );
    }
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

    let now = Instant::now();
    let transactions = compress_transactions(transactions);
    println!("Mapping time: {:?}", now.elapsed());
    let transactions = transactions.into_iter().batched(args.block_size);
    let x = |_: &CompressedPTransaction<AnalyzedTransaction>| 1 as NodeWeight;
    let mut params = transaction_graph_partitioner::Params {
        node_weight_function: x,
        edge_weight_function,
        shuffle_batches: false,
    };

    match args.partitioner {
        Partitioner::Fennel => {
            let mut fennel = FennelGraphPartitioner::new(args.num_shards);
            fennel.balance_constraint_mode = BalanceConstraintMode::Batched;
            fennel.alpha_computation_mode = AlphaComputationMode::Batched;
            params.shuffle_batches = true;
            run_benchmark(
                transactions,
                TransactionGraphPartitioner::new(fennel, params),
                args,
            )
        },
        #[cfg(feature = "metis-partitioner")]
        Partitioner::Metis => {
            let metis = MetisGraphPartitioner::new(args.num_shards);
            let metis_streaming = WholeGraphStreamingPartitioner::new(metis);
            run_benchmark(
                transactions,
                TransactionGraphPartitioner::new(metis_streaming, params),
                args,
            )
        },
        Partitioner::Random => {
            let random_partitioner = RandomPartitioner::new(args.num_shards);
            run_benchmark(
                transactions,
                TransactionGraphPartitioner::new(random_partitioner, params),
                args,
            )
        },
    };
}

pub fn edge_weight_function(idx1: SerializationIdx, idx2: SerializationIdx) -> EdgeWeight {
    ((1. / (1. + idx1 as f64 - idx2 as f64)) * 100000.) as EdgeWeight
}
