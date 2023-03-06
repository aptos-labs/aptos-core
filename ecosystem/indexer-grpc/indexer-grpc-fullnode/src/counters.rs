// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{register_int_counter, IntCounter};
use once_cell::sync::Lazy;

pub static TRANSACTIONS_SENT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_grpc_stream_transactions_sent_count",
        "Transactions converted and printed out to stdout, picked up by the StreamingFast Firehose indexer",
    )
    .unwrap()
});

/// Number of times the indexer has been unable to fetch a transaction. Ideally zero.
pub static UNABLE_TO_FETCH_TRANSACTION: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "indexer_unable_to_fetch_transaction_count",
        "Number of times the indexer has been unable to fetch a transaction"
    )
    .unwrap()
});

/// Number of times the indexer has been able to fetch a transaction
pub static FETCHED_TRANSACTION: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "indexer_fetched_transaction_count",
        "Number of times the indexer has been able to fetch a transaction"
    )
    .unwrap()
});
