// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{
    register_int_counter, register_int_gauge, register_int_gauge_vec, IntCounter, IntGauge,
    IntGaugeVec,
};
use once_cell::sync::Lazy;

/// Number of times the indexer has been unable to fetch a transaction. Ideally zero.
pub static UNABLE_TO_FETCH_TRANSACTION: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "indexer_grpc_unable_to_fetch_transaction_count",
        "Number of times the indexer has been unable to fetch a transaction from storage"
    )
    .unwrap()
});

/// Channel size
pub static CHANNEL_SIZE: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "indexer_grpc_fullnode_channel_size",
        "Channel size for full node",
        &["step"],
    )
    .unwrap()
});

pub static LATENCY_MS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "indexer_grpc_fullnode_latency_ms",
        "Latency of indexer fullnode (comparing with block timestamp).",
    )
    .unwrap()
});
