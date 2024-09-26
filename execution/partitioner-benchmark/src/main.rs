// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

mod clustered_txns_generator;

use std::time::Instant;
use clap::Parser;
use aptos_block_partitioner::v3::V3NaivePartitioner;
use aptos_block_partitioner::BlockPartitioner;
use aptos_logger::{error, info};
use aptos_streaming_partitioner::V3FennelBasedPartitioner;
use aptos_transaction_orderer::V3ReorderingPartitioner;
use clustered_txns_generator::ClusteredTxnsGenerator;
use aptos_types::transaction::analyzed_transaction::AnalyzedTransaction;

#[derive(Debug, Parser)]
struct Args {
    #[clap(long, default_value_t = 10)]
    pub num_clusters: usize,

    #[clap(long, default_value_t = 1000)]
    pub total_user_accounts: usize,

    #[clap(long, default_value_t = 100)]
    pub num_resource_addresses_per_cluster: usize,

    #[clap(long, default_value_t = 10)]
    pub mean_txns_per_user: usize,

    #[clap(long, default_value_t = 0.1)]
    pub cluster_size_relative_std_dev: f64,

    #[clap(long, default_value_t = 0.1)]
    pub txns_per_user_relative_std_dev: f64,

    #[clap(long, default_value_t = 0.1)]
    pub fraction_of_external_txns: f64,

    #[clap(long, default_value_t = 10000)]
    pub num_txns: usize,

    #[clap(long, default_value_t = String::from("v3-naive"))]
    pub partitioner_type: String,

    #[clap(long, default_value_t = 10)]
    pub num_shards: usize,

    #[clap(long)]
    pub debug_logs: bool,

    #[clap(long, default_value_t = 100)]
    pub v3_reorderer_min_ordered_transaction_before_execution: usize,

    #[clap(long, default_value_t = 1000)]
    pub v3_reorderer_max_window_size: usize,
}

fn main() {
    aptos_logger::Logger::new().init();
    let args = Args::parse();

    // Create the transaction generator
    let generator = ClusteredTxnsGenerator::new(
        args.num_clusters,
        args.total_user_accounts,
        args.num_resource_addresses_per_cluster,
        args.mean_txns_per_user,
        args.cluster_size_relative_std_dev,
        args.txns_per_user_relative_std_dev,
        args.fraction_of_external_txns,
        args.debug_logs,
    );

    info!("Generating {} transactions", args.num_txns);
    // Generate transactions
    let txns: Vec<AnalyzedTransaction> = generator.generate(args.num_txns);
    assert_eq!(args.num_txns, txns.len());

    // Determine the partitioner type
    let partitioner: Box<dyn BlockPartitioner> = match args.partitioner_type.as_str() {
        "v3-naive" => Box::new(V3NaivePartitioner { print_debug_stats: args.debug_logs }),
        "v3-orderer" => Box::new(V3ReorderingPartitioner {
            print_debug_stats: args.debug_logs,
            min_ordered_transaction_before_execution: args.v3_reorderer_min_ordered_transaction_before_execution,
            max_window_size: args.v3_reorderer_max_window_size,
        }),
        "v3-fennel" => Box::new(V3FennelBasedPartitioner { print_debug_stats: args.debug_logs }),
        _ => {
            error!("Unsupported partitioner type: {}", args.partitioner_type);
            return;
        }
    };

    // Partition the transactions
    let num_txns = txns.len();
    let start_time = Instant::now();
    let partitioned_txns = partitioner.partition(txns, args.num_shards);
    let elapsed_time = start_time.elapsed();
    assert_eq!(num_txns, partitioned_txns.num_sharded_txns());
    info!("Partitioning tps {:.2} ({} txns / {:.2} s; debug prints {})",
          num_txns as f64 / elapsed_time.as_secs_f64(), num_txns, elapsed_time.as_secs_f64(), args.debug_logs);
}
