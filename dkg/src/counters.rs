// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_infallible::duration_since_epoch;
use aptos_metrics_core::{register_histogram_vec, register_int_gauge, HistogramVec, IntGauge};
use aptos_short_hex_str::AsShortHexStr;
use move_core_types::account_address::AccountAddress;
use once_cell::sync::Lazy;
use std::time::Duration;

/// Count of the pending messages sent to itself in the channel
pub static PENDING_SELF_MESSAGES: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_dkg_pending_self_messages",
        "Count of the pending messages sent to itself in the channel"
    )
    .unwrap()
});

pub static DKG_STAGE_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_dkg_session_stage_seconds",
        "How long it takes to reach different DKG stages",
        &["dealer", "stage"]
    )
    .unwrap()
});

pub static ROUNDING_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_dkg_rounding_seconds",
        "Rounding seconds and counts by method",
        &["method"]
    )
    .unwrap()
});

pub static CHUNKY_DKG_STAGE_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_chunky_dkg_session_stage_seconds",
        "How long it takes to reach different ChunkyDKG stages",
        &["dealer", "stage"]
    )
    .unwrap()
});

/// Record the time during each stage of DKG, similar to observe_block.
#[allow(dead_code)]
pub fn observe_dkg_stage(start_time: Duration, my_addr: AccountAddress, stage: &'static str) {
    let secs_since_dkg_start = duration_since_epoch().as_secs_f64() - start_time.as_secs_f64();
    DKG_STAGE_SECONDS
        .with_label_values(&[my_addr.short_str().as_str(), stage])
        .observe(secs_since_dkg_start);
}

/// Record the time during each stage of ChunkyDKG, similar to observe_dkg_stage.
pub fn observe_chunky_dkg_stage(
    start_time: Duration,
    my_addr: AccountAddress,
    stage: &'static str,
) {
    let secs_since_dkg_start = duration_since_epoch().as_secs_f64() - start_time.as_secs_f64();
    CHUNKY_DKG_STAGE_SECONDS
        .with_label_values(&[my_addr.short_str().as_str(), stage])
        .observe(secs_since_dkg_start);
}
