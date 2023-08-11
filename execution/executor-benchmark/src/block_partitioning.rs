// Copyright © Aptos Foundation

use crate::pipeline::ExecuteBlockMessage;
use aptos_block_partitioner::{
    sharded_block_partitioner::ShardedBlockPartitioner, BlockPartitionerConfig,
};
use aptos_crypto::HashValue;
use aptos_logger::info;
use aptos_types::{
    block_executor::partitioner::{ExecutableBlock, ExecutableTransactions},
    transaction::Transaction,
};
use std::time::Instant;

pub(crate) struct BlockPartitioningStage {
    num_blocks_processed: usize,
    maybe_partitioner: Option<ShardedBlockPartitioner>,
}

impl BlockPartitioningStage {
    pub fn new(num_shards: usize, partition_last_round: bool) -> Self {
        let maybe_partitioner = if num_shards <= 1 {
            None
        } else {
            info!("Starting a sharded block partitioner with {} shards and last round partitioning {}", num_shards, partition_last_round);
            let partitioner = BlockPartitionerConfig::default()
                .num_shards(num_shards)
                .max_partitioning_rounds(4)
                .cross_shard_dep_avoid_threshold(0.95)
                .partition_last_round(partition_last_round)
                .build();
            Some(partitioner)
        };

        Self {
            num_blocks_processed: 0,
            maybe_partitioner,
        }
    }

    pub fn process(&mut self, mut txns: Vec<Transaction>) -> ExecuteBlockMessage {
        let current_block_start_time = Instant::now();
        info!(
            "In iteration {}, received {:?} transactions.",
            self.num_blocks_processed,
            txns.len()
        );
        let block_id = HashValue::random();
        let block: ExecutableBlock = match &self.maybe_partitioner {
            None => (block_id, txns).into(),
            Some(partitioner) => {
                let last_txn = txns.pop().unwrap();
                let analyzed_transactions = txns.into_iter().map(|t| t.into()).collect();
                let mut partitioned_txns = partitioner.partition(analyzed_transactions);
                partitioned_txns.add_checkpoint_txn(last_txn);
                ExecutableBlock::new(block_id, ExecutableTransactions::Sharded(partitioned_txns))
            },
        };
        self.num_blocks_processed += 1;
        ExecuteBlockMessage {
            current_block_start_time,
            partition_time: Instant::now().duration_since(current_block_start_time),
            block,
        }
    }
}
