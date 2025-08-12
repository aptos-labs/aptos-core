// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_config::network_id::{NetworkId, PeerNetworkId};
use aptos_metrics_core::{
    exponential_buckets, histogram_opts, op_counters::DurationHistogram, register_histogram,
    register_histogram_vec, register_int_counter, register_int_counter_vec, register_int_gauge,
    register_int_gauge_vec, Histogram, HistogramTimer, HistogramVec, IntCounter, IntCounterVec,
    IntGauge, IntGaugeVec,
};
use aptos_short_hex_str::AsShortHexStr;
use once_cell::sync::Lazy;
use std::time::Duration;

// Core mempool index labels
pub const PRIORITY_INDEX_LABEL: &str = "priority";
pub const EXPIRATION_TIME_INDEX_LABEL: &str = "expiration";
pub const SYSTEM_TTL_INDEX_LABEL: &str = "system_ttl";
pub const TIMELINE_INDEX_LABEL: &str = "timeline";
pub const PARKING_LOT_INDEX_LABEL: &str = "parking_lot";
pub const TRANSACTION_HASH_INDEX_LABEL: &str = "transaction_hash";
pub const SIZE_BYTES_LABEL: &str = "size_bytes";

// Core mempool stages labels
pub const BROADCAST_RECEIVED_LABEL: &str = "broadcast_received";
pub const COMMIT_ACCEPTED_LABEL: &str = "commit_accepted";
pub const COMMIT_ACCEPTED_BLOCK_LABEL: &str = "commit_accepted_block";
pub const COMMIT_REJECTED_LABEL: &str = "commit_rejected";
pub const COMMIT_REJECTED_DUPLICATE_LABEL: &str = "commit_rejected_duplicate";
pub const COMMIT_IGNORED_LABEL: &str = "commit_ignored";
pub const CONSENSUS_READY_LABEL: &str = "consensus_ready";
pub const CONSENSUS_PULLED_LABEL: &str = "consensus_pulled";
pub const BROADCAST_READY_LABEL: &str = "broadcast_ready";
pub const BROADCAST_BATCHED_LABEL: &str = "broadcast_batched";
pub const PARKED_TIME_LABEL: &str = "parked_time";
pub const NON_PARKED_COMMIT_ACCEPTED_LABEL: &str = "non_park_commit_accepted";

// Core mempool GC type labels
pub const GC_SYSTEM_TTL_LABEL: &str = "system_ttl";
pub const GC_CLIENT_EXP_LABEL: &str = "client_expiration";

// Core mempool GC txn status label
pub const GC_ACTIVE_TXN_LABEL: &str = "active";
pub const GC_PARKED_TXN_LABEL: &str = "parked";

// Mempool service request type labels
pub const GET_BLOCK_LABEL: &str = "get_block";
pub const GET_BLOCK_LOCK_LABEL: &str = "get_block_lock";
pub const GET_BLOCK_GC_LABEL: &str = "get_block_gc";
pub const GET_BLOCK_GET_BATCH_LABEL: &str = "get_block_get_batch";
pub const COMMIT_STATE_SYNC_LABEL: &str = "commit_accepted";
pub const COMMIT_CONSENSUS_LABEL: &str = "commit_rejected";

// Mempool service request result labels
pub const REQUEST_FAIL_LABEL: &str = "fail";
pub const REQUEST_SUCCESS_LABEL: &str = "success";

// Process txn breakdown type labels
pub const FETCH_SEQ_NUM_LABEL: &str = "storage_fetch";
pub const FILTER_TRANSACTIONS_LABEL: &str = "filter_transactions";
pub const VM_VALIDATION_LABEL: &str = "vm_validation";

// Txn process result labels
pub const CLIENT_LABEL: &str = "client";
pub const SUCCESS_LABEL: &str = "success";

// Bounded executor task labels
pub const CLIENT_EVENT_LABEL: &str = "client_event";
pub const CLIENT_EVENT_GET_TXN_LABEL: &str = "client_event_get_txn";
pub const CLIENT_EVENT_GET_PARKING_LOT_ADDRESSES: &str = "client_event_get_parking_lot_addresses";
pub const RECONFIG_EVENT_LABEL: &str = "reconfig";
pub const PEER_BROADCAST_EVENT_LABEL: &str = "peer_broadcast";

// task spawn stage labels
pub const SPAWN_LABEL: &str = "spawn";
pub const START_LABEL: &str = "start";

// Mempool network msg failure type labels:
pub const BROADCAST_TXNS: &str = "broadcast_txns";
pub const ACK_TXNS: &str = "ack_txns";

// Broadcast/ACK type labels
pub const EXPIRED_BROADCAST_LABEL: &str = "expired";
pub const RETRY_BROADCAST_LABEL: &str = "retry";
pub const BACKPRESSURE_BROADCAST_LABEL: &str = "backpressure";

// ACK direction labels
pub const RECEIVED_LABEL: &str = "received";
pub const SENT_LABEL: &str = "sent";

// invalid ACK type labels
pub const UNKNOWN_PEER: &str = "unknown_peer";

// Event types for ranking_score
pub const INSERT_LABEL: &str = "insert";
pub const REMOVE_LABEL: &str = "remove";

// The submission point where the transaction originated from
pub const SUBMITTED_BY_CLIENT_LABEL: &str = "client";
pub const SUBMITTED_BY_DOWNSTREAM_LABEL: &str = "downstream";
pub const SUBMITTED_BY_PEER_VALIDATOR_LABEL: &str = "peer_validator";

// Broadcast event labels
pub const DROP_BROADCAST_LABEL: &str = "drop_broadcast";
pub const RUNNING_LABEL: &str = "running";

// Histogram buckets with a large range of 0-500s and some constant sized buckets between:
// 0-1.5s (every 25ms), 1.5-2s (every 100ms), 2-5s (250ms), 5-10s (1s), and 10-25s (2.5s).
const MEMPOOL_LATENCY_BUCKETS: &[f64] = &[
    0.025, 0.05, 0.075, 0.1, 0.125, 0.15, 0.175, 0.2, 0.225, 0.250, 0.275, 0.3, 0.325, 0.35, 0.375,
    0.4, 0.425, 0.45, 0.475, 0.5, 0.525, 0.55, 0.575, 0.6, 0.625, 0.65, 0.675, 0.7, 0.725, 0.75,
    0.775, 0.8, 0.825, 0.85, 0.875, 0.9, 0.925, 0.95, 0.975, 1.0, 1.025, 1.05, 1.075, 1.1, 1.125,
    1.15, 1.175, 1.2, 1.225, 1.25, 1.275, 1.3, 1.325, 1.35, 1.375, 1.4, 1.425, 1.45, 1.475, 1.5,
    1.6, 1.7, 1.8, 1.9, 2.0, 2.25, 2.5, 2.75, 3.0, 3.25, 3.5, 3.75, 4.0, 4.25, 4.5, 4.75, 5.0, 6.0,
    7.0, 8.0, 9.0, 10.0, 12.5, 15.0, 17.5, 20.0, 22.5, 25.0, 50.0, 100.0, 250.0, 500.0,
];

// Histogram buckets for tracking ranking score (see below test for the formula)
const RANKING_SCORE_BUCKETS: &[f64] = &[
    100.0, 147.0, 215.0, 316.0, 464.0, 681.0, 1000.0, 1468.0, 2154.0, 3162.0, 4642.0, 6813.0,
    10000.0, 14678.0, 21544.0, 31623.0, 46416.0, 68129.0, 100000.0, 146780.0, 215443.0,
];

const TXN_CONSENSUS_PULLED_BUCKETS: &[f64] = &[1.0, 2.0, 3.0, 4.0, 5.0, 10.0, 25.0, 50.0, 100.0];

static TXN_COUNT_BUCKETS: Lazy<Vec<f64>> = Lazy::new(|| {
    exponential_buckets(
        /*start=*/ 1.5, /*factor=*/ 1.5, /*count=*/ 20,
    )
    .unwrap()
});

/// Counter tracking size of various indices in core mempool
pub static CORE_MEMPOOL_INDEX_SIZE: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_core_mempool_index_size",
        "Size of a core mempool index",
        &["index"]
    )
    .unwrap()
});

pub fn core_mempool_index_size(label: &'static str, size: usize) {
    CORE_MEMPOOL_INDEX_SIZE
        .with_label_values(&[label])
        .set(size as i64)
}

pub static SENDER_BUCKET_FREQUENCIES: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_core_mempool_sender_bucket_frequencies",
        "Frequency of each sender bucket in core mempool",
        &["sender_bucket"]
    )
    .unwrap()
});

/// Counter tracking size of each bucket in timeline index
static CORE_MEMPOOL_TIMELINE_INDEX_SIZE: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_core_mempool_timeline_index_size",
        "Size of each bucket in core mempool timeline index",
        &["bucket"]
    )
    .unwrap()
});

pub fn core_mempool_timeline_index_size(bucket_min_size_pairs: Vec<(String, usize)>) {
    for (bucket_min, size) in bucket_min_size_pairs {
        CORE_MEMPOOL_TIMELINE_INDEX_SIZE
            .with_label_values(&[bucket_min.as_str()])
            .set(size as i64)
    }
}

/// Counter tracking number of txns removed from core mempool
pub static CORE_MEMPOOL_REMOVED_TXNS: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_core_mempool_removed_txns_count",
        "Number of txns removed from core mempool"
    )
    .unwrap()
});

/// Counter tracking number of txns received that are idempotent duplicates
pub static CORE_MEMPOOL_IDEMPOTENT_TXNS: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_core_mempool_idempotent_txns_count",
        "Number of txns received that are idempotent duplicates"
    )
    .unwrap()
});

/// Counter tracking number of txns received that are gas upgraded for the same sequence number
pub static CORE_MEMPOOL_GAS_UPGRADED_TXNS: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_core_mempool_gas_upgraded_txns_count",
        "Number of txns received that are gas upgraded for the same sequence number"
    )
    .unwrap()
});

pub fn core_mempool_txn_commit_latency(
    stage: &'static str,
    submitted_by: &'static str,
    bucket: &str,
    latency: Duration,
    priority: &str,
) {
    CORE_MEMPOOL_TXN_COMMIT_LATENCY
        .with_label_values(&[stage, submitted_by, bucket])
        .observe(latency.as_secs_f64());

    CORE_MEMPOOL_TXN_LATENCIES
        .with_label_values(&[stage, submitted_by, bucket, priority])
        .observe(latency.as_secs_f64());
}

/// Counter tracking latency of txns reaching various stages in committing
/// (e.g. time from txn entering core mempool to being pulled in consensus block)
static CORE_MEMPOOL_TXN_COMMIT_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_core_mempool_txn_commit_latency",
        "Latency of txn reaching various stages in core mempool after insertion",
        &["stage", "submitted_by", "bucket"],
        MEMPOOL_LATENCY_BUCKETS.to_vec()
    )
    .unwrap()
});

/// Counter tracking latency of txns reaching various stages
/// (e.g. time from txn entering core mempool to being pulled in consensus block)
static CORE_MEMPOOL_TXN_LATENCIES: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_core_mempool_txn_latencies",
        "Latency of txn reaching various stages in mempool",
        &["stage", "submitted_by", "bucket", "priority"],
        MEMPOOL_LATENCY_BUCKETS.to_vec()
    )
    .unwrap()
});

pub static TXN_E2E_USE_CASE_COMMIT_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_txn_e2e_use_case_commit_latency",
        "Latency of txn commit_accept, by use_case",
        &["use_case", "submitted_by", "bucket"],
        MEMPOOL_LATENCY_BUCKETS.to_vec()
    )
    .unwrap()
});

pub fn core_mempool_txn_ranking_score(
    stage: &'static str,
    status: &str,
    bucket: &str,
    ranking_score: u64,
) {
    CORE_MEMPOOL_TXN_RANKING_BUCKET
        .with_label_values(&[stage, status, bucket])
        .inc();
    CORE_MEMPOOL_TXN_RANKING_SCORE
        .with_label_values(&[stage, status])
        .observe(ranking_score as f64);
}

static CORE_MEMPOOL_TXN_RANKING_BUCKET: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_core_mempool_txn_ranking_bucket",
        "Ranking bucket of txn reaching various stages in core mempool",
        &["stage", "status", "bucket"]
    )
    .unwrap()
});

static CORE_MEMPOOL_TXN_RANKING_SCORE: Lazy<HistogramVec> = Lazy::new(|| {
    let histogram_opts = histogram_opts!(
        "aptos_core_mempool_txn_ranking_score",
        "Ranking score of txn reaching various stages in core mempool",
        RANKING_SCORE_BUCKETS.to_vec()
    );
    register_histogram_vec!(histogram_opts, &["stage", "status"]).unwrap()
});

/// Counter for number of periodic garbage-collection (=GC) events that happen, regardless of
/// how many txns were actually cleaned up in this GC event
pub static CORE_MEMPOOL_GC_EVENT_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_core_mempool_gc_event_count",
        "Number of times the periodic garbage-collection event occurs, regardless of how many txns were actually removed",
        &["type"])
        .unwrap()
});

/// Counter for number of periodic client garbage-collection (=GC) events that happen with eager
/// expiration, regardless of how many txns were actually cleaned up in this GC event
pub static CORE_MEMPOOL_GC_EAGER_EXPIRE_EVENT_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_core_mempool_gc_eager_expire_event_count",
        "Number of times the periodic garbage-collection event triggers eager expiration, regardless of how many txns were actually removed")
        .unwrap()
});

/// Counter tracking time for how long a transaction stayed in core-mempool before being garbage-collected
pub static CORE_MEMPOOL_GC_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_core_mempool_gc_latency",
        "How long a transaction stayed in core mempool before garbage-collected",
        &["type", "status"]
    )
    .unwrap()
});

pub static CORE_MEMPOOL_TXN_CONSENSUS_PULLED_BY_BUCKET: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_core_mempool_txn_consensus_pulled_by_bucket",
        "Number of times a txn was pulled from core mempool by consensus",
        &["bucket"],
        TXN_CONSENSUS_PULLED_BUCKETS.to_vec()
    )
    .unwrap()
});

pub static CORE_MEMPOOL_PARKING_LOT_EVICTED_COUNT: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "aptos_core_mempool_parking_lot_evicted_count",
        "Number of txns evicted from parking lot",
        TXN_COUNT_BUCKETS.clone()
    )
    .unwrap()
});

pub static CORE_MEMPOOL_PARKING_LOT_EVICTED_BYTES: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "aptos_core_mempool_parking_lot_evicted_bytes",
        "Bytes of txns evicted from parking lot",
        exponential_buckets(/*start=*/ 500.0, /*factor=*/ 1.4, /*count=*/ 32).unwrap()
    )
    .unwrap()
});

pub static CORE_MEMPOOL_PARKING_LOT_EVICTED_LATENCY: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "aptos_core_mempool_parking_lot_evicted_latency",
        "Latency of evicting for each transaction from parking lot",
        MEMPOOL_LATENCY_BUCKETS.to_vec()
    )
    .unwrap()
});

/// Counter of pending network events to Mempool
pub static PENDING_MEMPOOL_NETWORK_EVENTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_mempool_pending_network_events",
        "Counters(queued,dequeued,dropped) related to pending network notifications to Mempool",
        &["state"]
    )
    .unwrap()
});

/// Counter of number of txns processed in each consensus/state sync message
/// (e.g. # txns in block pulled by consensus, # txns committed from state sync)
static MEMPOOL_SERVICE_TXNS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_mempool_service_transactions",
        "Number of transactions handled in one request/response between mempool and consensus/state sync",
        &["type"],
        TXN_COUNT_BUCKETS.clone()
    )
        .unwrap()
});

pub fn mempool_service_transactions(label: &'static str, num: usize) {
    MEMPOOL_SERVICE_TXNS
        .with_label_values(&[label])
        .observe(num as f64)
}

/// Histogram for the byte size of transactions processed in get_block
pub static MEMPOOL_SERVICE_BYTES_GET_BLOCK: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "aptos_mempool_service_bytes_get_block",
        "Histogram for the number of txns per (mempool returned for proposal) blocks."
    )
    .unwrap()
});

/// Counter for tracking latency of mempool processing requests from consensus/state sync
/// A 'fail' result means the mempool's callback response to consensus/state sync failed.
static MEMPOOL_SERVICE_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_mempool_service_latency_ms",
        "Latency of mempool processing request from consensus/state sync",
        &["type", "result"]
    )
    .unwrap()
});

pub fn mempool_service_latency(label: &'static str, result: &str, duration: Duration) {
    MEMPOOL_SERVICE_LATENCY
        .with_label_values(&[label, result])
        .observe(duration.as_secs_f64());
}

pub fn mempool_service_start_latency_timer(label: &'static str, result: &str) -> HistogramTimer {
    MEMPOOL_SERVICE_LATENCY
        .with_label_values(&[label, result])
        .start_timer()
}

/// Counter for types of network messages received by shared mempool
static SHARED_MEMPOOL_EVENTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_shared_mempool_events",
        "Number of network events received by shared mempool",
        &["event"] // type of event: "new_peer", "lost_peer", "message"
    )
    .unwrap()
});

pub fn shared_mempool_event_inc(event: &'static str) {
    SHARED_MEMPOOL_EVENTS.with_label_values(&[event]).inc();
}

/// Counter for tracking e2e latency for mempool to process txn submission requests from clients and peers
static PROCESS_TXN_SUBMISSION_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_shared_mempool_request_latency",
        "Latency of mempool processing txn submission requests",
        &["network"]
    )
    .unwrap()
});

pub fn process_txn_submit_latency_timer(network_id: NetworkId) -> HistogramTimer {
    PROCESS_TXN_SUBMISSION_LATENCY
        .with_label_values(&[network_id.as_str()])
        .start_timer()
}

pub fn process_txn_submit_latency_timer_client() -> HistogramTimer {
    PROCESS_TXN_SUBMISSION_LATENCY
        .with_label_values(&[CLIENT_LABEL])
        .start_timer()
}

/// Counter for tracking e2e latency for mempool to process get txn by hash requests from clients and peers
static PROCESS_GET_TXN_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_shared_mempool_get_txn_request_latency",
        "Latency of mempool processing get txn by hash requests",
        &["network"]
    )
    .unwrap()
});

pub fn process_get_txn_latency_timer_client() -> HistogramTimer {
    PROCESS_GET_TXN_LATENCY
        .with_label_values(&[CLIENT_LABEL])
        .start_timer()
}

/// Tracks latency of different stages of txn processing (e.g. vm validation, storage read)
pub static PROCESS_TXN_BREAKDOWN_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_mempool_process_txn_breakdown_latency",
        "Latency of different stages of processing txns in mempool",
        &["portion"]
    )
    .unwrap()
});

/// Counter for tracking latency for mempool to broadcast to a peer
static SHARED_MEMPOOL_BROADCAST_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_mempool_broadcast_latency",
        "Latency of mempool executing broadcast to another peer",
        &["network"]
    )
    .unwrap()
});

pub fn shared_mempool_broadcast_latency(network_id: NetworkId, latency: Duration) {
    SHARED_MEMPOOL_BROADCAST_LATENCY
        .with_label_values(&[network_id.as_str()])
        .observe(latency.as_secs_f64());
}

/// Counter for tracking roundtrip-time from sending a broadcast to receiving ACK for that broadcast
pub static SHARED_MEMPOOL_BROADCAST_RTT: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_shared_mempool_broadcast_roundtrip_latency",
        "Time elapsed between sending a broadcast and receiving an ACK for that broadcast",
        &["network"]
    )
    .unwrap()
});

/// Counter tracking number of mempool broadcasts that have not been ACK'ed for
static SHARED_MEMPOOL_PENDING_BROADCASTS_COUNT: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_shared_mempool_pending_broadcasts_count",
        "Number of mempool broadcasts not ACK'ed for yet",
        &["network", "recipient"]
    )
    .unwrap()
});

pub fn shared_mempool_pending_broadcasts(peer: &PeerNetworkId) -> IntGauge {
    SHARED_MEMPOOL_PENDING_BROADCASTS_COUNT.with_label_values(&[
        peer.network_id().as_str(),
        peer.peer_id().short_str().as_str(),
    ])
}

/// Counter tracking the number of peers that changed priority in shared mempool
pub static SHARED_MEMPOOL_PRIORITY_CHANGE_COUNT: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_shared_mempool_priority_change_count",
        "Number of peers that changed priority in shared mempool",
    )
    .unwrap()
});

pub fn shared_mempool_priority_change_count(change_count: i64) {
    SHARED_MEMPOOL_PRIORITY_CHANGE_COUNT.set(change_count);
}

static SHARED_MEMPOOL_TRANSACTIONS_PROCESSED: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_shared_mempool_transactions_processed",
        "Number of transactions received and handled by shared mempool",
        &[
            "status", // state of transaction processing: "received", "success", status code from failed txn processing
            "network", // state of transaction processing: "received", "success", status code from failed txn processing
        ]
    )
    .unwrap()
});

pub fn shared_mempool_transactions_processed_inc(status: &str, network: &str) {
    SHARED_MEMPOOL_TRANSACTIONS_PROCESSED
        .with_label_values(&[status, network])
        .inc();
}

/// Counter for number of transactions in each mempool broadcast sent
static SHARED_MEMPOOL_TRANSACTION_BROADCAST_SIZE: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_shared_mempool_transaction_broadcast",
        "Number of transactions in each mempool broadcast sent",
        &["network"]
    )
    .unwrap()
});

pub fn shared_mempool_broadcast_size(network_id: NetworkId, num_txns: usize) {
    SHARED_MEMPOOL_TRANSACTION_BROADCAST_SIZE
        .with_label_values(&[network_id.as_str()])
        .observe(num_txns as f64);
}

/// Counter for the number and type of broadcast events that shared mempool executes
static SHARED_MEMPOOL_BROADCAST_EVENTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_shared_mempool_broadcast_events",
        "Broadcast events (at runtime) for shared mempool",
        &["event", "network_id"]
    )
    .unwrap()
});

pub fn shared_mempool_broadcast_event_inc(event_label: &str, network_id: NetworkId) {
    SHARED_MEMPOOL_BROADCAST_EVENTS
        .with_label_values(&[event_label, network_id.as_str()])
        .inc();
}

static SHARED_MEMPOOL_BROADCAST_TYPE_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_shared_mempool_rebroadcast_count",
        "Number of various types of broadcasts executed by shared mempool",
        &["network", "type"]
    )
    .unwrap()
});

pub fn shared_mempool_broadcast_type_inc(network_id: NetworkId, label: &str) {
    SHARED_MEMPOOL_BROADCAST_TYPE_COUNT
        .with_label_values(&[network_id.as_str(), label])
        .inc();
}

static SHARED_MEMPOOL_ACK_TYPE_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_shared_mempool_ack_count",
        "Number of various types of ACKs sent/received by shared mempool",
        &["network", "direction", "type"]
    )
    .unwrap()
});

pub fn shared_mempool_ack_inc(network_id: NetworkId, direction: &str, label: &'static str) {
    SHARED_MEMPOOL_ACK_TYPE_COUNT
        .with_label_values(&[network_id.as_str(), direction, label])
        .inc();
}

static TASK_SPAWN_LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_mempool_bounded_executor_spawn_latency",
        "Time it takes for mempool's coordinator to spawn async tasks",
        &["task", "stage"]
    )
    .unwrap()
});

pub fn task_spawn_latency_timer(task: &'static str, stage: &'static str) -> HistogramTimer {
    TASK_SPAWN_LATENCY
        .with_label_values(&[task, stage])
        .start_timer()
}

pub static CORE_MEMPOOL_INVARIANT_VIOLATION_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_mempool_core_mempool_invariant_violated_count",
        "Number of times a core mempool invariant was violated"
    )
    .unwrap()
});

pub static VM_RECONFIG_UPDATE_FAIL_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_mempool_vm_reconfig_update_fail_count",
        "Number of times mempool's VM reconfig update failed"
    )
    .unwrap()
});

/// Counter for failed network sends
static NETWORK_SEND_FAIL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_mempool_network_send_fail_count",
        "Number of times mempool network send failure occurs",
        &["type"]
    )
    .unwrap()
});

pub fn network_send_fail_inc(label: &'static str) {
    NETWORK_SEND_FAIL.with_label_values(&[label]).inc();
}

static UNEXPECTED_NETWORK_MSG_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_mempool_unexpected_network_count",
        "Number of unexpected network msgs received",
        &["network"]
    )
    .unwrap()
});

pub fn unexpected_msg_count_inc(network_id: &NetworkId) {
    UNEXPECTED_NETWORK_MSG_COUNT
        .with_label_values(&[network_id.as_str()])
        .inc();
}

/// Counter for failed callback response to JSON RPC
pub static CLIENT_CALLBACK_FAIL: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_mempool_json_rpc_callback_fail_count",
        "Number of times callback to JSON RPC failed in mempool"
    )
    .unwrap()
});

/// Counter for how many ACKs were received with an invalid request_id that this node's mempool
/// did not send
static INVALID_ACK_RECEIVED_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_mempool_unrecognized_ack_received_count",
        "Number of ACK messages received with an invalid request_id that this node's mempool did not send",
        &["network", "type"]
    )
        .unwrap()
});

pub fn invalid_ack_inc(network_id: NetworkId, label: &'static str) {
    INVALID_ACK_RECEIVED_COUNT
        .with_label_values(&[network_id.as_str(), label])
        .inc();
}

/// Counter for number of times a DB read resulted in error
pub static DB_ERROR: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_mempool_db_error_count",
        "Number of times a DB read error was encountered in mempool"
    )
    .unwrap()
});

/// Counter for the current number of active upstream peers mempool can
/// broadcast to, summed across each of its networks
static ACTIVE_UPSTREAM_PEERS_COUNT: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_mempool_active_upstream_peers_count",
        "Number of active upstream peers for the node of this mempool",
        &["network"]
    )
    .unwrap()
});

pub fn active_upstream_peers(network_id: &NetworkId) -> IntGauge {
    ACTIVE_UPSTREAM_PEERS_COUNT.with_label_values(&[network_id.as_str()])
}

/// Duration of each run of the event loop.
pub static MAIN_LOOP: Lazy<DurationHistogram> = Lazy::new(|| {
    DurationHistogram::new(
        register_histogram!(
            "aptos_mempool_main_loop",
            "Duration of the each run of the event loop"
        )
        .unwrap(),
    )
});

#[cfg(test)]
mod test {
    use crate::counters::RANKING_SCORE_BUCKETS;

    #[test]
    fn generate_ranking_score_buckets() {
        let buckets: Vec<f64> = (0..21)
            .map(|n| 100.0 * (10.0_f64.powf(n as f64 / 6.0)))
            .map(|f| f.round())
            .collect();
        assert_eq!(RANKING_SCORE_BUCKETS, &buckets);
    }
}
