// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{
    exponential_buckets, op_counters::DurationHistogram, register_avg_counter, register_histogram,
    register_histogram_vec, register_int_counter, register_int_counter_vec, Histogram,
    HistogramVec, IntCounter, IntCounterVec,
};
use once_cell::sync::Lazy;
use std::time::Duration;

pub const GET_BATCH_LABEL: &str = "get_batch";
pub const GET_BLOCK_RESPONSE_LABEL: &str = "get_block_response";

pub const REQUEST_FAIL_LABEL: &str = "fail";
pub const REQUEST_SUCCESS_LABEL: &str = "success";

pub const CALLBACK_FAIL_LABEL: &str = "callback_fail";
pub const CALLBACK_SUCCESS_LABEL: &str = "callback_success";

pub const POS_EXPIRED_LABEL: &str = "expired";
pub const POS_DUPLICATE_LABEL: &str = "duplicate";

static TRANSACTION_COUNT_BUCKETS: Lazy<Vec<f64>> = Lazy::new(|| {
    exponential_buckets(
        /*start=*/ 1.5, /*factor=*/ 1.5, /*count=*/ 25,
    )
    .unwrap()
});

static BYTE_BUCKETS: Lazy<Vec<f64>> = Lazy::new(|| {
    exponential_buckets(
        /*start=*/ 500.0, /*factor=*/ 1.5, /*count=*/ 25,
    )
    .unwrap()
});

// Histogram buckets that expand DEFAULT_BUCKETS with more granularity between 100-2000 ms
const QUORUM_STORE_LATENCY_BUCKETS: &[f64] = &[
    0.005, 0.01, 0.025, 0.05, 0.1, 0.15, 0.2, 0.25, 0.3, 0.35, 0.4, 0.45, 0.5, 0.55, 0.65, 0.7,
    0.75, 1.0, 1.25, 1.5, 2.0, 2.5, 5.0, 10.0,
];

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
pub static PROOF_MANAGER_MAIN_LOOP: Lazy<DurationHistogram> = Lazy::new(|| {
    DurationHistogram::new(
        register_histogram!(
            "quorum_store_proof_manager_main_loop",
            "Duration of the each run of the proof manager event loop"
        )
        .unwrap(),
    )
});

/// Duration of each run of the event loop.
pub static BATCH_GENERATOR_MAIN_LOOP: Lazy<DurationHistogram> = Lazy::new(|| {
    DurationHistogram::new(
        register_histogram!(
            "quorum_store_batch_generator_main_loop",
            "Duration of the each run of the batch generator event loop"
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
        TRANSACTION_COUNT_BUCKETS.clone(),
    )
    .unwrap()
});

/// Histogram for the number of transactions per batch.
static NUM_TXN_PER_BATCH: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "quorum_store_num_txn_per_batch",
        "Histogram for the number of transanctions per batch.",
        &["bucket"],
        TRANSACTION_COUNT_BUCKETS.clone(),
    )
    .unwrap()
});

pub fn num_txn_per_batch(bucket_start: &str, num: usize) {
    NUM_TXN_PER_BATCH
        .with_label_values(&[bucket_start])
        .observe(num as f64)
}

/// Histogram for the number of transactions per block when pulled for consensus.
pub static BLOCK_SIZE_WHEN_PULL: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "quorum_store_block_size_when_pull",
        "Histogram for the number of transactions per block when pulled for consensus.",
        TRANSACTION_COUNT_BUCKETS.clone(),
    )
    .unwrap()
});

/// Histogram for the total size of transactions per block when pulled for consensus.
pub static BLOCK_BYTES_WHEN_PULL: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "quorum_store_block_bytes_when_pull",
        "Histogram for the total size of transactions per block when pulled for consensus.",
        BYTE_BUCKETS.clone(),
    )
    .unwrap()
});

/// Histogram for the number of proof-of-store per block when pulled for consensus.
pub static PROOF_SIZE_WHEN_PULL: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "quorum_store_proof_size_when_pull",
        "Histogram for the number of proof-of-store per block when pulled for consensus.",
        TRANSACTION_COUNT_BUCKETS.clone(),
    )
    .unwrap()
});

pub static EXCLUDED_TXNS_WHEN_PULL: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "quorum_store_excluded_txns_when_pull",
        "Histogram for the number of transactions were considered but excluded when pulled for consensus.",
        TRANSACTION_COUNT_BUCKETS.clone(),
    )
        .unwrap()
});

pub static GAP_BETWEEN_BATCH_EXPIRATION_AND_CURRENT_TIME_WHEN_SAVE: Lazy<Histogram> = Lazy::new(
    || {
        register_histogram!(
        "quorum_store_gap_batch_expiration_and_current_time_when_save",
        "Histogram for the gaps between expiration round and the current round when saving proofs, and expiration time is lower.",
        QUORUM_STORE_LATENCY_BUCKETS.to_vec()
    )
    .unwrap()
    },
);

pub static NUM_BATCH_EXPIRED_WHEN_SAVE: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_num_batch_expired_when_save",
        "Number of batches that were already expired when save is called"
    )
    .unwrap()
});

/// Histogram for the gaps between expiration time and the current block timestamp on commit, and expiration round is lower.
pub static GAP_BETWEEN_BATCH_EXPIRATION_AND_CURRENT_TIME_WHEN_COMMIT: Lazy<Histogram> = Lazy::new(
    || {
        register_histogram!(
        "quorum_store_gap_batch_expiration_and_current_time_when_commit",
        "Histogram for the gaps between expiration time and the current block timestamp on commit, and expiration round is lower.",
        QUORUM_STORE_LATENCY_BUCKETS.to_vec()
    )
            .unwrap()
    },
);

pub static NUM_PROOFS_EXPIRED_WHEN_COMMIT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_num_proofs_expired_when_commit",
        "Number of proofs that were expired when commit is called"
    )
    .unwrap()
});

static POS_TO_PULL: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "quorum_store_pos_to_pull",
        "Histogram for how long it took a PoS to go from inserted to pulled into a proposed block",
        &["bucket"],
        QUORUM_STORE_LATENCY_BUCKETS.to_vec()
    )
    .unwrap()
});

pub fn pos_to_pull(bucket: u64, secs: f64) {
    POS_TO_PULL
        .with_label_values(&[bucket.to_string().as_str()])
        .observe(secs)
}

static POS_TO_COMMIT: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "quorum_store_pos_to_commit",
        "Histogram for how long it took a PoS to go from inserted to commit notified",
        &["bucket"],
        QUORUM_STORE_LATENCY_BUCKETS.to_vec()
    )
    .unwrap()
});

pub fn pos_to_commit(bucket: u64, secs: f64) {
    POS_TO_COMMIT
        .with_label_values(&[bucket.to_string().as_str()])
        .observe(secs);
}

/// Histogram for the number of total txns left after adding or cleaning batches.
pub static NUM_TOTAL_TXNS_LEFT_ON_UPDATE: Lazy<Histogram> = Lazy::new(|| {
    register_avg_counter(
        "quorum_store_num_total_txns_left_on_update",
        "Histogram for the number of total txns left after adding or cleaning batches.",
    )
});

/// Histogram for the number of total batches/PoS left after adding or cleaning batches.
pub static NUM_TOTAL_PROOFS_LEFT_ON_UPDATE: Lazy<Histogram> = Lazy::new(|| {
    register_avg_counter(
        "quorum_store_num_total_proofs_left_on_update",
        "Histogram for the number of total batches/PoS left after adding or cleaning batches.",
    )
});

/// Histogram for the number of local txns left after adding or cleaning batches.
pub static NUM_LOCAL_TXNS_LEFT_ON_UPDATE: Lazy<Histogram> = Lazy::new(|| {
    register_avg_counter(
        "quorum_store_num_local_txns_left_on_update",
        "Histogram for the number of locally created txns left after adding or cleaning batches.",
    )
});

/// Histogram for the number of local batches/PoS left after adding or cleaning batches.
pub static NUM_LOCAL_PROOFS_LEFT_ON_UPDATE: Lazy<Histogram> = Lazy::new(|| {
    register_avg_counter(
        "quorum_store_num_local_proofs_left_on_update",
        "Histogram for the number of locally created batches/PoS left after adding or cleaning batches.",
    )
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
        TRANSACTION_COUNT_BUCKETS.clone()
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
static LOCAL_POS_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "quorum_store_local_PoS_count",
        "Count of the locally created PoS since last restart.",
        &["bucket"]
    )
    .unwrap()
});

pub fn inc_local_pos_count(bucket: u64) {
    LOCAL_POS_COUNT
        .with_label_values(&[bucket.to_string().as_str()])
        .inc()
}

/// Count of the created proof-of-store (PoS) since last restart.
static REMOTE_POS_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "quorum_store_remote_PoS_count",
        "Count of the received PoS since last restart.",
        &["bucket"]
    )
    .unwrap()
});

pub fn inc_remote_pos_count(bucket: u64) {
    REMOTE_POS_COUNT
        .with_label_values(&[bucket.to_string().as_str()])
        .inc()
}

static REJECTED_POS_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "quorum_store_rejected_PoS_count",
        "Count of the rejected PoS since last restart, grouped by reason.",
        &["reason"]
    )
    .unwrap()
});

pub fn inc_rejected_pos_count(reason: &str) {
    REJECTED_POS_COUNT.with_label_values(&[reason]).inc();
}

/// Count of the received batches since last restart.
pub static RECEIVED_REMOTE_BATCHES_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_received_remote_batch_count",
        "Count of the received batches since last restart."
    )
    .unwrap()
});

/// Count of the received batch msg since last restart.
pub static RECEIVED_BATCH_MSG_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_received_batch_msg_count",
        "Count of the received batch msg since last restart."
    )
    .unwrap()
});

/// Count of the received batch since last restart.
pub static RECEIVED_BATCH_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_received_batch_count",
        "Count of the received end batch since last restart."
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

/// Count of the exceeded storage quota.
pub static EXCEEDED_STORAGE_QUOTA_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_exceeded_storage_quota_count",
        "Count of the exceeded storage quota."
    )
    .unwrap()
});

/// Count of the exceeded batch quota.
pub static EXCEEDED_BATCH_QUOTA_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_exceeded_batch_quota_count",
        "Count of the exceeded batch quota."
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
pub static RECEIVED_BATCH_RESPONSE_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_received_batch_response_count",
        "Count of the number of batches received from other nodes."
    )
    .unwrap()
});

pub static QS_BACKPRESSURE_TXN_COUNT: Lazy<Histogram> = Lazy::new(|| {
    register_avg_counter(
        "quorum_store_backpressure_txn_count",
        "Indicator of whether Quorum Store is backpressured due to txn count exceeding threshold.",
    )
});

pub static QS_BACKPRESSURE_PROOF_COUNT: Lazy<Histogram> = Lazy::new(|| {
    register_avg_counter(
        "quorum_store_backpressure_proof_count",
        "Indicator of whether Quorum Store is backpressured due to proof count exceeding threshold."
    )
});

pub static QS_BACKPRESSURE_DYNAMIC_MAX: Lazy<Histogram> = Lazy::new(|| {
    register_avg_counter(
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
            QUORUM_STORE_LATENCY_BUCKETS.to_vec()
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
            QUORUM_STORE_LATENCY_BUCKETS.to_vec()
        )
        .unwrap(),
    )
});

/// Histogram of the time it takes to compute bucketed batches after txns are pulled from mempool.
pub static BATCH_CREATION_COMPUTE_LATENCY: Lazy<DurationHistogram> = Lazy::new(|| {
    DurationHistogram::new(
        register_histogram!(
            "quorum_store_batch_creation_compute_latency",
            "Histogram of the time it takes to compute bucketed batches after txns are pulled from mempool.",
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
            QUORUM_STORE_LATENCY_BUCKETS.to_vec()
        )
        .unwrap(),
    )
});

pub static BATCH_SUCCESSFUL_CREATION: Lazy<Histogram> = Lazy::new(|| {
    register_avg_counter(
        "quorum_store_batch_successful_creation",
        "Counter for whether we are successfully creating batches",
    )
});

/// Number of validators for which we received signed replies
pub static BATCH_RECEIVED_REPLIES_COUNT: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "quorum_store_batch_received_replies_votes",
        "Number of validators for which we received signed replies.",
        TRANSACTION_COUNT_BUCKETS.clone(),
    )
    .unwrap()
});

/// Voting power of validators for which we received signed replies
pub static BATCH_RECEIVED_REPLIES_VOTING_POWER: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "quorum_store_batch_received_replies_voting_power",
        "Voting power of validators for which we received signed replies.",
        TRANSACTION_COUNT_BUCKETS.clone(),
    )
    .unwrap()
});
