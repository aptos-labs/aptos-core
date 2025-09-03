// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{
    exponential_buckets, make_thread_local_histogram_vec, register_int_counter_vec, IntCounterVec,
};
use once_cell::sync::Lazy;

make_thread_local_histogram_vec!(
    pub,
    APTOS_SCHEMADB_SEEK_LATENCY_SECONDS,
    // metric name
    "aptos_schemadb_seek_latency_seconds",
    // metric description
    "Aptos schemadb seek latency in seconds",
    // metric labels (dimensions)
    &["cf_name", "tag"],
    exponential_buckets(/*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 22).unwrap(),
);

make_thread_local_histogram_vec!(
    pub,
    APTOS_SCHEMADB_ITER_LATENCY_SECONDS,
    // metric name
    "aptos_schemadb_iter_latency_seconds",
    // metric description
    "Aptos schemadb iter latency in seconds",
    // metric labels (dimensions)
    &["cf_name"],
    exponential_buckets(/*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 22).unwrap(),
);

make_thread_local_histogram_vec!(
    pub,
    APTOS_SCHEMADB_ITER_BYTES,
    // metric name
    "aptos_schemadb_iter_bytes",
    // metric description
    "Aptos schemadb iter size in bytes",
    // metric labels (dimensions)
    &["cf_name"]
);

make_thread_local_histogram_vec!(
    pub,
    APTOS_SCHEMADB_GET_LATENCY_SECONDS,
    // metric name
    "aptos_schemadb_get_latency_seconds",
    // metric description
    "Aptos schemadb get latency in seconds",
    // metric labels (dimensions)
    &["cf_name"],
    exponential_buckets(/*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 22).unwrap(),
);

make_thread_local_histogram_vec!(
    pub,
    APTOS_SCHEMADB_GET_BYTES,
    // metric name
    "aptos_schemadb_get_bytes",
    // metric description
    "Aptos schemadb get call returned data size in bytes",
    // metric labels (dimensions)
    &["cf_name"]
);

make_thread_local_histogram_vec!(
    pub,
    APTOS_SCHEMADB_BATCH_COMMIT_LATENCY_SECONDS,
    // metric name
    "aptos_schemadb_batch_commit_latency_seconds",
    // metric description
    "Aptos schemadb schema batch commit latency in seconds",
    // metric labels (dimensions)
    &["db_name"],
    exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
);

make_thread_local_histogram_vec!(
    pub,
    APTOS_SCHEMADB_BATCH_COMMIT_BYTES,
    // metric name
    "aptos_schemadb_batch_commit_bytes",
    // metric description
    "Aptos schemadb schema batch commit size in bytes",
    // metric labels (dimensions)
    &["db_name"]
);

make_thread_local_histogram_vec!(
    pub,
    APTOS_SCHEMADB_PUT_BYTES_SAMPLED,
    // metric name
    "aptos_schemadb_put_bytes_sampled",
    // metric description
    "Aptos schemadb put call puts data size in bytes (sampled)",
    // metric labels (dimensions)
    &["cf_name"]
);

pub static APTOS_SCHEMADB_DELETES_SAMPLED: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_storage_deletes_sampled",
        "Aptos storage delete calls (sampled)",
        &["cf_name"]
    )
    .unwrap()
});

make_thread_local_histogram_vec!(
    pub,
    TIMER,
    "aptos_schema_db_timer_seconds",
    "Various timers for performance analysis.",
    &["name", "sub_name"],
    exponential_buckets(/*start=*/ 1e-9, /*factor=*/ 2.0, /*count=*/ 32).unwrap(),
);
