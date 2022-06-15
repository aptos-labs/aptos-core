// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{register_int_counter_vec, IntCounterVec};
use once_cell::sync::Lazy;

/// Counter for successful telemetry events
pub(crate) static APTOS_TELEMETRY_SUCCESS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_telemetry_success",
        "Number of telemetry events successfully sent",
        &["event_name"]
    )
    .unwrap()
});

/// Counter for failed telemetry events
pub(crate) static APTOS_TELEMETRY_FAILURE: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_telemetry_failure",
        "Number of telemetry events that failed to send",
        &["event_name"]
    )
    .unwrap()
});

/// Increments the number of successful telemetry events
pub(crate) fn increment_telemetry_successes(event_name: &str) {
    APTOS_TELEMETRY_SUCCESS
        .with_label_values(&[event_name])
        .inc();
}

/// Increments the number of failed telemetry events
pub(crate) fn increment_telemetry_failures(event_name: &str) {
    APTOS_TELEMETRY_FAILURE
        .with_label_values(&[event_name])
        .inc();
}
