// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Metrics for proxy primary consensus.

use once_cell::sync::Lazy;
use prometheus::{
    register_histogram, register_int_counter, register_int_gauge, Histogram, IntCounter, IntGauge,
};

/// Number of proxy proposals sent
pub static PROXY_CONSENSUS_PROPOSALS_SENT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_proxy_consensus_proposals_sent",
        "Number of proxy proposals sent"
    )
    .unwrap()
});

/// Number of proxy votes sent
pub static PROXY_CONSENSUS_VOTES_SENT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_proxy_consensus_votes_sent",
        "Number of proxy votes sent"
    )
    .unwrap()
});

/// Number of proxy blocks ordered
pub static PROXY_CONSENSUS_BLOCKS_ORDERED: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_proxy_consensus_blocks_ordered",
        "Number of proxy blocks ordered"
    )
    .unwrap()
});

/// Number of proxy opt blocks ordered
pub static PROXY_CONSENSUS_OPT_BLOCKS_ORDERED: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_proxy_consensus_opt_blocks_ordered",
        "Number of proxy opt blocks ordered"
    )
    .unwrap()
});

/// Number of proxy consensus QCs formed
pub static PROXY_CONSENSUS_QCS_FORMED: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_proxy_consensus_qcs_formed",
        "Number of proxy consensus QCs formed"
    )
    .unwrap()
});

/// Number of ordered proxy block messages forwarded to primaries
pub static PROXY_CONSENSUS_BLOCKS_FORWARDED: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_proxy_consensus_blocks_forwarded",
        "Number of ordered proxy block messages forwarded to primaries"
    )
    .unwrap()
});

/// Remaining transaction budget for proxy blocks (resets on primary QC)
pub static PROXY_TXN_BUDGET_REMAINING: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_proxy_txn_budget_remaining",
        "Remaining proxy block transaction budget before exhaustion"
    )
    .unwrap()
});

/// Total transactions aggregated from proxy blocks into primary blocks
pub static PROXY_AGGREGATED_PAYLOAD_TXNS: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_proxy_aggregated_payload_txns",
        "Total transactions aggregated from proxy blocks into primary blocks"
    )
    .unwrap()
});

/// Current aggregated payload size (txn count) for the latest primary proposal
pub static PROXY_AGGREGATED_PAYLOAD_SIZE: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_proxy_aggregated_payload_size",
        "Transaction count in the latest aggregated proxy payload for primary proposal"
    )
    .unwrap()
});

/// Primary pipeline pending round gap as seen by proxy
pub static PROXY_PIPELINE_PENDING_GAP: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_proxy_pipeline_pending_gap",
        "Primary pipeline ordered-commit round gap as seen by proxy"
    )
    .unwrap()
});

/// Number of unconsumed proxy batches pending at primary
pub static PROXY_PENDING_BATCHES_AT_PRIMARY: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_proxy_pending_batches_at_primary",
        "Number of unconsumed proxy block batches pending at primary"
    )
    .unwrap()
});

/// Effective budget target after adaptive reduction
pub static PROXY_EFFECTIVE_BUDGET_TARGET: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_proxy_effective_budget_target",
        "Effective proxy block budget target after adaptive congestion reduction"
    )
    .unwrap()
});

/// Number of proxy blocks with transactions in current batch (since last cutting point)
pub static PROXY_BLOCKS_WITH_TXNS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_proxy_blocks_with_txns",
        "Proxy blocks with non-empty payload in current batch"
    )
    .unwrap()
});

/// Total proxy blocks in current batch (since last cutting point)
pub static PROXY_TOTAL_BLOCKS_IN_BATCH: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_proxy_total_blocks_in_batch",
        "Total proxy blocks in current batch since last cutting point"
    )
    .unwrap()
});

/// Counter for proxy rounds that returned empty payload due to budget exhaustion
pub static PROXY_EMPTY_PAYLOAD_BUDGET: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_proxy_empty_payload_budget",
        "Proxy rounds returning empty payload because budget was exhausted"
    )
    .unwrap()
});

/// Counter for proxy rounds that pulled real (non-empty) payload
pub static PROXY_NONEMPTY_PAYLOAD_PULLED: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_proxy_nonempty_payload_pulled",
        "Proxy rounds that pulled real payload from inner client"
    )
    .unwrap()
});

/// Effective max_txns after all adaptive reductions (gap-based + batch-based)
pub static PROXY_EFFECTIVE_MAX_TXNS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_proxy_effective_max_txns",
        "Effective max_txns per proxy block after adaptive backpressure reductions"
    )
    .unwrap()
});

/// Histogram for the number of proxy blocks included per primary block
pub static PROXY_BLOCKS_PER_PRIMARY: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "aptos_proxy_blocks_per_primary",
        "Number of proxy blocks aggregated into each primary block",
        vec![0.0, 1.0, 2.0, 3.0, 5.0, 8.0, 10.0, 15.0, 20.0, 30.0, 50.0]
    )
    .unwrap()
});

/// Histogram for txns per primary block (proxy-aggregated)
pub static PRIMARY_BLOCK_TXNS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "aptos_proxy_primary_block_txns",
        "Transaction count per primary block from proxy aggregation",
        vec![
            0.0, 50.0, 100.0, 250.0, 500.0, 750.0, 1000.0, 1500.0, 2000.0, 3000.0, 5000.0,
        ]
    )
    .unwrap()
});
