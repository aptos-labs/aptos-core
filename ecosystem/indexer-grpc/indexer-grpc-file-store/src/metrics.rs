// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_metrics_core::{register_int_counter, IntCounter};
use once_cell::sync::Lazy;

/// Number of transactions that have been stored.
pub static PROCESSED_VERSIONS_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "indexer_grpc_file_store_processed_versions",
        "Number of transactions that have been stored",
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
