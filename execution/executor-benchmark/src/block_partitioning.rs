// Copyright © Aptos Foundation

use crate::pipeline::ExecuteBlockMessage;
use aptos_block_partitioner::sharded_block_partitioner::ShardedBlockPartitioner;
use aptos_crypto::HashValue;
use aptos_logger::info;
use aptos_types::{
    block_executor::partitioner::{
        CrossShardDependencies, ExecutableBlock, ExecutableTransactions,
        TransactionWithDependencies,
    },
    transaction::Transaction,
};
use std::time::Instant;

pub(crate) struct BlockPartitioningStage {
    num_blocks_processed: usize,
    maybe_partitioner: Option<ShardedBlockPartitioner>,
}

impl BlockPartitioningStage {
    pub fn new(num_shards: usize) -> Self {
        let maybe_partitioner = if num_shards <= 1 {
            None
        } else {
            let partitioner = ShardedBlockPartitioner::new(num_shards);
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
        let block: ExecutableBlock<Transaction> = match &self.maybe_partitioner {
            None => (block_id, txns).into(),
            Some(partitioner) => {
                let last_txn = txns.pop().unwrap();
                assert!(matches!(last_txn, Transaction::StateCheckpoint(_)));
                let analyzed_transactions = txns.into_iter().map(|t| t.into()).collect();
                let mut sub_blocks = partitioner.partition(analyzed_transactions, 4, 0.95);
                sub_blocks
                    .last_mut()
                    .unwrap()
                    .sub_blocks
                    .last_mut()
                    .unwrap()
                    .transactions
                    .push(TransactionWithDependencies::new(
                        last_txn,
                        CrossShardDependencies::default(),
                    ));
                ExecutableBlock::new(block_id, ExecutableTransactions::Sharded(sub_blocks))
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
