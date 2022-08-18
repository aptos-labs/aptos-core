// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{register_int_gauge_vec, IntGaugeVec};
use once_cell::sync::Lazy;

//////////////////////
// Tokio Runtime Metrics. Refer to
// https://docs.rs/tokio-metrics/latest/tokio_metrics/struct.RuntimeMetrics.html#fields for more usage
// and explanation of tokio run time metrics
//////////////////////

/// Total amount of time elapsed since observing runtime metrics.
pub static ELAPSED_TIME_MILLIS: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_runtime_elapse_time_millis",
        "Total millis elapsed since observing runtime metrics.",
        &["runtime"]
    )
    .unwrap()
});

/// The number of tasks currently scheduled in the runtime’s injection queue.
pub static INJECTION_QUEUE_DEPTH: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_runtime_injection_queue_depth",
        "The number of tasks currently scheduled in the runtime’s injection queue.",
        &["runtime"]
    )
    .unwrap()
});

/// The number of tasks scheduled from outside of the runtime.
pub static REMOTE_SCHEDULED_COUNT: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_runtime_remote_scheduled_count",
        "The number of tasks scheduled from outside of the runtime.",
        &["runtime"]
    )
    .unwrap()
});

/// The amount of time worker threads were busy.
pub static BUSY_DURATION_MILLIS: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_runtime_busy_duration_millis",
        "The amount of time worker threads were busy.",
        &["runtime"]
    )
    .unwrap()
});

/// The total number of tasks currently scheduled in workers’ local queues.
pub static LOCAL_QUEUE_DEPTH: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_runtime_local_queue_depth",
        "The total number of tasks currently scheduled in workers’ local queues.",
        &["runtime"]
    )
    .unwrap()
});

/// The number of tasks scheduled from worker threads.
pub static LOCAL_SCHEDULED_COUNT: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_runtime_local_scheduled_count",
        "The number of tasks scheduled from worker threads.",
        &["runtime"]
    )
    .unwrap()
});

/// The number of times worker threads unparked but performed no work before parking again.
pub static NOOP_COUNT: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_runtime_noop_count",
        "The number of times worker threads unparked but performed no work before parking again.",
        &["runtime"]
    )
    .unwrap()
});

/// The number of times worker threads saturated their local queues.
pub static OVERFLOW_COUNT: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_runtime_overflow_count",
        "The number of times worker threads saturated their local queues.",
        &["runtime"]
    )
    .unwrap()
});

/// The number of times worker threads parked.
pub static PARKED_COUNT: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_runtime_parked_count",
        "The number of times worker threads parked.",
        &["runtime"]
    )
    .unwrap()
});

/// The number of tasks that have been polled across all worker threads.
pub static POLLS_COUNT: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_runtime_polls_count",
        "The number of tasks that have been polled across all worker threads.",
        &["runtime"]
    )
    .unwrap()
});

/// The number of worker threads used by the runtime.
pub static WORKERS_COUNT: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_runtime_worker_count",
        "The number of worker threads used by the runtime",
        &["runtime"]
    )
    .unwrap()
});
