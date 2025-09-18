// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    metrics::{NUM_TXNS, TIMER},
    pipeline::ExecuteBlockMessage,
};
use aptos_storage_interface::DbReader;
use aptos_block_partitioner::{BlockPartitioner, PartitionerConfig};
use std::sync::Arc;
use aptos_config::keys::ConfigKey;
use aptos_crypto::{ed25519::Ed25519PrivateKey, HashValue, Uniform};
use aptos_experimental_runtimes::thread_manager::optimal_min_len;
use aptos_logger::info;
use aptos_metrics_core::{IntCounterVecHelper, TimerHelper};
use aptos_types::{
    account_address::AccountAddress,
    block_executor::partitioner::{ExecutableBlock, ExecutableTransactions},
    block_metadata::BlockMetadata,
    transaction::{
        authenticator::AuthenticationKey,
        signature_verified_transaction::SignatureVerifiedTransaction, Transaction,
    },
};
use rand::{rngs::StdRng, Rng, SeedableRng};
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use std::time::Instant;

/// Smallest number of transactions Rayon should put into a single worker task.
/// Same as in consensus/src/execution_pipeline.rs
pub const SIG_VERIFY_RAYON_MIN_THRESHOLD: usize = 32;

fn validator_address() -> AccountAddress {
    let mut rng = StdRng::from_seed([0; 32]);
    let _: [u8; 32] = rng.gen();
    let seed: [u8; 32] = rng.gen();
    let key = Ed25519PrivateKey::generate(&mut StdRng::from_seed(seed));
    AuthenticationKey::ed25519(&ConfigKey::new(key).public_key()).account_address()
}

pub(crate) fn create_block_metadata_transaction_epoch_0() -> Transaction {
    // Use incremental timestamps to avoid triggering epoch reconfigurations
    // Large real timestamps cause immediate epoch changes since last_reconfiguration_time is small
    use std::sync::atomic::{AtomicU64, Ordering};
    static ROUND_COUNTER: AtomicU64 = AtomicU64::new(0);
    static LAST_TIMESTAMP: AtomicU64 = AtomicU64::new(0);

    ROUND_COUNTER.fetch_add(1, Ordering::SeqCst);
    
    // Get current real time to keep blockchain time close to real time for orderless transactions
    let current_time_usecs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64;
    
    // Ensure strictly increasing timestamps by comparing with last used timestamp
    let last_timestamp = LAST_TIMESTAMP.load(Ordering::SeqCst);
    let timestamp_usecs = if current_time_usecs > last_timestamp {
        current_time_usecs
    } else {
        // If current time is not greater, increment by 1 microsecond to maintain strict ordering
        last_timestamp + 1
    };
    
    // Update the last timestamp atomically
    LAST_TIMESTAMP.store(timestamp_usecs, Ordering::SeqCst);
    
    Transaction::BlockMetadata(BlockMetadata::new(
        HashValue::random(),
        0,                     // epoch stays 0 for benchmark
        round,                 // proper incrementing round number
        validator_address(),   // keep existing validator address
        vec![],
        vec![],
        timestamp_usecs,       // real time with strict ordering guarantee
    ))
}

pub(crate) fn create_block_metadata_transaction_epoch_1() -> Transaction {
    // Use incremental timestamps to avoid triggering epoch reconfigurations
    // Large real timestamps cause immediate epoch changes since last_reconfiguration_time is small
    use std::sync::atomic::{AtomicU64, Ordering};
    static ROUND_COUNTER: AtomicU64 = AtomicU64::new(0);
    static LAST_TIMESTAMP: AtomicU64 = AtomicU64::new(0);


    let round = ROUND_COUNTER.fetch_add(1, Ordering::SeqCst);

    // Get current real time to keep blockchain time close to real time for orderless transactions
    let current_time_usecs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64;
    
    // Ensure strictly increasing timestamps by comparing with last used timestamp
    let last_timestamp = LAST_TIMESTAMP.load(Ordering::SeqCst);
    let timestamp_usecs = if current_time_usecs > last_timestamp {
        current_time_usecs
    } else {
        // If current time is not greater, increment by 1 microsecond to maintain strict ordering
        last_timestamp + 1
    };
    
    // Update the last timestamp atomically
    LAST_TIMESTAMP.store(timestamp_usecs, Ordering::SeqCst);
    
    Transaction::BlockMetadata(BlockMetadata::new(
        HashValue::random(),
        1,                     // epoch stays 1 for benchmark
        round,                 // proper incrementing round number
        validator_address(),   // keep existing validator address
        vec![],
        vec![],
        timestamp_usecs,       // real time with strict ordering guarantee
    ))
}

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

        info!(
            "Added BlockMetadata transaction to block {}, total transactions: {} + 1 = {}",
            self.num_blocks_processed,
            txns.len(),
            txns.len() + 1
        );

        let sig_verified_txns: Vec<SignatureVerifiedTransaction> =
            self.sig_verify_pool.install(|| {
                let _timer = TIMER.timer_with(&["sig_verify"]);

                let num_txns = txns.len();
                NUM_TXNS.inc_with_by(&["sig_verify"], num_txns as u64);

                txns
                    .into_par_iter()
                    .with_min_len(optimal_min_len(num_txns, SIG_VERIFY_RAYON_MIN_THRESHOLD))
                    .map(|t| t.into())
                    .collect::<Vec<_>>()
            });
        let block: ExecutableBlock = match &self.maybe_partitioner {
            None => (block_id, sig_verified_txns).into(),
            Some(partitioner) => {
                NUM_TXNS.inc_with_by(&["partition"], sig_verified_txns.len() as u64);
                let analyzed_transactions =
                    sig_verified_txns.into_iter().map(|t| t.into()).collect();
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
