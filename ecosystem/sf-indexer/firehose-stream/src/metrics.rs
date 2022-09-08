// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{register_int_counter, IntCounter};
use once_cell::sync::Lazy;

pub static TRANSACTIONS_SENT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_fh_stream_transactions_sent_count",
        "Transactions converted and printed out to stdout, picked up by the StreamingFast Firehose indexer",
    )
    .unwrap()
});

pub static BLOCKS_SENT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_fh_stream_blocks_sent_count",
        "Blocks converted and printed out to stdout, picked up by the StreamingFast Firehose indexer",
    )
    .unwrap()
});
