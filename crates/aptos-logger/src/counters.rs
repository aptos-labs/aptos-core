// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Logging metrics for determining quality of log submission
use once_cell::sync::Lazy;
use prometheus::{register_int_counter, IntCounter};

/// Count of the struct logs submitted by macro
pub static STRUCT_LOG_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!("aptos_struct_log_count", "Count of the struct logs.").unwrap()
});

/// Metric for when we fail to log during sending to the queue
pub static STRUCT_LOG_QUEUE_ERROR_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_struct_log_queue_error_count",
        "Count of all errors during queuing struct logs."
    )
    .unwrap()
});

pub static STRUCT_LOG_PARSE_ERROR_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_struct_log_parse_error_count",
        "Count of all parse errors during struct logs."
    )
    .unwrap()
});
