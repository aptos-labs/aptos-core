// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_metrics_core::{
    register_int_counter, register_int_counter_vec, IntCounter, IntCounterVec,
};
use once_cell::sync::Lazy;

/// Number of errors that cache worker has encountered.
pub static ERROR_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "indexer_grpc_cache_worker_errors",
        "Number of errors that cache worker has encountered",
        &["error_type"]
    )
    .unwrap()
});

/// Number of waits that cache worker has encountered for file store.
pub static WAIT_FOR_FILE_STORE_COUNTER: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "indexer_grpc_cache_worker_wait_for_file_store_counter",
        "Number of waits that cache worker has encountered for file store",
    )
    .unwrap()
});
