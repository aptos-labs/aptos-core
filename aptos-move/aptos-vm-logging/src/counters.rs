// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{register_int_counter, IntCounter};
use once_cell::sync::Lazy;

/// Count the number of errors. This is not intended for display on a dashboard,
/// but rather for triggering alerts.
pub static CRITICAL_ERRORS: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!("aptos_vm_critical_errors", "Number of critical errors").unwrap()
});
