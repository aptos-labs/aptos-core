// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{register_int_gauge, IntGauge};
use once_cell::sync::Lazy;

pub static INDEXER_DB_LATENCY: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_internal_indexer_latency",
        "The latency between main db update and data written to indexer db"
    )
    .unwrap()
});
