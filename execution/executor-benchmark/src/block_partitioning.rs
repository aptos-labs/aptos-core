// Copyright © Aptos Foundation

use crate::{metrics::TIMER, pipeline::ExecuteBlockMessage};
use aptos_block_partitioner::{build_partitioner_from_envvar, BlockPartitioner};
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
    num_executor_shards: usize,
    num_blocks_processed: usize,
    maybe_partitioner: Option<Box<dyn BlockPartitioner>>,
}

impl BlockPartitioningStage {
    pub fn new(num_executor_shards: usize) -> Self {
        let maybe_partitioner = if num_executor_shards <= 1 {
            None
        } else {
            Some(build_partitioner_from_envvar(Some(num_executor_shards)))
        };

        Self {
            num_executor_shards,
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
                assert!(matches!(last_txn, Transaction::StateCheckpoint(_)));
                let analyzed_transactions = txns.into_iter().map(|t| t.into()).collect();
                let timer = TIMER.with_label_values(&["partition"]).start_timer();
                let mut sub_blocks =
                    partitioner.partition(analyzed_transactions, self.num_executor_shards);
                timer.stop_and_record();
                sub_blocks
                    .last_mut()
                    .unwrap()
                    .sub_blocks
                    .last_mut()
                    .unwrap()
                    .transactions
                    .push(TransactionWithDependencies::new(
                        last_txn.into(),
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
