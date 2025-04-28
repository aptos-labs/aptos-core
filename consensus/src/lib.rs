// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

//! Consensus for the Aptos Core blockchain
//!
//! The consensus protocol implemented is AptosBFT (based on
//! [DiemBFT](https://developers.diem.com/papers/diem-consensus-state-machine-replication-in-the-diem-blockchain/2021-08-17.pdf)).

#![cfg_attr(feature = "fuzzing", allow(dead_code))]
#![recursion_limit = "512"]

#[macro_use(defer)]
extern crate scopeguard;

extern crate core;

mod block_storage;
mod consensusdb;
mod dag;
mod epoch_manager;
mod error;
mod liveness;
mod logging;
mod metrics_safety_rules;
mod network;
#[cfg(test)]
mod network_tests;
mod payload_client;
mod pending_order_votes;
mod pending_votes;
#[cfg(test)]
mod pending_votes_test;
pub mod persistent_liveness_storage;
mod pipeline;
pub mod quorum_store;
mod rand;
mod recovery_manager;
mod round_manager;
mod state_computer;
#[cfg(test)]
mod state_computer_tests;
mod state_replication;
#[cfg(any(test, feature = "fuzzing"))]
mod test_utils;
#[cfg(test)]
mod twins;
mod txn_notifier;
pub mod util;

mod block_preparer;
pub mod consensus_observer;
/// AptosBFT implementation
pub mod consensus_provider;
/// Required by the telemetry service
pub mod counters;
mod execution_pipeline;
/// AptosNet interface.
pub mod network_interface;
mod payload_manager;
mod transaction_deduper;
mod transaction_filter;
mod transaction_shuffler;
#[cfg(feature = "fuzzing")]
pub use transaction_shuffler::transaction_shuffler_fuzzing;
mod txn_hash_and_authenticator_deduper;
mod scheduled_txns_handler;

use aptos_metrics_core::IntGauge;
pub use consensusdb::create_checkpoint;
/// Required by the smoke tests
pub use consensusdb::CONSENSUS_DB_NAME;
pub use quorum_store::quorum_store_db::QUORUM_STORE_DB_NAME;
#[cfg(feature = "fuzzing")]
pub use round_manager::round_manager_fuzzing;

pub struct IntGaugeGuard {
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

/// Helper function to record metrics for external calls.
/// Include call counts, time, and whether it's inside or not (1 or 0).
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
