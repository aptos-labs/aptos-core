// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_infallible::duration_since_epoch;
use aptos_metrics_core::{
    register_histogram, register_histogram_vec, register_int_counter, register_int_counter_vec,
    register_int_gauge, register_int_gauge_vec, Histogram, HistogramVec, IntCounter, IntCounterVec,
    IntGauge, IntGaugeVec,
};
use aptos_short_hex_str::AsShortHexStr;
use move_core_types::account_address::AccountAddress;
use once_cell::sync::Lazy;
use std::time::Duration;

/// Monitor counters, used by monitor! macro
pub static OP_COUNTERS: Lazy<aptos_metrics_core::op_counters::OpMetrics> =
    Lazy::new(|| aptos_metrics_core::op_counters::OpMetrics::new_and_registered("dkg"));

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

#[allow(dead_code)]
pub static CHUNKY_DKG_STAGE_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_chunky_dkg_session_stage_seconds",
        "How long it takes to reach different ChunkyDKG stages",
        &["dealer", "stage"]
    )
    .unwrap()
});

/// Record the time during each stage of DKG, similar to observe_block.
/// Only observes when the elapsed time is non-negative (guards against clock skew).
#[allow(dead_code)]
pub fn observe_dkg_stage(start_time: Duration, my_addr: AccountAddress, stage: &'static str) {
    if let Some(elapsed) = duration_since_epoch().checked_sub(start_time) {
        DKG_STAGE_SECONDS
            .with_label_values(&[my_addr.short_str().as_str(), stage])
            .observe(elapsed.as_secs_f64());
    }
}

pub static DIGEST_KEY_SOURCE: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_dkg_digest_key_source",
        "Which DigestKey source was used at startup (file, test_fallback, none)",
        &["source"]
    )
    .unwrap()
});

pub static DIGEST_KEY_LOAD_DURATION_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "aptos_dkg_digest_key_load_duration_seconds",
        "Time to read and deserialize the DigestKey blob file"
    )
    .unwrap()
});

pub static DIGEST_KEY_FILE_SIZE_BYTES: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_dkg_digest_key_file_size_bytes",
        "Size of the DigestKey blob file in bytes (only set when file exists)"
    )
    .unwrap()
});

pub static PUBLIC_PARAMS_SOURCE: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_dkg_public_params_source",
        "Which PublicParameters source was used at startup (file, test_fallback, none)",
        &["source"]
    )
    .unwrap()
});

pub static PUBLIC_PARAMS_LOAD_DURATION_SECONDS: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "aptos_dkg_public_params_load_duration_seconds",
        "Time to read and deserialize the PublicParameters blob file"
    )
    .unwrap()
});

pub static PUBLIC_PARAMS_FILE_SIZE_BYTES: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_dkg_public_params_file_size_bytes",
        "Size of the PublicParameters blob file in bytes (only set when file exists)"
    )
    .unwrap()
});

pub static CHUNKY_DKG_CONFIG_MODE: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_chunky_dkg_config_mode",
        "Active chunky DKG config mode (0=off, 1=shadow_v1, 2=v1)"
    )
    .unwrap()
});

pub static CHUNKY_DKG_OBJECT_SIZE_BYTES: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_chunky_dkg_object_size_bytes",
        "Serialized size of chunky DKG objects in bytes",
        &["type"],
        // Buckets from 64B to 10MB (powers of 4)
        vec![
            64.0, 256.0, 1024.0, 4096.0, 16384.0, 65536.0, 262144.0, 1048576.0, 4194304.0,
            10485760.0,
        ]
    )
    .unwrap()
});

pub static CHUNKY_DKG_TRANSCRIPT_FETCH_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_chunky_dkg_transcript_fetch_total",
        "Fetch outcomes for missing transcript fetcher",
        &["status"]
    )
    .unwrap()
});

pub static CHUNKY_DKG_SIGNATURE_REQUEST_SKIPPED: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_chunky_dkg_signature_request_skipped",
        "Signature requests skipped because a handler is already in-flight for the same sender"
    )
    .unwrap()
});

/// Record the time during each stage of ChunkyDKG, similar to observe_dkg_stage.
/// Only observes when the elapsed time is non-negative (guards against clock skew).
#[allow(dead_code)]
pub fn observe_chunky_dkg_stage(
    start_time: Duration,
    my_addr: AccountAddress,
    stage: &'static str,
) {
    if let Some(elapsed) = duration_since_epoch().checked_sub(start_time) {
        CHUNKY_DKG_STAGE_SECONDS
            .with_label_values(&[my_addr.short_str().as_str(), stage])
            .observe(elapsed.as_secs_f64());
    }
}
