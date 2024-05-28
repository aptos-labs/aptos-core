// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{register_histogram_vec, register_int_gauge, HistogramVec, IntGauge};
use once_cell::sync::Lazy;

/// Count of the pending messages sent to itself in the channel
pub static PENDING_SELF_MESSAGES: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_jwk_consensus_pending_self_messages",
        "Count of the pending JWK consensus messages sent to itself in the channel"
    )
    .unwrap()
});

pub static OBSERVATION_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_jwk_observation_seconds",
        "JWK observation seconds by issuer and result.",
        &["issuer", "result"]
    )
    .unwrap()
});
