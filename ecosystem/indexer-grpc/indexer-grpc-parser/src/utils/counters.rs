// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{
    register_gauge_vec, register_int_counter, register_int_counter_vec, register_int_gauge_vec,
    GaugeVec, IntCounter, IntCounterVec, IntGaugeVec,
};
use once_cell::sync::Lazy;

/// Data latency when processor receives transactions.
pub static PROCESSOR_DATA_RECEIVED_LATENCY_IN_SECS: Lazy<GaugeVec> = Lazy::new(|| {
    register_gauge_vec!(
        "indexer_processor_data_receive_latency_in_secs",
        "Data latency when processor receives transactions",
        &["request_token", "processor_name"]
    )
    .unwrap()
});

/// Data latency when processor finishes processing transactions.
pub static PROCESSOR_DATA_PROCESSED_LATENCY_IN_SECS: Lazy<GaugeVec> = Lazy::new(|| {
    register_gauge_vec!(
        "indexer_processor_data_processed_latency_in_secs",
        "Data latency when processor finishes processing transactions",
        &["request_token", "processor_name"]
    )
    .unwrap()
});

/// Number of times a given processor has been invoked
pub static PROCESSOR_INVOCATIONS_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "indexer_processor_invocation_count",
        "Number of times a given processor has been invoked",
        &["processor_name"]
    )
    .unwrap()
});

/// Number of times any given processor has raised an error
pub static PROCESSOR_ERRORS_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "indexer_processor_errors",
        "Number of times any given processor has raised an error",
        &["processor_name"]
    )
    .unwrap()
});

/// Number of times any given processor has completed successfully
pub static PROCESSOR_SUCCESSES_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "indexer_processor_success_count",
        "Number of times a given processor has completed successfully",
        &["processor_name"]
    )
    .unwrap()
});

/// Number of times the connection pool has timed out when trying to get a connection
pub static UNABLE_TO_GET_CONNECTION_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "indexer_connection_pool_err",
        "Number of times the connection pool has timed out when trying to get a connection"
    )
    .unwrap()
});

/// Number of times the connection pool got a connection
pub static GOT_CONNECTION_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "indexer_connection_pool_ok",
        "Number of times the connection pool got a connection"
    )
    .unwrap()
});

#[allow(dead_code)]
/// Number of times the indexer has been unable to fetch a transaction. Ideally zero.
pub static UNABLE_TO_FETCH_TRANSACTION: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "indexer_unable_to_fetch_transaction_count",
        "Number of times the indexer has been unable to fetch a transaction"
    )
    .unwrap()
});

#[allow(dead_code)]
/// Number of times the indexer has been able to fetch a transaction
pub static FETCHED_TRANSACTION: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "indexer_fetched_transaction_count",
        "Number of times the indexer has been able to fetch a transaction"
    )
    .unwrap()
});

/// Max version processed
pub static LATEST_PROCESSED_VERSION: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "indexer_processor_latest_version",
        "Latest version a processor has fully consumed",
        &["processor_name"]
    )
    .unwrap()
});
