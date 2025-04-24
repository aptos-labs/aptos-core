// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod delays;
pub mod framework;
pub mod leader_schedule;
pub mod metrics;
pub mod raptr;

#[cfg(all(feature = "sim-types", not(feature = "force-aptos-types")))]
pub mod simulation_test;

pub type Slot = i64;

pub const PBFT_TIMEOUT: u32 = 5; // in Deltas
pub const JOLTEON_TIMEOUT: u32 = 3; // in Deltas

/// Helper function to record metrics for external calls.
/// Include call counts, time, and whether it's inside or not (1 or 0).
/// It assumes a OpMetrics defined as OP_COUNTERS in crate::counters;
#[macro_export]
macro_rules! monitor {
    ($name:literal, $fn:expr) => {{
        use $crate::raptr::counters::OP_COUNTERS;
        let _timer = OP_COUNTERS.timer($name);

        $fn
    }};
}
