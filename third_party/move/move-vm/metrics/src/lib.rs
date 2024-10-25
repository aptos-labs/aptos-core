// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use once_cell::sync::Lazy;
use prometheus::{register_histogram_vec, HistogramTimer, HistogramVec};

pub trait Timer {
    fn timer_with_label(&self, label: &str) -> HistogramTimer;
}

impl Timer for HistogramVec {
    fn timer_with_label(&self, label: &str) -> HistogramTimer {
        self.with_label_values(&[label]).start_timer()
    }
}

pub static VM_TIMER: Lazy<HistogramVec> = Lazy::new(|| {
    let factor = 2.0;
    let num_buckets = 32;

    let mut next = 1e-9;
    let mut buckets = Vec::with_capacity(num_buckets);
    for _ in 0..num_buckets {
        buckets.push(next);
        next *= factor;
    }

    register_histogram_vec!(
        // Metric name:
        "vm_timer_seconds",
        // Metric description:
        "VM timers",
        &["name"],
        buckets,
    )
    .unwrap()
});
