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
use aptos_metrics_core::{IntCounterVecHelper, TimerHelper};
use aptos_types::{
    block_executor::partitioner::{ExecutableBlock, ExecutableTransactions},
    block_metadata::BlockMetadata,
    transaction::{signature_verified_transaction::SignatureVerifiedTransaction, Transaction},
};
use move_core_types::account_address::AccountAddress;
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

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
    /// Current epoch for BlockMetadata
    epoch: u64,
    /// Proposer address for BlockMetadata (using a default for benchmark)
    proposer: AccountAddress,
}

impl BlockPreparationStage {
    pub fn new(
        num_sig_verify_threads: usize,
        num_shards: usize,
        partitioner_config: &dyn PartitionerConfig,
        epoch: Option<u64>,
        proposer: Option<AccountAddress>,
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
            epoch: epoch.unwrap_or(0), // Default epoch 0
            proposer: proposer.unwrap_or_else(|| crate::get_benchmark_proposer_address()), // Default proposer with initialized stake pool
        }
    }

    pub fn process(&mut self, txns: Vec<Transaction>) -> ExecuteBlockMessage {
        let current_block_start_time = Instant::now();
        info!(
            "In iteration {}, received {:?} transactions (+ BlockMetadata).",
            self.num_blocks_processed,
            txns.len()
        );
        let block_id = HashValue::random();

        // Create BlockMetadata transaction for this block
        let timestamp_usecs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_micros() as u64;

        let block_metadata = BlockMetadata::new(
            block_id,
            self.epoch,
            self.num_blocks_processed as u64, // Use block number as round
            self.proposer,
            vec![], // Empty previous_block_votes_bitvec for benchmark
            vec![], // Empty failed_proposer_indices for benchmark
            timestamp_usecs,
        );

        let block_metadata_txn = Transaction::BlockMetadata(block_metadata);
        let block_metadata_sig_verified = SignatureVerifiedTransaction::from(block_metadata_txn);

        // Process user transactions through signature verification
        let user_sig_verified_txns: Vec<SignatureVerifiedTransaction> =
            self.sig_verify_pool.install(|| {
                let _timer = TIMER.timer_with(&["sig_verify"]);

                let num_txns = txns.len();
                NUM_TXNS.inc_with_by(&["sig_verify"], num_txns as u64);

                txns.into_par_iter()
                    .with_min_len(optimal_min_len(num_txns, SIG_VERIFY_RAYON_MIN_THRESHOLD))
                    .map(|t| t.into())
                    .collect::<Vec<_>>()
            });

        // Combine BlockMetadata + user transactions (BlockMetadata first)
        let mut all_sig_verified_txns = vec![block_metadata_sig_verified];
        all_sig_verified_txns.extend(user_sig_verified_txns);

        let block: ExecutableBlock = match &self.maybe_partitioner {
            None => (block_id, all_sig_verified_txns).into(),
            Some(partitioner) => {
                NUM_TXNS.inc_with_by(&["partition"], all_sig_verified_txns.len() as u64);
                let analyzed_transactions =
                    all_sig_verified_txns.into_iter().map(|t| t.into()).collect();
                let timer = TIMER.timer_with(&["partition"]);
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
