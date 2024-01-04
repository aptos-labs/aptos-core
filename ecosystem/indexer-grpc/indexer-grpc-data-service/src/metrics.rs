// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{register_int_counter_vec, IntCounterVec};
use once_cell::sync::Lazy;

/// Count of connections that data service has established.
pub static CONNECTION_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "indexer_grpc_data_service_connection_count_v2",
        "Count of connections that data service has established",
        &["request_token", "email", "processor"],
    )
    .unwrap()
});

/// Count of the short connections; i.e., < 10 seconds.
pub static SHORT_CONNECTION_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "indexer_grpc_data_service_short_connection_by_user_processor_count",
        "Count of the short connections; i.e., < 10 seconds",
        &["request_token", "email", "processor"],
    )
    .unwrap()
});
