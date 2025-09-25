// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use once_cell::sync::Lazy;
use prometheus::{
    register_histogram_vec, register_int_gauge, HistogramTimer, HistogramVec, IntGauge,
};

/// Helper trait to encapsulate [HistogramVec] functionality. Users can use this trait to time
/// different VM parts collecting metrics for different labels. Use wisely as timers do introduce
/// an overhead, so using on a hot path is not recommended.
pub trait Timer {
    /// Returns a new timer for the specified label.
    fn timer_with_label(&self, label: &str) -> HistogramTimer;
}

impl Timer for HistogramVec {
    #[inline]
    fn timer_with_label(&self, label: &str) -> HistogramTimer {
        self.with_label_values(&[label]).start_timer()
    }
}

/// Timer that can be used to instrument the VM to collect metrics for different parts of the code.
/// To access and view the metrics, set up where to send them, e.g., `PUSH_METRICS_NAMESPACE` and
/// `PUSH_METRICS_ENDPOINT`. Then, metrics can be seen on Grafana dashboard, for instance.
///
/// Note: the timer uses "exponential" buckets with a factor of 2.
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
    .expect("Registering the histogram should always succeed")
});

pub static VERIFIED_MODULE_CACHE_SIZE: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "verified_module_cache_size",
        "Number of modules stored in verified module cache"
    )
    .expect("Registering the counter should always succeed")
});
