// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{register_int_counter, IntCounter};
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
