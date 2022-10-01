// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

//! Consensus for the Aptos Core blockchain
//!
//! The consensus protocol implemented is AptosBFT (based on
//! [HotStuff](https://arxiv.org/pdf/1803.05069.pdf)).

#![cfg_attr(not(feature = "fuzzing"), deny(missing_docs))]
#![cfg_attr(feature = "fuzzing", allow(dead_code))]
#![recursion_limit = "512"]

extern crate core;

mod block_storage;
mod commit_notifier;
mod consensusdb;
mod epoch_manager;
mod error;
mod experimental;
mod liveness;
mod logging;
mod metrics_safety_rules;
mod network;
#[cfg(test)]
mod network_tests;
mod payload_manager;
mod pending_votes;
mod persistent_liveness_storage;
mod quorum_store;
mod recovery_manager;
mod round_manager;
mod state_computer;
mod state_replication;
#[cfg(any(test, feature = "fuzzing"))]
mod test_utils;
#[cfg(test)]
mod twins;
mod txn_notifier;
mod util;

/// AptosBFT implementation
pub mod consensus_provider;
/// Required by the telemetry service
pub mod counters;
/// AptosNet interface.
pub mod network_interface;

/// Required by the smoke tests
pub use consensusdb::CONSENSUS_DB_NAME;

#[cfg(feature = "fuzzing")]
pub use round_manager::round_manager_fuzzing;

/// Helper function to record metrics for external calls.
/// Include call counts, time, and whether it's inside or not (1 or 0).
/// It assumes a OpMetrics defined as OP_COUNTERS in crate::counters;
#[macro_export]
macro_rules! monitor {
    ( $name:literal, $fn:expr ) => {{
        use $crate::counters::OP_COUNTERS;
        let _timer = OP_COUNTERS.timer($name);
        let gauge = OP_COUNTERS.gauge(concat!($name, "_running"));
        gauge.inc();
        let result = $fn;
        gauge.dec();
        result
    }};
}
