// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![allow(clippy::unwrap_used)]

use aptos_consensus_types::{
    block::Block, common::Payload, payload::OptQuorumStorePayload, proof_of_store::TBatchInfo,
};
use aptos_metrics_core::{
    exponential_buckets, op_counters::DurationHistogram, register_avg_counter, register_histogram,
    register_histogram_vec, register_int_counter, register_int_counter_vec, Histogram,
    HistogramVec, IntCounter, IntCounterVec,
};
use aptos_short_hex_str::AsShortHexStr;
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

static PROOF_COUNT_BUCKETS: Lazy<Vec<f64>> = Lazy::new(|| {
    [
        1.0, 3.0, 5.0, 7.0, 10.0, 12.0, 15.0, 20.0, 25.0, 30.0, 40.0, 50.0, 60.0, 75.0, 100.0,
        125.0, 150.0, 200.0, 250.0, 300.0, 500.0,
    ]
    .to_vec()
});

static BYTE_BUCKETS: Lazy<Vec<f64>> = Lazy::new(|| {
    exponential_buckets(
        /*start=*/ 500.0, /*factor=*/ 1.5, /*count=*/ 25,
    )
    .unwrap()
});

const INLINE_BATCH_COUNT_BUCKETS: &[f64] = &[0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];

// Histogram buckets that expand DEFAULT_BUCKETS with more granularity between 100-2000 ms
const QUORUM_STORE_LATENCY_BUCKETS: &[f64] = &[
    0.005, 0.01, 0.025, 0.05, 0.1, 0.15, 0.2, 0.25, 0.3, 0.35, 0.4, 0.45, 0.5, 0.55, 0.65, 0.7,
    0.75, 1.0, 1.25, 1.5, 2.0, 2.5, 5.0, 10.0,
];

// Same as QUORUM_STORE_LATENCY_BUCKETS but in milliseconds
const QUORUM_STORE_LATENCY_BUCKETS_IN_MS: &[f64] = &[
    5.0, 10.0, 25.0, 50.0, 100.0, 150.0, 200.0, 250.0, 300.0, 350.0, 400.0, 450.0, 500.0, 550.0,
    650.0, 700.0, 750.0, 1000.0, 1250.0, 1500.0, 2000.0, 2500.0, 5000.0, 10000.0,
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

pub static PROOF_QUEUE_ADD_BATCH_SUMMARIES_DURATION: Lazy<DurationHistogram> = Lazy::new(|| {
    DurationHistogram::new(
        register_histogram!(
            "quorum_store_proof_queue_add_batch_summaries_duration",
            "Duration of adding batch summaries to proof queue"
        )
        .unwrap(),
    )
});

pub static PROOF_QUEUE_COMMIT_DURATION: Lazy<DurationHistogram> = Lazy::new(|| {
    DurationHistogram::new(
        register_histogram!(
            "quorum_store_proof_queue_commit_duration",
            "Duration of committing proofs from proof queue"
        )
        .unwrap(),
    )
});

pub static PROOF_QUEUE_UPDATE_TIMESTAMP_DURATION: Lazy<DurationHistogram> = Lazy::new(|| {
    DurationHistogram::new(
        register_histogram!(
            "quorum_store_proof_queue_update_block_timestamp_duration",
            "Duration of updating block timestamp in proof queue"
        )
        .unwrap(),
    )
});

pub static PROOF_QUEUE_REMAINING_TXNS_DURATION: Lazy<DurationHistogram> = Lazy::new(|| {
    DurationHistogram::new(
        register_histogram!(
            "quorum_store_proof_queue_remaining_txns_duration",
            "Duration of calculating remaining txns in proof queue"
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
/// types: proof, inline_batch, opt_batch
pub static BATCH_NUM_PER_BLOCK: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "quorum_store_batch_num_per_block",
        "Histogram for the number of batches per (committed) blocks.",
        &["type"],
        TRANSACTION_COUNT_BUCKETS.clone(),
    )
    .unwrap()
});

/// Histogram for the number of txns per batch type in (committed) blocks.
/// types: proof, inline_batch, opt_batch
pub static TXN_NUM_PER_BATCH_TYPE_PER_BLOCK: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "quorum_store_txn_num_per_batch_type_per_block",
        "Histogram for the number of txns per batch type in (committed) blocks.",
        &["type"],
        TRANSACTION_COUNT_BUCKETS.clone(),
    )
    .unwrap()
});

/// Histogram for the txn bytes per batch type in (committed) blocks.
/// types: proof, inline_batch, opt_batch
pub static TXN_BYTES_PER_BATCH_TYPE_PER_BLOCK: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "quorum_store_txn_bytes_per_batch_type_per_block",
        "Histogram for the txn bytes per batch type in (committed) blocks.",
        &["type"],
        BYTE_BUCKETS.clone(),
    )
    .unwrap()
});

pub fn update_batch_stats(block: &Block) {
    let (proof_num, proof_txn_num, proof_txn_bytes) = block.proof_stats();
    BATCH_NUM_PER_BLOCK
        .with_label_values(&["proof"])
        .observe(proof_num as f64);
    TXN_NUM_PER_BATCH_TYPE_PER_BLOCK
        .with_label_values(&["proof"])
        .observe(proof_txn_num as f64);
    TXN_BYTES_PER_BATCH_TYPE_PER_BLOCK
        .with_label_values(&["proof"])
        .observe(proof_txn_bytes as f64);
    let (inline_batch_num, inline_batch_txn_num, inline_batch_txn_bytes) =
        block.inline_batch_stats();
    BATCH_NUM_PER_BLOCK
        .with_label_values(&["inline_batch"])
        .observe(inline_batch_num as f64);
    TXN_NUM_PER_BATCH_TYPE_PER_BLOCK
        .with_label_values(&["inline_batch"])
        .observe(inline_batch_txn_num as f64);
    TXN_BYTES_PER_BATCH_TYPE_PER_BLOCK
        .with_label_values(&["inline_batch"])
        .observe(inline_batch_txn_bytes as f64);
    let (opt_batch_num, opt_batch_txn_num, opt_batch_txn_bytes) = block.opt_batch_stats();
    BATCH_NUM_PER_BLOCK
        .with_label_values(&["opt_batch"])
        .observe(opt_batch_num as f64);
    TXN_NUM_PER_BATCH_TYPE_PER_BLOCK
        .with_label_values(&["opt_batch"])
        .observe(opt_batch_txn_num as f64);
    TXN_BYTES_PER_BATCH_TYPE_PER_BLOCK
        .with_label_values(&["opt_batch"])
        .observe(opt_batch_txn_bytes as f64);

    update_committed_batches_by_author(block);
}

fn update_committed_batches_by_author(block: &Block) {
    let Some(payload) = block.payload() else {
        return;
    };
    let Payload::OptQuorumStore(opt_qs) = payload else {
        return;
    };

    // Helper to record per-author stats for a batch type
    fn record_batch_author(author: aptos_types::PeerId, num_txns: u64, batch_type: &str) {
        let author_str = author.short_str();
        COMMITTED_BATCHES_BY_AUTHOR
            .with_label_values(&[author_str.as_str(), batch_type])
            .inc();
        COMMITTED_TXNS_BY_AUTHOR
            .with_label_values(&[author_str.as_str(), batch_type])
            .inc_by(num_txns);
    }

    match opt_qs {
        OptQuorumStorePayload::V1(p) => {
            for proof in p.proof_with_data().iter() {
                let info = proof.info();
                record_batch_author(info.author(), info.num_txns(), "proof");
            }
            for batch in p.opt_batches().iter() {
                record_batch_author(batch.author(), batch.num_txns(), "opt_batch");
            }
            for batch in p.inline_batches().iter() {
                let info = batch.info();
                record_batch_author(info.author(), TBatchInfo::num_txns(info), "inline_batch");
            }
        },
        OptQuorumStorePayload::V2(p) => {
            for proof in p.proof_with_data().iter() {
                let info = proof.info();
                record_batch_author(info.author(), info.num_txns(), "proof");
            }
            for batch in p.opt_batches().iter() {
                record_batch_author(batch.author(), batch.num_txns(), "opt_batch");
            }
            for batch in p.inline_batches().iter() {
                let info = batch.info();
                record_batch_author(info.author(), TBatchInfo::num_txns(info), "inline_batch");
            }
        },
    }
}

/// Histogram for the number of transactions per batch.
static NUM_TXN_PER_BATCH: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "quorum_store_num_txn_per_batch",
        "Histogram for the number of transanctions per batch.",
        &["bucket", "batch_version"],
        TRANSACTION_COUNT_BUCKETS.clone(),
    )
    .unwrap()
});

pub fn num_txn_per_batch(bucket_start: &str, num: usize, batch_version: &str) {
    NUM_TXN_PER_BATCH
        .with_label_values(&[bucket_start, batch_version])
        .observe(num as f64)
}

/// Histogram for the number of transactions per block when pulled for consensus.
pub static BLOCK_SIZE_WHEN_PULL: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "quorum_store_block_size_when_pull",
        "Histogram for the number of unique transactions per block when pulled for consensus.",
        TRANSACTION_COUNT_BUCKETS.clone(),
    )
    .unwrap()
});

pub static TOTAL_BLOCK_SIZE_WHEN_PULL: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "quorum_store_total_block_size_when_pull",
        "Histogram for the total number of transactions including duplicates per block when pulled for consensus.",
        BYTE_BUCKETS.clone(),
    )
    .unwrap()
});

/// Histogram for the number of transactions per block when pulled for consensus.
pub static CONSENSUS_PULL_NUM_TXNS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "quorum_store_consensus_pull_num_txns",
        "Histogram for the number of transactions including duplicates when pulled for consensus.",
        &["pull_kind"],
        TRANSACTION_COUNT_BUCKETS.clone(),
    )
    .unwrap()
});

/// Histogram for the number of transactions per block when pulled for consensus.
pub static CONSENSUS_PULL_NUM_UNIQUE_TXNS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "quorum_store_consensus_pull_num_unique_txns",
        "Histogram for the number of unique transactions when pulled for consensus.",
        &["pull_kind"],
        TRANSACTION_COUNT_BUCKETS.clone(),
    )
    .unwrap()
});

pub static CONSENSUS_PULL_NUM_TXNS_PER_KIND: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "quorum_store_consensus_pull_num_txns_per_kind",
        "Number of txns pulled for consensus, by pull kind and batch kind (normal, encrypted, or v1).",
        &["pull_kind", "batch_kind"],
        TRANSACTION_COUNT_BUCKETS.clone(),
    )
    .unwrap()
});

pub static CONSENSUS_PULL_SIZE_IN_BYTES: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "quorum_store_consensus_pull_size_in_bytes",
        "Histogram for the size of the pulled transactions for consensus.",
        &["pull_kind"],
        TRANSACTION_COUNT_BUCKETS.clone(),
    )
    .unwrap()
});

pub static KNOWN_DUPLICATE_TXNS_WHEN_PULL: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "quorum_store_known_duplicate_txns_when_pull",
        "Histogram for the number of known duplicate transactions in a block when pulled for consensus.",
        TRANSACTION_COUNT_BUCKETS.clone(),
    )
    .unwrap()
});

pub static NUM_INLINE_BATCHES: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "num_inline_batches_in_block_proposal",
        "Histogram for the number of inline batches in a block proposed by proof manager",
        INLINE_BATCH_COUNT_BUCKETS.to_vec(),
    )
    .unwrap()
});

pub static NUM_INLINE_TXNS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "num_inline_transactions_in_block_proposal",
        "Histogram for the number of inline transactions in a block proposed by proof manager",
        TRANSACTION_COUNT_BUCKETS.clone(),
    )
    .unwrap()
});

pub static NUM_BATCHES_WITHOUT_PROOF_OF_STORE: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "num_batches_without_proof_of_store",
        "Histogram for the number of batches without proof of store in proof manager",
        TRANSACTION_COUNT_BUCKETS.clone(),
    )
    .unwrap()
});

pub static PROOF_QUEUE_FULLY_UTILIZED: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "proof_queue_utilized_fully_in_proposal",
        "Histogram for whether the proof queue is fully utilized when creating block proposal",
        [0.0, 1.0].to_vec(),
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

pub static BATCH_IN_PROGRESS_COMMITTED: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_batch_in_progress_committed",
        "Number of batches that are removed from in progress by a commit."
    )
    .unwrap()
});

pub static NUM_CORRUPT_BATCHES: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "corrupt_batches_in_proof_manager",
        "Number of batches in proof manager for which the digest does not match"
    )
    .unwrap()
});

pub static BATCH_IN_PROGRESS_EXPIRED: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_batch_in_progress_expired",
        "Number of batches that are removed from in progress by a block timestamp expiration."
    )
    .unwrap()
});

pub static BATCH_IN_PROGRESS_TIMEOUT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_batch_in_progress_timeout",
        "Number of batches that are removed from in progress by a proof collection timeout."
    )
    .unwrap()
});

pub static BATCH_GENERATOR_SKIPPED_OVERSIZED_TXN: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_batch_generator_skipped_oversized_txn",
        "Number of transactions skipped because they exceed sender_max_batch_bytes."
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

//////////////////////
// Proof Queue
//////////////////////

pub static PROOFS_WITHOUT_BATCH_SUMMARY: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "quorum_store_proofs_without_batch_data",
        "Number of proofs received without batch data",
        PROOF_COUNT_BUCKETS.clone(),
    )
    .unwrap()
});

pub static PROOFS_WITH_BATCH_SUMMARY: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "quorum_store_proofs_with_batch_data",
        "Number of proofs received without batch data",
        PROOF_COUNT_BUCKETS.clone(),
    )
    .unwrap()
});

pub static TXNS_WITH_DUPLICATE_BATCHES: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "quorum_store_txns_with_duplicate_batches",
        "Number of transactions received with duplicate batches",
        TRANSACTION_COUNT_BUCKETS.clone(),
    )
    .unwrap()
});

pub static TXNS_IN_PROOFS_WITH_SUMMARIES: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "quorum_store_txns_in_proof_queue_with_summaries",
        "Number of transactions in the proof queue",
        TRANSACTION_COUNT_BUCKETS.clone(),
    )
    .unwrap()
});

pub static TXNS_IN_PROOFS_WITHOUT_SUMMARIES: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "quorum_store_txns_in_proof_queue_without_summaries",
        "Number of transactions in the proof queue",
        TRANSACTION_COUNT_BUCKETS.clone(),
    )
    .unwrap()
});

pub static NUM_PROOFS_IN_PROOF_QUEUE_AFTER_PULL: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "quorum_store_num_proofs_left_in_proof_queue_after_pull",
        "Histogram for the number of proofs left in the proof queue after block proposal generation.",
        PROOF_COUNT_BUCKETS.clone(),
    ).unwrap()
});

pub static NUM_TXNS_IN_PROOF_QUEUE_AFTER_PULL: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "quorum_store_num_txns_left_in_proof_queue_after_pull",
        "Histogram for the number of transactions left in the proof queue after block proposal generation.",
        TRANSACTION_COUNT_BUCKETS.clone(),
    ).unwrap()
});

/// Histogram for the number of total txns left after adding or cleaning batches.
pub static NUM_TOTAL_TXNS_LEFT_ON_UPDATE: Lazy<Histogram> = Lazy::new(|| {
    register_avg_counter(
        "quorum_store_num_total_txns_left_on_update",
        "Histogram for the number of total txns left after adding or cleaning batches.",
    )
});

pub static NUM_UNIQUE_TOTAL_TXNS_LEFT_ON_UPDATE: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "quorum_store_num_unique_total_txns_left_on_update",
        "Histogram for the number of total txns left after adding or cleaning batches, without duplicates.",
        TRANSACTION_COUNT_BUCKETS.clone()
    ).unwrap()
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

/// Number of txns (equals max_count) for each time the pull for batches returns full.
pub static BATCH_PULL_FULL_TXNS: Lazy<Histogram> = Lazy::new(|| {
    register_avg_counter(
        "quorum_store_batch_pull_full_txns",
        "Number of txns (equals max_count) for each time the pull for batches returns full.",
    )
});

/// Histogram for the number of txns excluded on pull for batches.
pub static BATCH_PULL_EXCLUDED_TXNS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "quorum_store_batch_pull_excluded_txns",
        "Histogram for the number of txns excluded on pull for batches.",
        TRANSACTION_COUNT_BUCKETS.clone()
    )
    .unwrap()
});

/// Count of the created batches since last restart, by version and kind.
pub static CREATED_BATCHES_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "quorum_store_created_batch_count",
        "Count of the created batches since last restart.",
        &["batch_version", "batch_kind"]
    )
    .unwrap()
});

/// Count of total transactions created, by version and kind.
pub static CREATED_TXNS_BY_KIND: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "quorum_store_created_txns_by_kind",
        "Count of total transactions created, split by batch version and kind.",
        &["batch_version", "batch_kind"]
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
        &["bucket", "batch_version"]
    )
    .unwrap()
});

pub fn inc_local_pos_count(bucket: u64, batch_version: &str) {
    LOCAL_POS_COUNT
        .with_label_values(&[bucket.to_string().as_str(), batch_version])
        .inc()
}

/// Count of the created proof-of-store (PoS) since last restart.
static REMOTE_POS_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "quorum_store_remote_PoS_count",
        "Count of the received PoS since last restart.",
        &["bucket", "batch_version"]
    )
    .unwrap()
});

pub fn inc_remote_pos_count(bucket: u64, batch_version: &str) {
    REMOTE_POS_COUNT
        .with_label_values(&[bucket.to_string().as_str(), batch_version])
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
pub static RECEIVED_REMOTE_BATCH_COUNT: Lazy<IntCounter> = Lazy::new(|| {
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

/// Count of the received batches that failed max limit check.
pub static RECEIVED_BATCH_MAX_LIMIT_FAILED: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_received_batch_max_limit_failed",
        "Count of the received batches that failed max limit check."
    )
    .unwrap()
});

/// Count of the batch messages that contained transactions rejected by the filter
pub static RECEIVED_BATCH_REJECTED_BY_FILTER: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_received_batch_rejected_by_filter",
        "Count of the batch messages that contained transactions rejected by the filter"
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

/// Count of the number of batch not found responses received from other nodes.
pub static RECEIVED_BATCH_NOT_FOUND_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_received_batch_not_found_count",
        "Count of the number of batch not found responses received from other nodes."
    )
    .unwrap()
});

/// Count of the number of batch expired responses received from other nodes.
pub static RECEIVED_BATCH_EXPIRED_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_received_batch_expired_count",
        "Count of the number of batch expired responses received from other nodes."
    )
    .unwrap()
});

/// Count of the number of error batches received from other nodes.
pub static RECEIVED_BATCH_RESPONSE_ERROR_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_received_batch_response_error_count",
        "Count of the number of error batches received from other nodes."
    )
    .unwrap()
});

pub static RECEIVED_BATCH_FROM_SUBSCRIPTION_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_batch_from_subscription_count",
        "Count of the number of batches received via batch store subscription."
    )
    .unwrap()
});

pub static QS_BACKPRESSURE_TXN_COUNT: Lazy<Histogram> = Lazy::new(|| {
    register_avg_counter(
        "quorum_store_backpressure_txn_count",
        "Indicator of whether Quorum Store is backpressured due to txn count exceeding threshold.",
    )
});

pub static QS_BACKPRESSURE_MAKE_STRICTER_TXN_COUNT: Lazy<Histogram> = Lazy::new(|| {
    register_avg_counter(
        "quorum_store_backpressure_make_stricter_txn_count",
        "Indicator of whether Quorum Store txn count backpressure is being made stricter.",
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

pub static GARBAGE_COLLECTED_IN_PROOF_QUEUE_COUNTER: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "quorum_store_garbage_collected_batch_count",
        "Count of the number of garbage collected batches.",
        &["reason"]
    )
    .unwrap()
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

/// Histogram of the time it takes to persist batches generated locally to the DB.
pub static BATCH_CREATION_PERSIST_LATENCY: Lazy<DurationHistogram> = Lazy::new(|| {
    DurationHistogram::new(
        register_histogram!(
            "quorum_store_batch_creation_persist_latency",
            "Histogram of the time it takes to persist batches generated locally to the DB.",
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

pub static SIGNED_BATCH_INFO_VERIFY_DURATION: Lazy<DurationHistogram> = Lazy::new(|| {
    DurationHistogram::new(
        register_histogram!(
            "quorum_store_signed_batch_info_verify_duration",
            "Histogram of the time durations for verifying signed batch info.",
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

pub static QUORUM_STORE_MSG_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "quorum_store_msg_count",
        "Count of messages received by various quoroum store components",
        &["type"]
    )
    .unwrap()
});

pub static TIME_LAG_IN_BATCH_PROOF_QUEUE: Lazy<DurationHistogram> = Lazy::new(|| {
    DurationHistogram::new(
        register_histogram!(
            "quorum_store_time_lag_in_proof_queue",
            "Time lag between txn timestamp and current time when txn is added to proof queue",
        )
        .unwrap(),
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

pub static BATCH_RECEIVED_LATE_REPLIES_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_batch_received_late_replies",
        "Number of votes that came late."
    )
    .unwrap()
});

pub static BATCH_COORDINATOR_NUM_BATCH_REQS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "quorum_store_batch_coord_requests",
        "Number of requests to batch coordinator.",
        &["bucket"]
    )
    .unwrap()
});

pub static REMOTE_BATCH_COORDINATOR_DROPPED_MSGS: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "quorum_store_remote_batch_coordinator_dropped_msgs",
        "Dropped messages at remote batch coordinator ingress."
    )
    .unwrap()
});

// Histogram buckets that expand DEFAULT_BUCKETS with more granularity:
// * 0.3 to 2.0: step 0.1
// * 2.0 to 4.0: step 0.2
// * 4.0 to 7.5: step 0.5
const BATCH_TRACING_BUCKETS: &[f64] = &[
    0.005, 0.01, 0.025, 0.05, 0.1, 0.15, 0.2, 0.25, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 1.1,
    1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.9, 2.0, 2.2, 2.4, 2.6, 2.8, 3.0, 3.2, 3.4, 3.6, 3.8, 4.0,
    4.5, 5.0, 5.5, 6.0, 6.5, 7.0, 7.5, 10.0,
];

pub static BATCH_TRACING: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "quorum_store_batch_tracing",
        "Histogram for different stages of a QS batch",
        &["author", "stage", "batch_version"],
        BATCH_TRACING_BUCKETS.to_vec()
    )
    .unwrap()
});

pub static BATCH_VOTE_PROGRESS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "quorum_store_batch_vote_progress",
        "Histogram for vote collection of a QS batch",
        &["author", "vote_pct", "batch_version"],
        BATCH_TRACING_BUCKETS.to_vec()
    )
    .unwrap()
});

/// Counter for committed batches per author and type (proof, opt_batch, inline_batch).
/// Unlike BATCH_PULLED_BY_AUTHOR which counts at proposal pull time, this counts
/// batches that are actually committed in blocks.
pub static COMMITTED_BATCHES_BY_AUTHOR: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "quorum_store_committed_batches_by_author",
        "Number of committed batches by author and batch type (proof, opt_batch, inline_batch)",
        &["author", "type"]
    )
    .unwrap()
});

/// Counter for committed txns per author and type (proof, opt_batch, inline_batch).
pub static COMMITTED_TXNS_BY_AUTHOR: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "quorum_store_committed_txns_by_author",
        "Number of committed transactions by author and batch type (proof, opt_batch, inline_batch)",
        &["author", "type"]
    )
    .unwrap()
});

/// Counter for batches pulled by author and pull_kind (proof, optbatch, inline).
pub static BATCH_PULLED_BY_AUTHOR: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "quorum_store_batch_pulled_by_author",
        "Number of batches pulled by author and pull kind",
        &["author", "pull_kind"]
    )
    .unwrap()
});

/// Histogram for batch age when pulled (in ms), by author and pull_kind.
pub static BATCH_AGE_WHEN_PULLED: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "quorum_store_batch_age_when_pulled_ms",
        "Batch age in ms when pulled into a block, by author and pull kind",
        &["author", "pull_kind"],
        QUORUM_STORE_LATENCY_BUCKETS_IN_MS.to_vec()
    )
    .unwrap()
});

/// Histogram for batch queue wait time in ms by author (time from insertion to pull).
pub static BATCH_QUEUE_DURATION: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "quorum_store_batch_queue_duration_ms",
        "Time in ms a batch spent in the queue before being pulled, by author",
        &["author"],
        QUORUM_STORE_LATENCY_BUCKETS_IN_MS.to_vec()
    )
    .unwrap()
});

/// Histogram for proof arrival delay in ms relative to batch insertion, by author.
pub static PROOF_DELAY_AFTER_BATCH: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "quorum_store_proof_delay_after_batch_ms",
        "Delay in ms between batch insertion and proof insertion, by author",
        &["author"],
        QUORUM_STORE_LATENCY_BUCKETS_IN_MS.to_vec()
    )
    .unwrap()
});

/// Counter for batches skipped by min_batch_age filter, by author.
pub static BATCH_SKIPPED_TOO_YOUNG: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "quorum_store_batch_skipped_too_young",
        "Number of batches skipped because they were too young (min_batch_age filter)",
        &["author"]
    )
    .unwrap()
});

pub static PROOF_MANAGER_OUT_OF_ORDER_PROOF_INSERTION: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "quorum_store_proof_manager_ooo_proof_insert",
        "Number of ooo proof insertions into proof manager",
        &["author"]
    )
    .unwrap()
});
