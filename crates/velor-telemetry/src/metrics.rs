// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_metrics_core::{
    register_int_counter, register_int_counter_vec, IntCounter, IntCounterVec,
};
use once_cell::sync::Lazy;

/// Counter for successful telemetry events sent from Telemetry Sender to Telemetry Service
pub(crate) static VELOR_TELEMETRY_SERVICE_SUCCESS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_telemetry_service_success",
        "Number of telemetry events successfully sent to telemetry service",
        &["event_name"]
    )
    .unwrap()
});

/// Counter for failed telemetry events sent from Telemetry Sender to Telemetry Service
pub(crate) static VELOR_TELEMETRY_SERVICE_FAILURE: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_telemetry_service_failure",
        "Number of telemetry events that failed to send to telemetry service",
        &["event_name"]
    )
    .unwrap()
});

/// Counter for successful telemetry events sent to GA
/// /// TODO: Clean up when cleaning up telemetry exporter to GA
pub(crate) static VELOR_TELEMETRY_SUCCESS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_telemetry_success",
        "Number of telemetry events successfully sent",
        &["event_name"]
    )
    .unwrap()
});

/// Counter for failed telemetry events sent to GA
/// TODO: Clean up when cleaning up telemetry exporter to GA
pub(crate) static VELOR_TELEMETRY_FAILURE: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_telemetry_failure",
        "Number of telemetry events that failed to send",
        &["event_name"]
    )
    .unwrap()
});

/// Increments the number of successful telemetry events sent to GA
pub(crate) fn increment_telemetry_successes(event_name: &str) {
    VELOR_TELEMETRY_SUCCESS
        .with_label_values(&[event_name])
        .inc();
}

/// Increments the number of failed telemetry events sent to GA
pub(crate) fn increment_telemetry_failures(event_name: &str) {
    VELOR_TELEMETRY_FAILURE
        .with_label_values(&[event_name])
        .inc();
}

/// Increments the number of successful telemetry events sent to Telemetry service
pub(crate) fn increment_telemetry_service_successes(event_name: &str) {
    VELOR_TELEMETRY_SERVICE_SUCCESS
        .with_label_values(&[event_name])
        .inc();
}

/// Increments the number of failed telemetry events sent to Telemetry service
pub(crate) fn increment_telemetry_service_failures(event_name: &str) {
    VELOR_TELEMETRY_SERVICE_FAILURE
        .with_label_values(&[event_name])
        .inc();
}

/// Counter for successful log ingest events sent to Telemetry Service
pub(crate) static VELOR_LOG_INGEST_SUCCESS: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "velor_log_ingest_success",
        "Number of log ingest events successfully sent"
    )
    .unwrap()
});

/// Counter for successful log ingest events sent to Telemetry Service
pub(crate) static VELOR_LOG_INGEST_TOO_LARGE: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "velor_log_ingest_too_large",
        "Number of log ingest events that were too large"
    )
    .unwrap()
});

/// Counter for failed log ingest events sent to Telemetry Service
pub(crate) static VELOR_LOG_INGEST_FAILURE: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "velor_log_ingest_failure",
        "Number of log ingest events that failed to send"
    )
    .unwrap()
});

/// Increments the number of successful log ingest events sent to Telemetry Service
pub(crate) fn increment_log_ingest_successes_by(v: u64) {
    VELOR_LOG_INGEST_SUCCESS.inc_by(v);
}

/// Increments the number of ignored log ingest events because too large
pub(crate) fn increment_log_ingest_too_large_by(v: u64) {
    VELOR_LOG_INGEST_TOO_LARGE.inc_by(v);
}

/// Increments the number of failed log ingest events
pub(crate) fn increment_log_ingest_failures_by(v: u64) {
    VELOR_LOG_INGEST_FAILURE.inc_by(v);
}
