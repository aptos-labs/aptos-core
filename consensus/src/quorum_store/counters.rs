// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{
    exponential_buckets, op_counters::DurationHistogram, register_histogram,
    register_histogram_vec, register_int_counter, Histogram, HistogramVec, IntCounter,
};
use once_cell::sync::Lazy;
use std::time::Duration;

pub const GET_BATCH_LABEL: &str = "get_batch";
pub const GET_BLOCK_RESPONSE_LABEL: &str = "get_block_response";

pub const REQUEST_FAIL_LABEL: &str = "fail";
pub const REQUEST_SUCCESS_LABEL: &str = "success";

pub const CALLBACK_FAIL_LABEL: &str = "callback_fail";
pub const CALLBACK_SUCCESS_LABEL: &str = "callback_success";

/// Counter for tracking latency of quorum store processing requests from consensus
/// A 'fail' result means the quorum store's callback response to consensus failed.
static QUORUM_STORE_SERVICE_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "quorum_store_service_latency_ms",
        "Latency of quorum store processing request from consensus/state sync",
        &["type", "result"]
    )
    .unwrap()
});

pub fn quorum_store_service_latency(label: &'static str, result: &str, duration: Duration) {
    QUORUM_STORE_SERVICE_LATENCY
        .with_label_values(&[label, result])
        .observe(duration.as_secs_f64());
}

/// Duration of each run of the event loop.
pub static WRAPPER_MAIN_LOOP: Lazy<DurationHistogram> = Lazy::new(|| {
    DurationHistogram::new(
        register_histogram!(
            "quorum_store_wrapper_main_loop",
            "Duration of the each run of the event loop"
        )
        .unwrap(),
    )
});

//////////////////////
// NEW QUORUM STORE
//////////////////////

/// Histograms

/// Histogram for the number of batches per (committed) blocks.
pub static NUM_BATCH_PER_BLOCK: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "quorum_store_num_batch_per_block",
        "Histogram for the number of batches per (committed) blocks.",
        exponential_buckets(/*start=*/ 5.0, /*factor=*/ 1.1, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});

/// Histogram for the number of transactions per batch.
pub static NUM_TXN_PER_BATCH: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "quorum_store_num_txn_per_batch",
        "Histogram for the number of transanctions per batch.",
        exponential_buckets(/*start=*/ 100.0, /*factor=*/ 1.1, /*count=*/ 100).unwrap(),
    )
    .unwrap()
});

/// Counters

/// Count of the created batches since last restart.
pub static CREATED_BATCHES_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_created_batch_count",
        "Count of the created batches since last restart."
    )
    .unwrap()
});

/// Count of the created empty batches since last restart.
pub static CREATED_EMPTY_BATCHES_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_created_empty_batch_count",
        "Count of the created empty batches since last restart."
    )
    .unwrap()
});

/// Count of the created proof-of-store (PoS) since last restart.
pub static POS_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_PoS_count",
        "Count of the created PoS since last restart."
    )
    .unwrap()
});

/// Count of the created batches since last restart.
pub static DELIVERED_BATCHES_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_delivered_batch_count",
        "Count of the delivered batches since last restart."
    )
    .unwrap()
});

/// Count of the missed batches when execute.
pub static MISSED_BATCHES_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_missed_batch_count",
        "Count of the missed batches when execute."
    )
    .unwrap()
});

/// Count of the timeout batches at the sender side.
pub static TIMEOUT_BATCHES_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_timeout_batch_count",
        "Count of the timeout batches at the sender side."
    )
    .unwrap()
});

/// Count of the expired batch fragments at the receiver side.
pub static EXPIRED_BATCH_FRAGMENTS_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_expired_batch_fragments_count",
        "Count of the expired batch fragments at the receiver side."
    )
    .unwrap()
});

/// Count of the missed batch fragments at the receiver side.
pub static MISSED_BATCH_FRAGMENTS_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_missed_batch_fragments_count",
        "Count of the missed batch fragments at the receiver side."
    )
    .unwrap()
});

/// Latencies

/// Histogram of the time durations for batch creation.
pub static BATCH_CREATION_DURATION: Lazy<DurationHistogram> = Lazy::new(|| {
    DurationHistogram::new(
        register_histogram!(
            "quorum_store_batch_creation_duration",
            "Histogram of the time durations for batch creation.",
            exponential_buckets(/*start=*/ 100.0, /*factor=*/ 1.1, /*count=*/ 100).unwrap(),
        )
        .unwrap(),
    )
});

/// Histogram of the time durations for empty batch creation.
pub static EMPTY_BATCH_CREATION_DURATION: Lazy<DurationHistogram> = Lazy::new(|| {
    DurationHistogram::new(
        register_histogram!(
            "quorum_store_empty_batch_creation_duration",
            "Histogram of the time durations for empty batch creation.",
            exponential_buckets(/*start=*/ 100.0, /*factor=*/ 1.1, /*count=*/ 100).unwrap(),
        )
        .unwrap(),
    )
});

/// Histogram of the time durations from created batch to created PoS.
pub static BATCH_TO_POS_DURATION: Lazy<DurationHistogram> = Lazy::new(|| {
    DurationHistogram::new(
        register_histogram!(
            "quorum_store_batch_to_PoS_duration",
            "Histogram of the time durations from batch creation to PoS creation.",
            exponential_buckets(/*start=*/ 100.0, /*factor=*/ 1.1, /*count=*/ 100).unwrap(),
        )
        .unwrap(),
    )
});
