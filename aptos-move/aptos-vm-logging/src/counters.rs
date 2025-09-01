// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::make_local_int_counter;
use once_cell::sync::Lazy;

// Count the number of errors. This is not intended for display on a dashboard,
// but rather for triggering alerts.
make_local_int_counter!(
    pub,
    CRITICAL_ERRORS,
    "aptos_vm_critical_errors",
    "Number of critical errors"
);

// Count the number of errors within the speculative logging logic / implementation.
// Intended to trigger lower priority / urgency alerts.
make_local_int_counter!(
    pub,
    SPECULATIVE_LOGGING_ERRORS,
    "aptos_vm_speculative_logging_errors",
    "Number of errors in speculative logging implementation"
);
