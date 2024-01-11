// Copyright © Aptos Foundation

use crate::{metrics::TIMER, pipeline::ExecuteBlockMessage};
use aptos_block_partitioner::{BlockPartitioner, PartitionerConfig};
use aptos_crypto::HashValue;
use aptos_experimental_runtimes::thread_manager::optimal_min_len;
use aptos_logger::info;
use aptos_types::{
    block_executor::partitioner::{ExecutableBlock, ExecutableTransactions},
    transaction::{signature_verified_transaction::SignatureVerifiedTransaction, Transaction},
};
use once_cell::sync::Lazy;
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use std::{sync::Arc, time::Instant};

pub static SIG_VERIFY_POOL: Lazy<Arc<rayon::ThreadPool>> = Lazy::new(|| {
    Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(8) // More than 8 threads doesn't seem to help much
            .thread_name(|index| format!("signature-checker-{}", index))
            .build()
            .unwrap(),
    )
});

pub(crate) struct BlockPreparationStage {
    num_executor_shards: usize,
    num_blocks_processed: usize,
    maybe_partitioner: Option<Box<dyn BlockPartitioner>>,
}

impl BlockPreparationStage {
    pub fn new(num_shards: usize, partitioner_config: &dyn PartitionerConfig) -> Self {
        let maybe_partitioner = if num_shards == 0 {
            None
        } else {
            let partitioner = partitioner_config.build();
            Some(partitioner)
        };

        Self {
            num_executor_shards: num_shards,
            num_blocks_processed: 0,
            maybe_partitioner,
        }
    }

    pub fn process(&mut self, txns: Vec<Transaction>) -> ExecuteBlockMessage {
        let current_block_start_time = Instant::now();
        info!(
            "In iteration {}, received {:?} transactions.",
            self.num_blocks_processed,
            txns.len()
        );
        let block_id = HashValue::random();
        let sig_verified_txns: Vec<SignatureVerifiedTransaction> = SIG_VERIFY_POOL.install(|| {
            let num_txns = txns.len();
            txns.into_par_iter()
                .with_min_len(optimal_min_len(num_txns, 32))
                .map(|t| t.into())
                .collect::<Vec<_>>()
        });
        let block: ExecutableBlock = match &self.maybe_partitioner {
            None => (block_id, sig_verified_txns).into(),
            Some(partitioner) => {
                let analyzed_transactions =
                    sig_verified_txns.into_iter().map(|t| t.into()).collect();
                let timer = TIMER.with_label_values(&["partition"]).start_timer();
                let partitioned_txns =
                    partitioner.partition(analyzed_transactions, self.num_executor_shards);
                timer.stop_and_record();
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
