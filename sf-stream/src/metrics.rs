// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{register_int_counter, IntCounter};
use once_cell::sync::Lazy;

pub static TRANSACTIONS_SENT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_sf_stream_transactions_sent_count",
        "Transactions pushed out to the SF Stream",
    )
    .unwrap()
});
