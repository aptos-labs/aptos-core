// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_metrics_core::{register_int_counter, IntCounter};
use once_cell::sync::Lazy;

/// Count the number of errors. This is not intended for display on a dashboard,
/// but rather for triggering alerts.
pub static CRITICAL_ERRORS: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!("aptos_vm_critical_errors", "Number of critical errors").unwrap()
});

/// Count the number of errors within the speculative logging logic / implementation.
/// Intended to trigger lower priority / urgency alerts.
pub static SPECULATIVE_LOGGING_ERRORS: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_vm_speculative_logging_errors",
        "Number of errors in speculative logging implementation"
    )
    .unwrap()
});
