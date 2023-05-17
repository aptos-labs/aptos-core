// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{register_gauge_vec, GaugeVec};
use once_cell::sync::Lazy;

/// "Latest observed transaction timestamp vs current timestamp.
/// Node type can be "pfn" or "indexer".
pub static OBSERVED_LATEST_TRANSACTION_LATENCY: Lazy<GaugeVec> = Lazy::new(|| {
    register_gauge_vec!(
        "indexer_grpc_post_processor_observed_transaction_latency_in_secs",
        "Latest observed transaction timestamp vs current timestamp.",
        &["node_type"],
    )
    .unwrap()
});
