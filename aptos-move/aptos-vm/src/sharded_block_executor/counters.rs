// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{
    exponential_buckets, make_local_histogram_vec, register_histogram, register_int_gauge,
    Histogram, HistogramVec, IntGauge,
};
use once_cell::sync::Lazy;

pub static NUM_EXECUTOR_SHARDS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "num_executor_shards",
        "Number of shards for the sharded block executor"
    )
    .unwrap()
});

pub static SHARDED_BLOCK_EXECUTION_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "sharded_block_execution_seconds",
        "Time to execute a block in sharded execution in seconds",
    )
    .unwrap()
});

pub static SHARDED_EXECUTION_RESULT_AGGREGATION_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "sharded_execution_result_aggregation_seconds",
        "Time to aggregate the results of sharded execution in seconds",
    )
    .unwrap()
});

pub static WAIT_FOR_SHARDED_OUTPUT_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "wait_for_sharded_output_seconds",
        "Time to wait for sharded output in seconds",
    )
    .unwrap()
});

make_local_histogram_vec!(
    pub,
    SHARDED_BLOCK_EXECUTION_BY_ROUNDS_SECONDS,
    "sharded_block_execution_by_rounds_seconds",
    "Time to execute a sub block in sharded execution in seconds",
    &["shard_id", "round_id"]
);

// Count of the committed transactions since last restart.
make_local_histogram_vec!(
    pub,
    SHARDED_BLOCK_EXECUTOR_TXN_COUNT,
    "sharded_block_executor_txn_count",
    "Count of number of transactions per shard per round in sharded execution",
    &["shard_id", "round_id"]
);

make_local_histogram_vec!(
    pub,
    SHARDED_EXECUTOR_SERVICE_SECONDS,
    // metric name
    "sharded_executor_execute_block_seconds",
    // metric description
    "Time spent in seconds on executing a block on a shard including: \
         1. execute_block: fetching state values and cross-shard communications; \
         2. result_tx: TX of results to coordinator.",
    &["shard_id", "name"],
    exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
);
