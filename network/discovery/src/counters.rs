// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use velor_metrics_core::{
    op_counters::DurationHistogram, register_histogram, register_int_counter_vec,
    register_int_gauge_vec, IntCounterVec, IntGaugeVec,
};
use once_cell::sync::Lazy;

/// Histogram of busy time of spent in event processing loop
pub static EVENT_PROCESSING_LOOP_BUSY_DURATION_S: Lazy<DurationHistogram> = Lazy::new(|| {
    DurationHistogram::new(
        register_histogram!(
            "simple_onchain_discovery_event_processing_loop_busy_duration_s",
            "Histogram of busy time of spent in event processing loop"
        )
        .unwrap(),
    )
});

pub static DISCOVERY_COUNTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_simple_onchain_discovery_counts",
        "Histogram of busy time of spent in event processing loop",
        &["role_type", "network_id", "peer_id", "metric"]
    )
    .unwrap()
});

pub static NETWORK_KEY_MISMATCH: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "velor_network_key_mismatch",
        "Gauge of whether the network key mismatches onchain state",
        &["role_type", "network_id", "peer_id"]
    )
    .unwrap()
});
