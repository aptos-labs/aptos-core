// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::constants::GCP_SERVICE_PROJECT_ID_ENV;
use std::env;

pub fn gcp_trace_id() -> Option<String> {
    let current_span = tracing::Span::current();
    current_span
        .field("trace_id")
        .zip(env::var(GCP_SERVICE_PROJECT_ID_ENV).ok())
        .map(|(trace_id, project_id)| format!("projects/{}/traces/{}", project_id, trace_id))
}

/// Log at the `trace` level
#[macro_export]
macro_rules! trace {
    ($($arg:tt)+) => {
        $crate::log!(velor_logger::Level::Trace, $($arg)+)
    };
}

/// Log at the `debug` level
#[macro_export]
macro_rules! debug {
    ($($arg:tt)+) => {
        $crate::log!(velor_logger::Level::Debug, $($arg)+)
    };
}

/// Log at the `info` level
#[macro_export]
macro_rules! info {
    ($($arg:tt)+) => {
        $crate::log!(velor_logger::Level::Info, $($arg)+)
    };
}

/// Log at the `warn` level
#[macro_export]
macro_rules! warn {
    ($($arg:tt)+) => {
        $crate::log!(velor_logger::Level::Warn, $($arg)+)
    };
}

/// Log at the `error` level
#[macro_export]
macro_rules! error {
    ($($arg:tt)+) => {
        $crate::log!(velor_logger::Level::Error, $($arg)+)
    };
}

/// Log at the given level, it's recommended to use a specific level macro instead
#[macro_export]
macro_rules! log {
    // Entry, Log Level + stuff
    ($level:expr, $($args:tt)+) => {{
        if let Some(trace_id) = $crate::gcp_logger::gcp_trace_id() {
            velor_logger::log!($level, "logging.googleapis.com/trace"=%trace_id, $($args)+)
        } else {
            velor_logger::log!($level, $($args)+)
        }
    }};
}
