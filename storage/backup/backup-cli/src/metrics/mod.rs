// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use velor_push_metrics::{exponential_buckets, register_histogram_vec, HistogramVec};
use once_cell::sync::Lazy;

pub mod backup;
pub mod metadata;
pub mod restore;
pub mod verify;

pub static OTHER_TIMERS_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        // metric name
        "velor_backup_cli_other_timers_seconds",
        // metric description
        "Various timers for performance analysis.",
        // metric labels (dimensions)
        &["name"],
        exponential_buckets(/*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 22).unwrap(),
    )
    .unwrap()
});
