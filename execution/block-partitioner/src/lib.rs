// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

pub mod no_op;
pub mod sharded_block_partitioner;
pub mod simple_partitioner;

pub mod test_utils;

use aptos_metrics_core::{exponential_buckets, register_histogram, Histogram};
use aptos_types::{block_executor::partitioner::SubBlocksForShard, transaction::Transaction};
use once_cell::sync::Lazy;
use aptos_types::transaction::analyzed_transaction::AnalyzedTransaction;

pub trait BlockPartitioner: Send {
    fn partition(
        &self,
        transactions: Vec<Transaction>,
        num_executor_shards: usize,
    ) -> Vec<SubBlocksForShard<Transaction>>;
}

/// An implementation of partitioner that splits the transactions into equal-sized chunks.
pub struct UniformPartitioner {}

pub static APTOS_BLOCK_PARTITIONER_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_block_partitioner_seconds",
        // metric description
        "The total time spent in seconds of block partitioning in the sharded block partitioner.",
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});

pub static APTOS_BLOCK_ANALYZER_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_block_analyzer_seconds",
        // metric description
        "The total time spent in seconds of block analyzing.",
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
        .unwrap()
});

pub fn analyze_block(txns: Vec<Transaction>) -> Vec<AnalyzedTransaction> {
    let _timer = APTOS_BLOCK_ANALYZER_SECONDS.start_timer();
    txns.into_iter().map(|t| t.into()).collect()
}
