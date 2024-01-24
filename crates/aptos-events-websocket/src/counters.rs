// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{
    register_gauge, register_int_counter, register_int_gauge_vec, Gauge, IntCounter, IntGaugeVec,
};
use once_cell::sync::Lazy;

pub static TRANSACTIONS_STREAMED: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "indexer_grpc_transactions_streamed_count",
        "Transaction streamed via GRPC",
    )
    .unwrap()
});

/// Number of times the indexer has been unable to fetch a transaction. Ideally zero.
pub static UNABLE_TO_FETCH_TRANSACTION: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "indexer_grpc_unable_to_fetch_transaction_count",
        "Number of times the indexer has been unable to fetch a transaction from storage"
    )
    .unwrap()
});

/// Number of times the indexer has been able to fetch a transaction
pub static FETCHED_TRANSACTION: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "indexer_grpc_fetched_transaction_count",
        "Number of times the indexer has been able to fetch a transaction from storage"
    )
    .unwrap()
});

/// Data latency for fullnode fetched the data from storage.
pub static FETCHED_LATENCY_IN_SECS: Lazy<Gauge> = Lazy::new(|| {
    register_gauge!(
        "indexer_grpc_fullnode_fetched_data_latency_in_secs",
        "Latency of fullnode fetched the data from storage",
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
