// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{
    exponential_buckets, register_histogram, register_int_gauge_vec, Histogram, IntGaugeVec,
};
use once_cell::sync::Lazy;

pub static NUM_PARTITIONED_TXNS: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_num_partitioned_txns",
        "Number of transactions partitioned by shard and round",
        &["shard_id", "round_id"]
    )
    .unwrap()
});

pub static BLOCK_PARTITIONING_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_block_partitioning_seconds",
        // metric description
        "The total time spent in seconds of block partitioning in the sharded block partitioner.",
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});
