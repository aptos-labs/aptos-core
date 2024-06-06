// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{register_int_counter_vec, IntCounterVec};
use once_cell::sync::Lazy;

/// Counter for pending network events to the consensus observer
pub static PENDING_CONSENSUS_OBSERVER_NETWORK_EVENTS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_consensus_observer_pending_network_events",
        "Counters for pending network events for consensus observer",
        &["state"]
    )
    .unwrap()
});
