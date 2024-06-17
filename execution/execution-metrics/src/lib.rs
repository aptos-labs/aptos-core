// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{
    exponential_buckets, register_histogram_vec, register_int_counter_vec, HistogramVec,
    IntCounterVec,
};
use once_cell::sync::Lazy;

pub static REMOTE_EXECUTOR_TIMER_V2: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        // metric name
        "remote_executor_timer_v2",
        // metric description
        "The time spent in remote shard on: \
         1. get_txn_avg_waiting_time; Time spent on waiting on transaction to be loaded from local store; ",
        // metric labels (dimensions)
        &["shard_id", "name"],
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
        .unwrap()
});
