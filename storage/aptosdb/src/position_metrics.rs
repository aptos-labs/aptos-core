// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_metrics_core::{register_int_counter_vec, IntCounterVec};
use once_cell::sync::Lazy;

pub static POSITION_WRITES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_position_writes_total",
        "Position writes applied by NativeStateCommitter",
        &["kind"]
    )
    .unwrap()
});
