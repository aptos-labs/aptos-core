// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::counters::{
    BUSY_DURATION_MILLIS, ELAPSED_TIME_MILLIS, INJECTION_QUEUE_DEPTH, LOCAL_QUEUE_DEPTH,
    LOCAL_SCHEDULED_COUNT, NOOP_COUNT, OVERFLOW_COUNT, PARKED_COUNT, POLLS_COUNT,
    REMOTE_SCHEDULED_COUNT, WORKERS_COUNT,
};
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio_metrics::RuntimeMonitor;

/// Instruments the provided tokio run time by spawning a periodic task on the runtime itself.
pub fn instrument_tokio_runtime(runtime: &Runtime, name: &'static str) {
    let runtime_monitor = RuntimeMonitor::new(runtime.handle());
    runtime.spawn(async move {
        for metrics in runtime_monitor.intervals() {
            ELAPSED_TIME_MILLIS
                .with_label_values(&[name])
                .set(metrics.elapsed.as_millis() as i64);
            INJECTION_QUEUE_DEPTH
                .with_label_values(&[name])
                .set(metrics.injection_queue_depth as i64);
            REMOTE_SCHEDULED_COUNT
                .with_label_values(&[name])
                .set(metrics.num_remote_schedules as i64);
            BUSY_DURATION_MILLIS
                .with_label_values(&[name])
                .set(metrics.total_busy_duration.as_millis() as i64);
            LOCAL_QUEUE_DEPTH
                .with_label_values(&[name])
                .set(metrics.min_local_queue_depth as i64);
            LOCAL_SCHEDULED_COUNT
                .with_label_values(&[name])
                .set(metrics.total_local_schedule_count as i64);
            NOOP_COUNT
                .with_label_values(&[name])
                .set(metrics.total_noop_count as i64);
            OVERFLOW_COUNT
                .with_label_values(&[name])
                .set(metrics.total_overflow_count as i64);
            PARKED_COUNT
                .with_label_values(&[name])
                .set(metrics.total_park_count as i64);
            POLLS_COUNT
                .with_label_values(&[name])
                .set(metrics.total_polls_count as i64);
            WORKERS_COUNT
                .with_label_values(&[name])
                .set(metrics.workers_count as i64);
            // Sleep for 1s before collecting the next batch of metrics
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });
}
