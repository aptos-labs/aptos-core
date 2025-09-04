// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_metrics_core::{
    exponential_buckets, histogram_opts, register_histogram_vec, register_int_counter_vec,
    register_int_gauge, register_int_gauge_vec, HistogramTimer, HistogramVec, IntCounterVec,
    IntGauge, IntGaugeVec,
};
use once_cell::sync::Lazy;
use std::time::Instant;

/// Driver metric labels
pub const DRIVER_CLIENT_NOTIFICATION: &str = "driver_client_notification";
pub const DRIVER_CONSENSUS_COMMIT_NOTIFICATION: &str = "driver_consensus_commit_notification";
pub const DRIVER_CONSENSUS_SYNC_DURATION_NOTIFICATION: &str =
    "driver_consensus_sync_duration_notification";
pub const DRIVER_CONSENSUS_SYNC_TARGET_NOTIFICATION: &str =
    "driver_consensus_sync_target_notification";

/// Data notification metric labels
pub const NOTIFICATION_CREATE_TO_APPLY: &str = "notification_create_to_apply";
pub const NOTIFICATION_CREATE_TO_COMMIT: &str = "notification_create_to_commit";
pub const NOTIFICATION_CREATE_TO_COMMIT_POST_PROCESS: &str =
    "notification_create_to_commit_post_process";
pub const NOTIFICATION_CREATE_TO_EXECUTE: &str = "notification_create_to_execute";
pub const NOTIFICATION_CREATE_TO_RECEIVE: &str = "notification_create_to_receive";
pub const NOTIFICATION_CREATE_TO_UPDATE_LEDGER: &str = "notification_create_to_update_ledger";

/// Storage synchronizer metric labels
pub const STORAGE_SYNCHRONIZER_PENDING_DATA: &str = "storage_synchronizer_pending_data";
pub const STORAGE_SYNCHRONIZER_APPLY_CHUNK: &str = "apply_chunk";
pub const STORAGE_SYNCHRONIZER_EXECUTE_CHUNK: &str = "execute_chunk";
pub const STORAGE_SYNCHRONIZER_UPDATE_LEDGER: &str = "update_ledger";
pub const STORAGE_SYNCHRONIZER_COMMIT_CHUNK: &str = "commit_chunk";
pub const STORAGE_SYNCHRONIZER_COMMIT_POST_PROCESS: &str = "commit_post_process";
pub const STORAGE_SYNCHRONIZER_STATE_VALUE_CHUNK: &str = "state_value_chunk";

/// Storage synchronizer pipeline channel labels
pub const STORAGE_SYNCHRONIZER_EXECUTOR: &str = "executor";
pub const STORAGE_SYNCHRONIZER_LEDGER_UPDATER: &str = "ledger_updater";
pub const STORAGE_SYNCHRONIZER_COMMITTER: &str = "committer";
pub const STORAGE_SYNCHRONIZER_COMMIT_POST_PROCESSOR: &str = "commit_post_processor";
pub const STORAGE_SYNCHRONIZER_STATE_SNAPSHOT_RECEIVER: &str = "state_snapshot_receiver";

/// An enum representing the component currently executing
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutingComponent {
    Bootstrapper,
    Consensus,
    ConsensusObserver,
    ContinuousSyncer,
}

impl ExecutingComponent {
    pub fn get_label(&self) -> &'static str {
        match self {
            ExecutingComponent::Bootstrapper => "bootstrapper",
            ExecutingComponent::Consensus => "consensus",
            ExecutingComponent::ConsensusObserver => "consensus_observer",
            ExecutingComponent::ContinuousSyncer => "continuous_syncer",
        }
    }
}

/// An enum of storage synchronizer operations performed by
/// state sync. Each of these is a metric label to track.
pub enum StorageSynchronizerOperations {
    AppliedTransactionOutputs, // The total number of applied transaction outputs
    ExecutedTransactions,      // The total number of executed transactions
    Synced,                    // The latest synced version (as read from storage)
    SyncedIncremental, // The latest synced version (calculated as the sum of all processed transactions)
    SyncedStates,      // The total number of synced states
    SyncedEpoch,       // The latest synced epoch (as read from storage)
    SyncedEpochIncremental, // The latest synced epoch (calculated as the sum of all processed epochs)
}

impl StorageSynchronizerOperations {
    pub fn get_label(&self) -> &'static str {
        match self {
            StorageSynchronizerOperations::AppliedTransactionOutputs => {
                "applied_transaction_outputs"
            },
            StorageSynchronizerOperations::ExecutedTransactions => "executed_transactions",
            StorageSynchronizerOperations::Synced => "synced",
            StorageSynchronizerOperations::SyncedIncremental => "synced_incremental",
            StorageSynchronizerOperations::SyncedStates => "synced_states",
            StorageSynchronizerOperations::SyncedEpoch => "synced_epoch",
            StorageSynchronizerOperations::SyncedEpochIncremental => "synced_epoch_incremental",
        }
    }
}

/// Histogram buckets for tracking chunk sizes
const CHUNK_SIZE_BUCKETS: &[f64] = &[
    1.0, 2.0, 4.0, 5.0, 10.0, 25.0, 50.0, 75.0, 100.0, 250.0, 500.0, 750.0, 1000.0, 2500.0, 5000.0,
    7500.0, 10_000.0, 12_500.0, 15_000.0, 17_500.0, 20_000.0, 25_000.0, 30_000.0, 35_000.0,
    40_000.0, 45_000.0, 50_000.0, 75_000.0, 100_000.0,
];

/// Counter for state sync bootstrapper errors
pub static BOOTSTRAPPER_ERRORS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_state_sync_bootstrapper_errors",
        "Counters related to state sync bootstrapper errors",
        &["error_label"]
    )
    .unwrap()
});

/// Gauge indicating whether consensus is currently executing
pub static CONSENSUS_EXECUTING_GAUGE: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "velor_state_sync_consensus_executing_gauge",
        "Gauge indicating whether consensus is currently executing"
    )
    .unwrap()
});

/// Gauge for state sync continuous syncer fallback mode
pub static CONTINUOUS_SYNCER_ERRORS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_state_sync_continuous_syncer_errors",
        "Counters related to state sync continuous syncer errors",
        &["error_label"]
    )
    .unwrap()
});

/// Counters related to the state sync driver
pub static DRIVER_COUNTERS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_state_sync_driver_counters",
        "Counters related to the state sync driver",
        &["label"]
    )
    .unwrap()
});

/// Counter for tracking data notification latencies
pub static DATA_NOTIFICATION_LATENCIES: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "velor_state_sync_data_notification_latencies",
        "Counters related to the data notification latencies",
        &["label"],
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 30).unwrap(),
    )
    .unwrap()
});

/// Gauge for state sync bootstrapper fallback mode
pub static DRIVER_FALLBACK_MODE: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "velor_state_sync_driver_fallback_mode",
        "Gauges related to the driver fallback mode",
        &["label"]
    )
    .unwrap()
});

/// Counters related to the currently executing component in the main driver loop
pub static EXECUTING_COMPONENT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_state_sync_executing_component_counters",
        "Counters related to the currently executing component",
        &["label"]
    )
    .unwrap()
});

/// Counter for tracking sizes of data chunks sent to the storage synchronizer
pub static STORAGE_SYNCHRONIZER_CHUNK_SIZES: Lazy<HistogramVec> = Lazy::new(|| {
    let histogram_opts = histogram_opts!(
        "velor_state_sync_storage_synchronizer_chunk_sizes",
        "Counter for tracking sizes of data chunks sent to the storage synchronizer",
        CHUNK_SIZE_BUCKETS.to_vec()
    );
    register_histogram_vec!(histogram_opts, &["label"]).unwrap()
});

/// Counter for storage synchronizer errors
pub static STORAGE_SYNCHRONIZER_ERRORS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_state_sync_storage_synchronizer_errors",
        "Counters related to storage synchronizer errors",
        &["error_label"]
    )
    .unwrap()
});

/// Gauges related to the storage synchronizer
pub static STORAGE_SYNCHRONIZER_GAUGES: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "velor_state_sync_storage_synchronizer_gauges",
        "Gauges related to the storage synchronizer",
        &["label"]
    )
    .unwrap()
});

/// Counter for tracking storage synchronizer latencies
pub static STORAGE_SYNCHRONIZER_LATENCIES: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "velor_state_sync_storage_synchronizer_latencies",
        "Counters related to the storage synchronizer latencies",
        &["label"],
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});

/// Gauges for the storage synchronizer operations
pub static STORAGE_SYNCHRONIZER_OPERATIONS: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "velor_state_sync_version",
        "The versions processed by the storage synchronizer operations",
        &["type"]
    )
    .unwrap()
});

/// Gauges for tracking the storage synchronizer pipeline channel backpressure
pub static STORAGE_SYNCHRONIZER_PIPELINE_CHANNEL_BACKPRESSURE: Lazy<IntGaugeVec> =
    Lazy::new(|| {
        register_int_gauge_vec!(
            "velor_state_sync_storage_synchronizer_pipeline_channel_backpressure",
            "Gauges for tracking the storage synchronizer pipeline channel backpressure",
            &["channel"]
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

/// Adds a new duration observation for the given histogram and label
pub fn observe_duration(histogram: &Lazy<HistogramVec>, label: &str, start_time: Instant) {
    histogram
        .with_label_values(&[label])
        .observe(start_time.elapsed().as_secs_f64());
}

/// Adds a new observation for the given histogram, label and value
pub fn observe_value(histogram: &Lazy<HistogramVec>, label: &str, value: u64) {
    histogram.with_label_values(&[label]).observe(value as f64);
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
