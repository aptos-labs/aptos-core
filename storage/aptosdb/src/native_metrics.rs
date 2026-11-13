// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_metrics_core::{register_int_counter_vec, register_int_gauge, IntCounterVec, IntGauge};
use once_cell::sync::Lazy;

pub static POSITION_WRITES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_position_writes_total",
        "Position writes applied by NativeStateCommitter",
        &["kind"]
    )
    .unwrap()
});

pub static ORDER_WRITES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_order_writes_total",
        "Order writes applied by NativeStateCommitter",
        &["kind"]
    )
    .unwrap()
});

pub static COLLATERAL_WRITES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_collateral_writes_total",
        "Collateral writes applied by NativeStateCommitter",
        &["kind"]
    )
    .unwrap()
});

/// Chunks executed past the position base and not yet folded into it.
pub static POSITION_OVERLAY_LAYERS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_position_overlay_layers",
        "Layers of the published position overlay above the position base"
    )
    .unwrap()
});

/// Versions the position base trails the published tip by.
pub static POSITION_BASE_LAG_VERSIONS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_position_base_lag_versions",
        "Versions between the position base and the published position overlay"
    )
    .unwrap()
});
