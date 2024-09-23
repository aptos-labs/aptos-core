// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{
    register_gauge_vec, register_int_counter_vec, register_int_gauge_vec, GaugeVec, IntCounterVec,
    IntGaugeVec,
};
use once_cell::sync::Lazy;

// The `identifier` label at the time of writing (2024-04-08) is always the
// application ID, a globally unique ID.

/// Latest processed transaction version.
pub static LATEST_PROCESSED_VERSION_PER_PROCESSOR: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "indexer_grpc_data_service_with_user_latest_processed_version",
        "Latest processed transaction version",
        &[
            "identifier_type",
            "identifier",
            "email",
            "application_name",
            "processor"
        ],
    )
    .unwrap()
});

/// Number of transactions that served by data service.
pub static PROCESSED_VERSIONS_COUNT_PER_PROCESSOR: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "indexer_grpc_data_service_with_user_processed_versions",
        "Number of transactions that have been processed by data service",
        &[
            "identifier_type",
            "identifier",
            "email",
            "application_name",
            "processor"
        ],
    )
    .unwrap()
});

/// Number of errors that data service has encountered.
pub static ERROR_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "indexer_grpc_data_service_error",
        "Number of errors that data service has encountered",
        &["error_type"]
    )
    .unwrap()
});

/// Data latency for data service based on latest processed transaction based on selected processor.
pub static PROCESSED_LATENCY_IN_SECS_PER_PROCESSOR: Lazy<GaugeVec> = Lazy::new(|| {
    register_gauge_vec!(
        "indexer_grpc_data_service_with_user_latest_data_latency_in_secs",
        "Latency of data service based on latest processed transaction",
        &[
            "identifier_type",
            "identifier",
            "email",
            "application_name",
            "processor"
        ],
    )
    .unwrap()
});

/// Count of connections that data service has established.
pub static CONNECTION_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "indexer_grpc_data_service_connection_count_v2",
        "Count of connections that data service has established",
        &[
            "identifier_type",
            "identifier",
            "email",
            "application_name",
            "processor"
        ],
    )
    .unwrap()
});

/// Count of the short connections; i.e., < 10 seconds.
pub static SHORT_CONNECTION_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "indexer_grpc_data_service_short_connection_by_user_processor_count",
        "Count of the short connections; i.e., < 10 seconds",
        &[
            "identifier_type",
            "identifier",
            "email",
            "application_name",
            "processor"
        ],
    )
    .unwrap()
});

/// Count of bytes transfered to the client. This only represents the bytes prepared and
/// ready to send to the client. This only t It does not represent the bytes actually
/// sent to the client.
///
/// This is pre stripping, so it may include bytes for transactions that were later
/// stripped. See BYTES_READY_TO_TRANSFER_FROM_SERVER_AFTER_STRIPPING for post
/// stirpping.
pub static BYTES_READY_TO_TRANSFER_FROM_SERVER: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "indexer_grpc_data_service_bytes_ready_to_transfer_from_server",
        "Count of bytes ready to transfer to the client (pre stripping)",
        &[
            "identifier_type",
            "identifier",
            "email",
            "application_name",
            "processor"
        ],
    )
    .unwrap()
});

/// Count of bytes transfered to the client. This only represents the bytes prepared and
/// ready to send to the client. This only t It does not represent the bytes actually
/// sent to the client.
///
/// This is post stripping, meaning some transactions may have been stripped (removing
/// things such as events, writesets, payload, signature). Compare this with
/// BYTES_READY_TO_TRANSFER_FROM_SERVER to see how many bytes were stripped.
pub static BYTES_READY_TO_TRANSFER_FROM_SERVER_AFTER_STRIPPING: Lazy<IntCounterVec> =
    Lazy::new(|| {
        register_int_counter_vec!(
            "indexer_grpc_data_service_bytes_ready_to_transfer_from_server_after_stripping",
            "Count of bytes ready to transfer to the client (post stripping)",
            &[
                "identifier_type",
                "identifier",
                "email",
                "application_name",
                "processor"
            ],
        )
        .unwrap()
    });

/// The number of transactions that had data (such as events, writesets, payload,
/// signature) stripped from them due to the `txns_to_strip_filter`. See
/// `strip_transactions` for more.
pub static NUM_TRANSACTIONS_STRIPPED: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "indexer_grpc_data_service_num_transactions_stripped",
        "Number of transactions that had data (such as events, writesets, payload, signature) stripped from them",
        &[
            "identifier_type",
            "identifier",
            "email",
            "application_name",
            "processor"
        ],
    )
    .unwrap()
});
