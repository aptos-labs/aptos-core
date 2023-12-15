// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{register_int_counter, register_int_gauge, IntCounter, IntGauge};
use once_cell::sync::Lazy;

/// Latest version of transactions that have been stored.
pub static LATEST_PROCESSED_VERSION: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "indexer_grpc_file_store_latest_processed_version",
        "Latest version of transactions that have been stored",
    )
    .unwrap()
});

/// Number of transactions that have been stored.
pub static PROCESSED_VERSIONS_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "indexer_grpc_file_store_processed_versions",
        "Number of transactions that have been stored",
    )
    .unwrap()
});

/// Number of errors that file store has encountered.
pub static ERROR_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "indexer_grpc_file_store_errors",
        "Number of errors that file store has encountered"
    )
    .unwrap()
});

/// Number of metadata upload failures that file store has encountered.
pub static METADATA_UPLOAD_FAILURE_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "indexer_grpc_file_store_metadata_upload_failures",
        "Number of metadata upload failures that file store has encountered"
    )
    .unwrap()
});
