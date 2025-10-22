// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    metrics::{NUM_TXNS, TIMER},
    pipeline::ExecuteBlockMessage,
};
use aptos_block_partitioner::{BlockPartitioner, PartitionerConfig};
use aptos_crypto::HashValue;
use aptos_experimental_runtimes::thread_manager::optimal_min_len;
use aptos_logger::info;
use aptos_types::{
    block_executor::partitioner::{ExecutableBlock, ExecutableTransactions},
    transaction::{signature_verified_transaction::SignatureVerifiedTransaction, Transaction},
};
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use std::time::Instant;

/// Smallest number of transactions Rayon should put into a single worker task.
/// Same as in consensus/src/execution_pipeline.rs
pub const SIG_VERIFY_RAYON_MIN_THRESHOLD: usize = 32;

/// Executes preparation stage - set of operations that are
/// executed in a separate stage of the pipeline from execution,
/// like signature verificaiton or block partitioning
pub(crate) struct BlockPreparationStage {
    /// Number of blocks processed
    num_blocks_processed: usize,
    /// Pool of theads for signature verification
    sig_verify_pool: rayon::ThreadPool,
    /// When execution sharding is enabled, number of executor shards
    num_executor_shards: usize,
    /// When execution sharding is enabled, partitioner that splits block into shards
    maybe_partitioner: Option<Box<dyn BlockPartitioner>>,
}

impl BlockPreparationStage {
    pub fn new(
        num_sig_verify_threads: usize,
        num_shards: usize,
        partitioner_config: &dyn PartitionerConfig,
    ) -> Self {
        let maybe_partitioner = if num_shards == 0 {
            None
        } else {
            let partitioner = partitioner_config.build();
            Some(partitioner)
        };

        let sig_verify_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_sig_verify_threads)
            .thread_name(|index| format!("signature-checker-{}", index))
            .build()
            .expect("couldn't create sig_verify thread pool");
        Self {
            num_executor_shards: num_shards,
            num_blocks_processed: 0,
            maybe_partitioner,
            sig_verify_pool,
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
        let sig_verified_txns: Vec<SignatureVerifiedTransaction> =
            self.sig_verify_pool.install(|| {
                let _timer = TIMER.with_label_values(&["sig_verify"]).start_timer();

                let num_txns = txns.len();
                NUM_TXNS
                    .with_label_values(&["sig_verify"])
                    .inc_by(num_txns as u64);

                txns.into_par_iter()
                    .with_min_len(optimal_min_len(num_txns, SIG_VERIFY_RAYON_MIN_THRESHOLD))
                    .map(|t| t.into())
                    .collect::<Vec<_>>()
            });
        let block: ExecutableBlock = match &self.maybe_partitioner {
            None => (block_id, sig_verified_txns).into(),
            Some(partitioner) => {
                NUM_TXNS
                    .with_label_values(&["partition"])
                    .inc_by(sig_verified_txns.len() as u64);
                let analyzed_transactions =
                    sig_verified_txns.into_iter().map(|t| t.into()).collect();
                let timer = TIMER.with_label_values(&["partition"]).start_timer();
                let partitioned_txns =
                    partitioner.partition(analyzed_transactions, self.num_executor_shards);
                timer.stop_and_record();
                ExecutableBlock::new(
                    block_id,
                    ExecutableTransactions::Sharded(partitioned_txns),
                    vec![],
                )
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
