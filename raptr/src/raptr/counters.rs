use aptos_metrics_core::{
    register_counter, register_histogram, register_histogram_vec, register_int_counter,
    register_int_counter_vec, Histogram, HistogramVec, IntCounter, IntCounterVec,
};
use once_cell::sync::Lazy;

pub static OP_COUNTERS: Lazy<aptos_metrics_core::op_counters::OpMetrics> =
    Lazy::new(|| aptos_metrics_core::op_counters::OpMetrics::new_and_registered("raptr"));

pub static RAIKOU_BATCH_CONSENSUS_LATENCY: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "raikou_batch_consensus_latency",
        "Raikou Batch Consensus Latency",
    )
    .unwrap()
});

pub static RAIKOU_BLOCK_CONSENSUS_LATENCY: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "raikou_block_consensus_latency",
        "Raikou Block Consensus Latnecy"
    )
    .unwrap()
});

pub static RAIKOU_BLOCK_COMMIT_RATE: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!("raikou_block_commit_rate", "Raikou Block Commit Rate").unwrap()
});

const BLOCK_TRACING_BUCKETS: &[f64] = &[
    0.005, 0.01, 0.025, 0.05, 0.1, 0.15, 0.2, 0.25, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 1.1,
    1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.9, 2.0, 2.2, 2.4, 2.6, 2.8, 3.0, 3.2, 3.4, 3.6, 3.8, 4.0,
    4.5, 5.0, 5.5, 6.0, 6.5, 7.0, 7.5, 10.0,
];

/// Traces block movement throughout the node
pub static BLOCK_TRACING: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_consensus_block_tracing",
        "Histogram for different stages of a block",
        &["stage"],
        BLOCK_TRACING_BUCKETS.to_vec()
    )
    .unwrap()
});

pub static ROUND_ENTER_REASON: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!("raikou_round_enter_reason", "Raikou Round Enter Reason", &[
        "reason"
    ])
    .unwrap()
});

pub static QC_VOTING_PREFIX_HISTOGRAM: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!("raikou_qc_voting_prefix", "Raikou Voting Prefix", vec![
        0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0
    ])
    .unwrap()
});

pub static QC_PREFIX_HISTOGRAM: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!("raikou_qc_prefix", "Raikou QC Prefix", vec![
        0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0
    ])
    .unwrap()
});

pub static PREFIX_VOTED_PREVIOUSLY_COUNTER: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "raikou_prefix_voted_previously_count",
        "Raikou Voting Prefix Previous Count",
    )
    .unwrap()
});

pub static QC_TIMER_VOTE_FULLBLOCK_COUNTER: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "raikou_qc_timer_vote_fullblock_count",
        "Raikou QC Timer Vote Full Block Count",
    )
    .unwrap()
});
