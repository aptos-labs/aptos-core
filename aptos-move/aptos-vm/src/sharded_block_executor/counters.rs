// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{register_histogram_vec, register_int_gauge, HistogramVec, IntGauge};
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
