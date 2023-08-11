// Copyright © Aptos Foundation

use aptos_block_partitioner::{
    test_utils::{
        create_signed_p2p_transaction, generate_test_account, P2PBlockGenerator, TestAccount,
    },
    v2::PartitionerV2,
    BlockPartitioner, PartitionerV1Config,
};
use aptos_logger::info;
use aptos_types::transaction::analyzed_transaction::AnalyzedTransaction;
use clap::Parser;
use rand::{rngs::OsRng, thread_rng};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::{sync::Mutex, time::Instant};

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
    aptos_logger::Logger::new().init();
    info!("Starting the block partitioning benchmark");
    let args = Args::parse();
    let block_gen = P2PBlockGenerator::new(args.num_accounts);
    let partitioner = PartitionerV2::new(8, 4, 10, 64, true);
    let mut rng = thread_rng();
    for _ in 0..args.num_blocks {
        let transactions = block_gen.rand_block(&mut rng, args.block_size);
        info!("Starting to partition");
        let now = Instant::now();
        let partitioned = partitioner.partition(transactions.clone(), args.num_shards);
        let elapsed = now.elapsed();
        info!("Time taken to partition: {:?}", elapsed);
    }
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Args::command().debug_assert()
}
