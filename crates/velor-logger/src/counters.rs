// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Logging metrics for determining quality of log submission
use once_cell::sync::Lazy;
use prometheus::{register_int_counter, IntCounter};

/// Count of the struct logs submitted by macro
pub static STRUCT_LOG_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!("velor_struct_log_count", "Count of the struct logs.").unwrap()
});

/// Count of struct logs processed, but not necessarily sent
pub static PROCESSED_STRUCT_LOG_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "velor_struct_log_processed_count",
        "Count of the struct logs received by the sender."
    )
    .unwrap()
});

/// Counts of logs
pub static ERROR_LOG_COUNT: Lazy<IntCounter> =
    Lazy::new(|| register_int_counter!("velor_error_log_count", "Count of error!() logs").unwrap());
pub static WARN_LOG_COUNT: Lazy<IntCounter> =
    Lazy::new(|| register_int_counter!("velor_warn_log_count", "Count of warn!() logs").unwrap());
pub static INFO_LOG_COUNT: Lazy<IntCounter> =
    Lazy::new(|| register_int_counter!("velor_info_log_count", "Count of info!() logs").unwrap());

/// Metric for when we fail to log during sending to the queue
pub static STRUCT_LOG_QUEUE_ERROR_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "velor_struct_log_queue_error_count",
        "Count of all errors during queuing struct logs."
    )
    .unwrap()
});

pub static STRUCT_LOG_PARSE_ERROR_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "velor_struct_log_parse_error_count",
        "Count of all parse errors during struct logs."
    )
    .unwrap()
});

/// Counter for failed log ingest writes (see also: velor-telemetry for sender metrics)
pub static VELOR_LOG_INGEST_WRITER_FULL: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "velor_log_ingest_writer_full",
        "Number of log ingest writes that failed due to channel full"
    )
    .unwrap()
});

/// Counter for failed log ingest writes (see also: velor-telemetry for sender metrics)
pub static VELOR_LOG_INGEST_WRITER_DISCONNECTED: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "velor_log_ingest_writer_disconnected",
        "Number of log ingest writes that failed due to channel disconnected"
    )
    .unwrap()
});
