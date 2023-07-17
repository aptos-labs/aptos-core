// Copyright © Aptos Foundation

use crate::pipeline::ExecuteBlockMessage;
use aptos_block_partitioner::{BlockPartitioner, APTOS_BLOCK_PARTITIONER_SECONDS, report_sub_block_matrix};
use aptos_crypto::HashValue;
use aptos_logger::info;
use aptos_types::{
    block_executor::partitioner::{
        CrossShardDependencies, ExecutableBlock, ExecutableTransactions,
        TransactionWithDependencies,
    },
    transaction::Transaction,
};
use std::{sync::Arc, time::Instant};
use aptos_types::transaction::analyzed_transaction::AnalyzedTransaction;

pub(crate) struct BlockPartitioningStage {
    num_executor_shards: usize,
    num_blocks_processed: usize,
    partitioner: Arc<dyn BlockPartitioner>,
}

impl BlockPartitioningStage {
    pub fn new(num_executor_shards: usize, partitioner: Arc<dyn BlockPartitioner>) -> Self {
        Self {
            num_executor_shards,
            num_blocks_processed: 0,
            partitioner,
        }
    }

    pub fn process(&mut self, mut txns: Vec<AnalyzedTransaction>) -> ExecuteBlockMessage {
        let current_block_start_time = Instant::now();
        info!(
            "In iteration {}, received {:?} transactions.",
            self.num_blocks_processed,
            txns.len()
        );
        let block_id = HashValue::random();
        let block: ExecutableBlock<Transaction> = {
            let timer = APTOS_BLOCK_PARTITIONER_SECONDS.start_timer();
            let last_txn = txns.pop().unwrap();
            assert!(matches!(last_txn.transaction(), &Transaction::StateCheckpoint(_)));
            let mut sub_blocks = self.partitioner.partition(txns, self.num_executor_shards);
            timer.stop_and_record();
            report_sub_block_matrix(&sub_blocks);
            sub_blocks
                .last_mut()
                .unwrap()
                .sub_blocks
                .last_mut()
                .unwrap()
                .transactions
                .push(TransactionWithDependencies::new(
                    last_txn.into_txn(),
                    CrossShardDependencies::default(),
                ));
            ExecutableBlock::new(block_id, ExecutableTransactions::Sharded(sub_blocks))
        };
        self.num_blocks_processed += 1;
        ExecuteBlockMessage {
            current_block_start_time,
            partition_time: Instant::now().duration_since(current_block_start_time),
            block,
        }
    }
}
