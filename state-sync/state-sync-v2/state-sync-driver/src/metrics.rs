// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{
    register_histogram_vec, register_int_counter_vec, register_int_gauge_vec, HistogramTimer,
    HistogramVec, IntCounterVec, IntGaugeVec,
};
use once_cell::sync::Lazy;

/// Useful metric labels
pub const DRIVER_CLIENT_NOTIFICATION: &str = "driver_client_notification";
pub const DRIVER_CONSENSUS_COMMIT_NOTIFICATION: &str = "driver_consensus_commit_notification";
pub const DRIVER_CONSENSUS_SYNC_NOTIFICATION: &str = "driver_consensus_sync_notification";
pub const STORAGE_SYNCHRONIZER_PENDING_DATA: &str = "storage_synchronizer_pending_data";
pub const STORAGE_SYNCHRONIZER_APPLY_CHUNK: &str = "apply_chunk";
pub const STORAGE_SYNCHRONIZER_EXECUTE_CHUNK: &str = "execute_chunk";
pub const STORAGE_SYNCHRONIZER_COMMIT_CHUNK: &str = "commit_chunk";

/// An enum representing the component currently executing
pub enum ExecutingComponent {
    Bootstrapper,
    Consensus,
    ContinuousSyncer,
}

impl ExecutingComponent {
    pub fn get_label(&self) -> &'static str {
        match self {
            ExecutingComponent::Bootstrapper => "bootstrapper",
            ExecutingComponent::Consensus => "consensus",
            ExecutingComponent::ContinuousSyncer => "continuous_syncer",
        }
    }
}

/// An enum of storage synchronizer operations performed by state sync
pub enum StorageSynchronizerOperations {
    AppliedTransactionOutputs, // Applied a chunk of transactions outputs.
    ExecutedTransactions,      // Executed a chunk of transactions.
    Synced,                    // Wrote a chunk of transactions and outputs to storage.
    SyncedStates,              // Wrote a chunk of state values to storage.
    SyncedEpoch, // Wrote a chunk of transactions and outputs to storage that resulted in a new epoch.
}

impl StorageSynchronizerOperations {
    pub fn get_label(&self) -> &'static str {
        match self {
            StorageSynchronizerOperations::AppliedTransactionOutputs => {
                "applied_transaction_outputs"
            }
            StorageSynchronizerOperations::ExecutedTransactions => "executed_transactions",
            StorageSynchronizerOperations::Synced => "synced",
            StorageSynchronizerOperations::SyncedEpoch => "synced_epoch",
            StorageSynchronizerOperations::SyncedStates => "synced_states",
        }
    }
}

/// Counter for state sync bootstrapper errors
pub static BOOTSTRAPPER_ERRORS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_state_sync_bootstrapper_errors",
        "Counters related to state sync bootstrapper errors",
        &["error_label"]
    )
    .unwrap()
});

/// Counter for state sync continuous syncer errors
pub static CONTINUOUS_SYNCER_ERRORS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_state_sync_continuous_syncer_errors",
        "Counters related to state sync continuous syncer errors",
        &["error_label"]
    )
    .unwrap()
});

/// Counters related to the state sync driver
pub static DRIVER_COUNTERS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_state_sync_driver_counters",
        "Counters related to the state sync driver",
        &["label"]
    )
    .unwrap()
});

/// Counters related to the currently executing component
pub static EXECUTING_COMPONENT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_state_sync_executing_component_counters",
        "Counters related to the currently executing component",
        &["label"]
    )
    .unwrap()
});

/// Counter for storage synchronizer errors
pub static STORAGE_SYNCHRONIZER_ERRORS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_state_sync_storage_synchronizer_errors",
        "Counters related to storage synchronizer errors",
        &["error_label"]
    )
    .unwrap()
});

/// Gauges related to the storage synchronizer
pub static STORAGE_SYNCHRONIZER_GAUGES: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_state_sync_storage_synchronizer_gauges",
        "Gauges related to the storage synchronizer",
        &["label"]
    )
    .unwrap()
});

/// Counter for tracking storage synchronizer latencies
pub static STORAGE_SYNCHRONIZER_LATENCIES: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_state_sync_storage_synchronizer_latencies",
        "Counters related to the storage synchronizer latencies",
        &["label"]
    )
    .unwrap()
});

/// Gauges for the storage synchronizer operations
pub static STORAGE_SYNCHRONIZER_OPERATIONS: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_state_sync_version",
        "The versions processed by the storage synchronizer operations",
        &["type"]
    )
    .unwrap()
});

/// Increments the given counter with the provided label values.
pub fn increment_counter(counter: &Lazy<IntCounterVec>, label: &str) {
    counter.with_label_values(&[label]).inc();
}

/// Increments the gauge with the specific label by the given delta
pub fn increment_gauge(gauge: &Lazy<IntGaugeVec>, label: &str, delta: u64) {
    gauge.with_label_values(&[label]).add(delta as i64);
}

/// Decrements the gauge with the specific label by the given delta
pub fn decrement_gauge(gauge: &Lazy<IntGaugeVec>, label: &str, delta: u64) {
    gauge.with_label_values(&[label]).sub(delta as i64);
}

/// Reads the gauge with the specific label
pub fn read_gauge(gauge: &Lazy<IntGaugeVec>, label: &str) -> i64 {
    gauge.with_label_values(&[label]).get()
}

/// Sets the gauge with the specific label to the given value
pub fn set_gauge(gauge: &Lazy<IntGaugeVec>, label: &str, value: u64) {
    gauge.with_label_values(&[label]).set(value as i64);
}

/// Starts the timer for the provided histogram and label
pub fn start_timer(histogram: &Lazy<HistogramVec>, label: &str) -> HistogramTimer {
    histogram.with_label_values(&[label]).start_timer()
}
