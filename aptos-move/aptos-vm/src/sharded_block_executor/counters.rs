// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{
    register_histogram_vec, register_int_counter_vec, register_int_gauge, HistogramVec,
    IntCounterVec, IntGauge,
};
use once_cell::sync::Lazy;

pub static NUM_EXECUTOR_SHARDS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "num_executor_shards",
        "Number of shards for the sharded block executor"
    )
    .unwrap()
});

pub static SHARDED_BLOCK_EXECUTION_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "sharded_block_execution_seconds",
        "Time to execute a sub block in sharded execution in seconds",
        &["shard_id", "round_id"]
    )
    .unwrap()
});

/// Count of the committed transactions since last restart.
pub static SHARDED_BLOCK_EXECUTOR_TXN_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "sharded_block_executor_txn_count",
        "Count of number of transactions per shard per round in sharded execution",
        &["shard_id", "round_id"]
    )
    .unwrap()
});

pub static CROSS_SHARD_STATE_VALUE_WAIT_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "cross_shard_state_value_wait_seconds",
        "Time to wait for a state value from another shard in seconds",
        &["shard_id", "round_id"]
    )
    .unwrap()
});

pub static CROSS_SHARD_STATE_VALUE_GET_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "cross_shard_state_value_get_seconds",
        "Time to wait for a state value from another shard in seconds",
        &["shard_id", "round_id"]
    )
    .unwrap()
});
