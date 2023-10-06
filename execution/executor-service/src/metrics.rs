// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{exponential_buckets, register_histogram, Histogram, IntCounterVec, register_int_counter_vec, IntCounter, register_int_counter};
use once_cell::sync::Lazy;

pub static APTOS_REMOTE_EXECUTOR_CMD_RX_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_remote_executor_cmd_rx_seconds",
        // metric description
        "The time spent in seconds on receiving rx_command on a shard in Aptos executor",
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    ).unwrap()
});

pub static APTOS_REMOTE_EXECUTOR_CMD_RX_BCS_DESERIALIZE_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_remote_executor_cmd_rx_bcs_deserialize_seconds",
        // metric description
        "The time spent in seconds on deserializing the received rx_command on a shard in Aptos executor",
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    ).unwrap()
});

pub static APTOS_REMOTE_EXECUTOR_INIT_PREFETCH_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_remote_executor_init_prefetch_seconds",
        // metric description
        "The time spent in seconds on initializing the prefetching of remote state values on a shard in Aptos executor",
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    ).unwrap()
});

pub static APTOS_REMOTE_EXECUTOR_REMOTE_KV_RESPONSES_PROCESSING_TIME_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_remote_executor_remote_kv_response_processing_time_seconds",
        // metric description
        "The time spent in seconds on processing the remote key value responses on a shard",
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    ).unwrap()
});

pub static APTOS_REMOTE_EXECUTOR_REMOTE_KV_RESPONSES_DESER_TIME_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_remote_executor_remote_kv_response_deser_time_seconds",
        // metric description
        "The time spent in seconds on deserializing the remote key value responses on a shard",
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    ).unwrap()
});

pub static APTOS_REMOTE_EXECUTOR_PREFETCH_WAIT_TIME_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_remote_executor_prefetch_wait_time_seconds",
        // metric description
        "Approx time spent in seconds on waiting for the remote state values to be prefetched",
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    ).unwrap()
});

pub static APTOS_REMOTE_EXECUTOR_NON_PREFETCH_WAIT_TIME_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_remote_executor_non_prefetch_wait_time_seconds",
        // metric description
        "Time spent in seconds on waiting for the remote state values that were not prefetched",
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    ).unwrap()
});

pub static APTOS_REMOTE_EXECUTOR_REMOTE_KV_RESPONSES_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        // metric name
        "aptos_remote_executor_remote_kv_responses_count",
        // metric description
        "The number of remote key value responses received on a shard",
    ).unwrap()
});

pub static APTOS_REMOTE_EXECUTOR_NON_PREFETCH_KV_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        // metric name
        "aptos_remote_executor_non_prefetch_kv_count",
        // metric description
        "The number of remote key value responses received on a shard that were not prefetched",
    ).unwrap()
});
