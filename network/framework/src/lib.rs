// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

// <Black magic>
// Increase recursion limit to allow for use of select! macro.
#![recursion_limit = "1024"]
// </Black magic>

// TODO(philiphayes): uncomment when feature stabilizes (est. 1.50.0)
// tracking issue: https://github.com/rust-lang/rust/issues/78835
// #![doc = include_str!("../README.md")]

use aptos_metrics_core::IntGauge;

pub mod application;
pub mod connectivity_manager;
pub mod constants;
pub mod counters;
pub mod error;
pub mod logging;
pub mod noise;
pub mod peer;
pub mod peer_manager;
pub mod protocols;
pub mod transport;

#[cfg(feature = "fuzzing")]
pub mod fuzzing;
#[cfg(any(test, feature = "testing", feature = "fuzzing"))]
pub mod testutils;

pub type DisconnectReason = peer::DisconnectReason;
pub type ConnectivityRequest = connectivity_manager::ConnectivityRequest;
pub type ProtocolId = protocols::wire::handshake::v1::ProtocolId;

/// It assumes a OpMetrics defined as OP_COUNTERS in crate::counters;
#[macro_export]
macro_rules! monitor {
    ($name:literal, $fn:expr) => {{
        use $crate::{counters::OP_COUNTERS, IntGaugeGuard};
        let _timer = OP_COUNTERS.timer($name);
        let _guard = IntGaugeGuard::new(OP_COUNTERS.gauge(concat!($name, "_running")));
        $fn
    }};
}

struct IntGaugeGuard {
    gauge: IntGauge,
}

impl IntGaugeGuard {
    fn new(gauge: IntGauge) -> Self {
        gauge.inc();
        Self { gauge }
    }
}

impl Drop for IntGaugeGuard {
    fn drop(&mut self) {
        self.gauge.dec();
    }
}
