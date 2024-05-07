// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{op_counters::DurationHistogram, register_histogram};
use once_cell::sync::Lazy;

/// Duration of each run of the event loop.
pub static BLOCK_FETCH_MANAGER_MAIN_LOOP: Lazy<DurationHistogram> = Lazy::new(|| {
    DurationHistogram::new(
        register_histogram!(
            "block_fetch_manager_main_loop",
            "Duration of the each run of the block fetch manager event loop"
        )
        .unwrap(),
    )
});
