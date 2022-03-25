// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics::{
    register_histogram_vec, register_int_counter_vec, HistogramVec, IntCounterVec,
};
use once_cell::sync::Lazy;

pub static APTOS_SCHEMADB_ITER_LATENCY_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        // metric name
        "aptos_schemadb_iter_latency_seconds",
        // metric description
        "Aptos schemadb iter latency in seconds",
        // metric labels (dimensions)
        &["cf_name"]
    )
    .unwrap()
});

pub static APTOS_SCHEMADB_ITER_BYTES: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        // metric name
        "aptos_schemadb_iter_bytes",
        // metric description
        "Aptos schemadb iter size in bytess",
        // metric labels (dimensions)
        &["cf_name"]
    )
    .unwrap()
});

pub static APTOS_SCHEMADB_GET_LATENCY_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        // metric name
        "aptos_schemadb_get_latency_seconds",
        // metric description
        "Aptos schemadb get latency in seconds",
        // metric labels (dimensions)
        &["cf_name"]
    )
    .unwrap()
});

pub static APTOS_SCHEMADB_GET_BYTES: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        // metric name
        "aptos_schemadb_get_bytes",
        // metric description
        "Aptos schemadb get call returned data size in bytes",
        // metric labels (dimensions)
        &["cf_name"]
    )
    .unwrap()
});

pub static APTOS_SCHEMADB_BATCH_COMMIT_LATENCY_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        // metric name
        "aptos_schemadb_batch_commit_latency_seconds",
        // metric description
        "Aptos schemadb schema batch commit latency in seconds",
        // metric labels (dimensions)
        &["db_name"]
    )
    .unwrap()
});

pub static APTOS_SCHEMADB_BATCH_COMMIT_BYTES: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        // metric name
        "aptos_schemadb_batch_commit_bytes",
        // metric description
        "Aptos schemadb schema batch commit size in bytes",
        // metric labels (dimensions)
        &["db_name"]
    )
    .unwrap()
});

pub static APTOS_SCHEMADB_PUT_BYTES: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        // metric name
        "aptos_schemadb_put_bytes",
        // metric description
        "Aptos schemadb put call puts data size in bytes",
        // metric labels (dimensions)
        &["cf_name"]
    )
    .unwrap()
});

pub static APTOS_SCHEMADB_DELETES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_storage_deletes",
        "Aptos storage delete calls",
        &["cf_name"]
    )
    .unwrap()
});

pub static APTOS_SCHEMADB_RANGE_DELETES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_storage_range_deletes",
        "Aptos storage range delete calls",
        &["cf_name"]
    )
    .unwrap()
});

pub static APTOS_SCHEMADB_INCLUSIVE_RANGE_DELETES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_storage_range_inclusive_deletes",
        "Aptos storage range inclusive delete calls",
        &["cf_name"]
    )
    .unwrap()
});

pub static APTOS_SCHEMADB_BATCH_PUT_LATENCY_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        // metric name
        "aptos_schemadb_batch_put_latency_seconds",
        // metric description
        "Aptos schemadb schema batch put latency in seconds",
        // metric labels (dimensions)
        &["db_name"]
    )
    .unwrap()
});
