// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Metrics for proxy primary consensus.

use once_cell::sync::Lazy;
use prometheus::{register_int_counter, register_int_gauge, IntCounter, IntGauge};

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

/// Backpressure delay applied to proxy proposals in milliseconds
pub static PROXY_BACKPRESSURE_DELAY_MS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_proxy_backpressure_delay_ms",
        "Current backpressure delay applied to proxy proposals in milliseconds"
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
