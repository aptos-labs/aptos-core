// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_metrics_core::{register_histogram_vec, register_int_gauge, HistogramVec, IntGauge};
use once_cell::sync::Lazy;

/// Count of the pending messages sent to itself in the channel
pub static PENDING_SELF_MESSAGES: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "velor_dkg_pending_self_messages",
        "Count of the pending messages sent to itself in the channel"
    )
    .unwrap()
});

pub static DKG_STAGE_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "velor_dkg_session_stage_seconds",
        "How long it takes to reach different DKG stages",
        &["dealer", "stage"]
    )
    .unwrap()
});

pub static ROUNDING_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "velor_dkg_rounding_seconds",
        "Rounding seconds and counts by method",
        &["method"]
    )
    .unwrap()
});
