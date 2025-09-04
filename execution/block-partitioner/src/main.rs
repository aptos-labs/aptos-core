// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_block_partitioner::{
    test_utils::P2PBlockGenerator, v2::config::PartitionerV2Config, PartitionerConfig,
};
use velor_logger::info;
use clap::Parser;
use rand::thread_rng;
use std::time::Instant;

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

    #[clap(long, default_value_t = 48)]
    pub num_shards: usize,
}

fn main() {
    velor_logger::Logger::new().init();
    info!("Starting the block partitioning benchmark");
    let args = Args::parse();
    let block_gen = P2PBlockGenerator::new(args.num_accounts);
    let partitioner = PartitionerV2Config::default()
        .max_partitioning_rounds(4)
        .num_threads(8)
        .cross_shard_dep_avoid_threshold(0.9)
        .dashmap_num_shards(64)
        .partition_last_round(false)
        .build();
    let mut rng = thread_rng();
    for _ in 0..args.num_blocks {
        let transactions = block_gen.rand_block(&mut rng, args.block_size);
        info!("Starting to partition");
        let now = Instant::now();
        let _partitioned = partitioner.partition(transactions.clone(), args.num_shards);
        let elapsed = now.elapsed();
        info!("Time taken to partition: {:?}", elapsed);
    }
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Args::command().debug_assert()
}
