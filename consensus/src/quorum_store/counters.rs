// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{
    exponential_buckets, op_counters::DurationHistogram, register_histogram,
    register_histogram_vec, register_int_counter, register_int_counter_vec, AverageIntCounter,
    Histogram, HistogramVec, IntCounter, IntCounterVec,
};
use once_cell::sync::Lazy;
use std::time::Duration;

pub const GET_BATCH_LABEL: &str = "get_batch";
pub const GET_BLOCK_RESPONSE_LABEL: &str = "get_block_response";

pub const REQUEST_FAIL_LABEL: &str = "fail";
pub const REQUEST_SUCCESS_LABEL: &str = "success";

pub const CALLBACK_FAIL_LABEL: &str = "callback_fail";
pub const CALLBACK_SUCCESS_LABEL: &str = "callback_success";

static TRANSACTION_COUNT_BUCKETS: Lazy<Vec<f64>> = Lazy::new(|| {
    exponential_buckets(
        /*start=*/ 1.5, /*factor=*/ 1.5, /*count=*/ 20,
    )
    .unwrap()
});

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
pub static MAIN_LOOP: Lazy<DurationHistogram> = Lazy::new(|| {
    DurationHistogram::new(
        register_histogram!(
            "quorum_store_direct_mempool_main_loop",
            "Duration of the each run of the event loop"
        )
        .unwrap(),
    )
});

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
        // exponential_buckets(/*start=*/ 5.0, /*factor=*/ 1.1, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});

/// Histogram for the number of transactions per batch.
pub static NUM_TXN_PER_BATCH: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "quorum_store_num_txn_per_batch",
        "Histogram for the number of transanctions per batch.",
        // exponential_buckets(/*start=*/ 100.0, /*factor=*/ 1.1, /*count=*/ 100).unwrap(),
    )
    .unwrap()
});

/// Histogram for the number of fragments per batch.
pub static NUM_FRAGMENT_PER_BATCH: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "quorum_store_num_fragment_per_batch",
        "Histogram for the number of fragments per batch.",
        // exponential_buckets(/*start=*/ 100.0, /*factor=*/ 1.1, /*count=*/ 100).unwrap(),
    )
    .unwrap()
});

/// Histogram for the number of transactions per block when pulled for consensus.
pub static BLOCK_SIZE_WHEN_PULL: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "quorum_store_block_size_when_pull",
        "Histogram for the number of transactions per block when pulled for consensus.",
        // exponential_buckets(/*start=*/ 5.0, /*factor=*/ 1.1, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});

/// Histogram for the total size of transactions per block when pulled for consensus.
pub static BLOCK_BYTES_WHEN_PULL: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "quorum_store_block_bytes_when_pull",
        "Histogram for the total size of transactions per block when pulled for consensus.",
        // exponential_buckets(/*start=*/ 5.0, /*factor=*/ 1.1, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});

/// Histogram for the number of proof-of-store per block when pulled for consensus.
pub static PROOF_SIZE_WHEN_PULL: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "quorum_store_proof_size_when_pull",
        "Histogram for the number of proof-of-store per block when pulled for consensus.",
        // exponential_buckets(/*start=*/ 5.0, /*factor=*/ 1.1, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});

/// Histogram for the number of expired proof-of-store when pulled for consensus.
pub static EXPIRED_PROOFS_WHEN_PULL: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "quorum_store_expired_proof_size_when_pull",
        "Histogram for the number of expired proof-of-store when pulled for consensus.",
        // exponential_buckets(/*start=*/ 5.0, /*factor=*/ 1.1, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});

/// Histogram for the gaps between expiration round of the batch and the last certified round, and expiration round is higher.
pub static GAP_BETWEEN_BATCH_EXPIRATION_AND_LAST_CERTIFIED_ROUND_HIGHER: Lazy<Histogram> =
    Lazy::new(|| {
        register_histogram!(
        "quorum_store_gap_batch_expiration_and_last_certified_round_higher",
        "Histogram for the gaps between expiration round of the batch and the last certified round, and expiration round is higher.",
        // exponential_buckets(/*start=*/ 100.0, /*factor=*/ 1.1, /*count=*/ 100).unwrap(),
    )
    .unwrap()
    });

/// Histogram for the gaps between expiration round of the batch and the last certified round, and expiration round is lower.
pub static GAP_BETWEEN_BATCH_EXPIRATION_AND_LAST_CERTIFIED_ROUND_LOWER: Lazy<Histogram> = Lazy::new(
    || {
        register_histogram!(
        "quorum_store_gap_batch_expiration_and_last_certified_round_lower",
        "Histogram for the gaps between expiration round of the batch and the last certified round, and expiration round is lower.",
        // exponential_buckets(/*start=*/ 100.0, /*factor=*/ 1.1, /*count=*/ 100).unwrap(),
    )
    .unwrap()
    },
);

/// Histogram for the gaps between expiration round and the current round when pulling the proofs, and expiration round is lower.
pub static GAP_BETWEEN_BATCH_EXPIRATION_AND_CURRENT_ROUND_WHEN_PULL_PROOFS: Lazy<Histogram> =
    Lazy::new(|| {
        register_histogram!(
        "quorum_store_gap_batch_expiration_and_current_round_when_pull",
        "Histogram for the gaps between expiration round and the current round when pulling the proofs, and expiration round is lower.",
        // exponential_buckets(/*start=*/ 100.0, /*factor=*/ 1.1, /*count=*/ 100).unwrap(),
    )
    .unwrap()
    });

pub static POS_TO_PULL: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "quorum_store_pos_to_pull",
        "Histogram for how long it took a PoS to go from inserted to pulled into a proposed block",
        // exponential_buckets(/*start=*/ 100.0, /*factor=*/ 1.1, /*count=*/ 100).unwrap(),
    )
    .unwrap()
});

pub static POS_TO_COMMIT: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "quorum_store_pos_to_commit",
        "Histogram for how long it took a PoS to go from inserted to commit notified",
        // exponential_buckets(/*start=*/ 100.0, /*factor=*/ 1.1, /*count=*/ 100).unwrap(),
    )
    .unwrap()
});

/// Histogram for the number of total txns left after cleaning up commit notifications.
pub static NUM_TOTAL_TXNS_LEFT_ON_COMMIT: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "quorum_store_num_total_txns_left_on_commit",
        "Histogram for the number of total txns left after cleaning up commit notifications.",
        TRANSACTION_COUNT_BUCKETS.clone(),
    )
    .unwrap()
});

/// Histogram for the number of total batches/PoS left after cleaning up commit notifications.
pub static NUM_TOTAL_PROOFS_LEFT_ON_COMMIT: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "quorum_store_num_total_proofs_left_on_commit",
        "Histogram for the number of total batches/PoS left after cleaning up commit notifications.",
        TRANSACTION_COUNT_BUCKETS.clone(),
    )
        .unwrap()
});

/// Histogram for the number of local batches/PoS left after cleaning up commit notifications.
pub static NUM_LOCAL_PROOFS_LEFT_ON_COMMIT: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "quorum_store_num_local_proofs_left_on_commit",
        "Histogram for the number of locally created batches/PoS left after cleaning up commit notifications.",
        TRANSACTION_COUNT_BUCKETS.clone(),
    )
    .unwrap()
});

/// Counters

/// Count of how many times txns are pulled.
pub static PULLED_TXNS_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!("quorum_store_pulled_txn_count", "Count of the pulled txns.").unwrap()
});

/// Histogram for the number of txns are pulled.
pub static PULLED_TXNS_NUM: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "quorum_store_pulled_txns_num",
        "Histogram for the number of txns are pulled.",
        // exponential_buckets(/*start=*/ 5.0, /*factor=*/ 1.1, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});

/// Count of the pulled empty txns.
pub static PULLED_EMPTY_TXNS_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_pulled_empty_txn_count",
        "Count of the pulled empty txns."
    )
    .unwrap()
});

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
pub static LOCAL_POS_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_local_PoS_count",
        "Count of the locally created PoS since last restart."
    )
    .unwrap()
});

/// Count of the created proof-of-store (PoS) since last restart.
pub static REMOTE_POS_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_remote_PoS_count",
        "Count of the received PoS since last restart."
    )
    .unwrap()
});

/// Count of the delivered batches since last restart.
pub static DELIVERED_BATCHES_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_delivered_batch_count",
        "Count of the delivered batches since last restart."
    )
    .unwrap()
});

/// Count of the delivered fragments since last restart.
pub static DELIVERED_FRAGMENTS_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_delivered_fragments_count",
        "Count of the delivered fragments since last restart."
    )
    .unwrap()
});

/// Count of the delivered end batch since last restart.
pub static DELIVERED_END_BATCH_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_delivered_end_batch_count",
        "Count of the delivered end batch since last restart."
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

/// Count of the exceeded storage quota.
pub static EXCEEDED_STORAGE_QUOTA_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_exceeded_storage_quota_count",
        "Count of the exceeded storage quota."
    )
    .unwrap()
});

/// Count of the number of batch request sent to other nodes.
pub static GET_BATCH_FROM_DB_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_get_batch_from_db_count",
        "Count of the number of get batch request from QS DB."
    )
    .unwrap()
});

/// Count of the number of batch request sent to other nodes.
pub static SENT_BATCH_REQUEST_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_sent_batch_request_count",
        "Count of the number of batch request sent to other nodes."
    )
    .unwrap()
});

/// Count of the number of batch request retry sent to other nodes.
pub static SENT_BATCH_REQUEST_RETRY_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_sent_batch_request_retry_count",
        "Count of the number of batch request retry sent to other nodes."
    )
    .unwrap()
});

/// Counters(queued,dequeued,dropped) related to batch retrieval per epoch task
pub static BATCH_RETRIEVAL_TASK_MSGS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_quorum_store_batch_retrieval_task_msgs_count",
        "Counters(queued,dequeued,dropped) related to batch retrieval task",
        &["state"]
    )
    .unwrap()
});

/// Count of the number of batch request received from other nodes.
pub static RECEIVED_BATCH_REQUEST_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_received_batch_request_count",
        "Count of the number of batch request received from other nodes."
    )
    .unwrap()
});

/// Count of the number of batch request received from other nodes that is timeout.
pub static RECEIVED_BATCH_REQUEST_TIMEOUT_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_received_batch_request_timeout_count",
        "Count of the number of batch request received from other nodes that is timeout."
    )
    .unwrap()
});

/// Count of the number of batches received from other nodes.
pub static RECEIVED_BATCH_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_received_batch_count",
        "Count of the number of batches received from other nodes."
    )
    .unwrap()
});

pub static QS_BACKPRESSURE_TXN_COUNT: Lazy<AverageIntCounter> = Lazy::new(|| {
    AverageIntCounter::register(
        "quorum_store_backpressure_txn_count",
        "Indicator of whether Quorum Store is backpressured due to txn count exceeding threshold.",
    )
});

pub static QS_BACKPRESSURE_PROOF_COUNT: Lazy<AverageIntCounter> = Lazy::new(|| {
    AverageIntCounter::register(
        "quorum_store_backpressure_proof_count",
        "Indicator of whether Quorum Store is backpressured due to proof count exceeding threshold."
    )
});

pub static QS_BACKPRESSURE_DYNAMIC_MAX: Lazy<AverageIntCounter> = Lazy::new(|| {
    AverageIntCounter::register(
        "quorum_store_backpressure_dynamic_max",
        "What the dynamic max is set to",
    )
});

/// Latencies

/// Histogram of the time durations for batch creation.
pub static BATCH_CREATION_DURATION: Lazy<DurationHistogram> = Lazy::new(|| {
    DurationHistogram::new(
        register_histogram!(
            "quorum_store_batch_creation_duration",
            "Histogram of the time durations for batch creation.",
            // exponential_buckets(/*start=*/ 100.0, /*factor=*/ 1.1, /*count=*/ 100).unwrap(),
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
            // exponential_buckets(/*start=*/ 100.0, /*factor=*/ 1.1, /*count=*/ 100).unwrap(),
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
            // exponential_buckets(/*start=*/ 100.0, /*factor=*/ 1.1, /*count=*/ 100).unwrap(),
        )
        .unwrap(),
    )
});
