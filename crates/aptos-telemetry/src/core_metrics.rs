// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{utils, utils::sum_all_histogram_counts};
use aptos_config::config::NodeConfig;
use aptos_state_sync_driver::metrics::StorageSynchronizerOperations;
use aptos_telemetry_service::types::telemetry::TelemetryEvent;
use once_cell::sync::Lazy;
use prometheus::{
    core::{Collector, GenericGauge},
    Histogram, IntCounter, IntCounterVec,
};
use std::collections::BTreeMap;

/// Core metrics event name
const APTOS_NODE_CORE_METRICS: &str = "APTOS_NODE_CORE_METRICS";

/// Core metric keys
const CONSENSUS_LAST_COMMITTED_ROUND: &str = "consensus_last_committed_round";
const CONSENSUS_PROPOSALS_COUNT: &str = "consensus_proposals_count";
const CONSENSUS_TIMEOUT_COUNT: &str = "consensus_timeout_count";
const CONSENSUS_LAST_COMMITTED_VERSION: &str = "consensus_last_committed_version";
const CONSENSUS_COMMITTED_BLOCKS_COUNT: &str = "consensus_committed_blocks_count";
const CONSENSUS_COMMITTED_TXNS_COUNT: &str = "consensus_committed_txns_count";
const CONSENSUS_ROUND_TIMEOUT_MS: &str = "consensus_round_timeout_ms";
const CONSENSUS_SYNC_INFO_MSG_SENT_COUNT: &str = "consensus_sync_info_msg_sent_count";
const CONSENSUS_CURRENT_ROUND: &str = "consensus_current_round";
const CONSENSUS_WAIT_DURATION_MS: &str = "consensus_wait_duration_ms";
const MEMPOOL_CORE_MEMPOOL_INDEX_SIZE: &str = "mempool_core_mempool_index_size";
const REST_RESPONSE_COUNT: &str = "rest_response_count";
const ROLE_TYPE: &str = "role_type";
const STATE_SYNC_BOOTSTRAP_MODE: &str = "state_sync_bootstrap_mode";
const STATE_SYNC_CODE_VERSION: &str = "state_sync_code_version";
const STATE_SYNC_CONTINUOUS_SYNC_MODE: &str = "state_sync_continuous_sync_mode";
const STATE_SYNC_SYNCED_VERSION: &str = "state_sync_synced_version";
const STATE_SYNC_SYNCED_EPOCH: &str = "state_sync_synced_epoch";
const STORAGE_LEDGER_VERSION: &str = "storage_ledger_version";
const STORAGE_MIN_READABLE_LEDGER_VERSION: &str = "storage_min_readable_ledger_version";
const STORAGE_MIN_READABLE_STATE_MERKLE_VERSION: &str = "storage_min_readable_state_merkle_version";
const STORAGE_MIN_READABLE_STATE_KV_VERSION: &str = "storage_min_readable_state_kv_version";
const TELEMETRY_FAILURE_COUNT: &str = "telemetry_failure_count";
const TELEMETRY_SUCCESS_COUNT: &str = "telemetry_success_count";

/// Collects and sends the build information via telemetry
pub(crate) async fn create_core_metric_telemetry_event(node_config: &NodeConfig) -> TelemetryEvent {
    // Collect the core metrics
    let core_metrics = get_core_metrics(node_config);

    // Create and return a new telemetry event
    TelemetryEvent {
        name: APTOS_NODE_CORE_METRICS.into(),
        params: core_metrics,
    }
}

/// Used to expose core metrics for the node
pub fn get_core_metrics(node_config: &NodeConfig) -> BTreeMap<String, String> {
    let mut core_metrics: BTreeMap<String, String> = BTreeMap::new();
    collect_core_metrics(&mut core_metrics, node_config);
    core_metrics
}

/// Collects the core metrics and appends them to the given map
fn collect_core_metrics(core_metrics: &mut BTreeMap<String, String>, node_config: &NodeConfig) {
    // Collect the core metrics for each component
    collect_consensus_metrics(core_metrics);
    collect_mempool_metrics(core_metrics);
    collect_rest_metrics(core_metrics);
    collect_state_sync_metrics(core_metrics, node_config);
    collect_storage_metrics(core_metrics);
    collect_telemetry_metrics(core_metrics);

    // Collect the node role
    let node_role_type = node_config.base.role;
    core_metrics.insert(ROLE_TYPE.into(), node_role_type.as_str().into());
}

/// Collects the consensus metrics and appends it to the given map
fn collect_consensus_metrics(core_metrics: &mut BTreeMap<String, String>) {
    // Helper function to safely get consensus counter metric
    let get_counter_metric = |metric: &'static Lazy<IntCounter>| -> String {
        Lazy::get(metric).map_or("0".to_string(), |counter| counter.get().to_string())
    };

    // Helper function to safely get consensus gauge metric
    let get_gauge_metric = |metric: &'static Lazy<GenericGauge<prometheus::core::AtomicI64>>| -> String {
        Lazy::get(metric).map_or("0".to_string(), |gauge| gauge.get().to_string())
    };

    let get_counter_vec_metric = |metric: &'static Lazy<IntCounterVec>| -> String {
        Lazy::get(metric).map_or("0".to_string(), |counter| {
            counter.with_label_values(&["success"]).get().to_string()
        })
    };

    // Helper function to safely get histogram values
    let get_histogram_values = |metric: &'static Lazy<Histogram>| -> String {
        Lazy::get(metric).map_or("0".to_string(), |histogram| {
            let sum = histogram.get_sample_sum();
            let count = histogram.get_sample_count();
            format!("{} {}", sum, count) // Report sum and count for dashboard aggregation
        })
    };

    // Collect basic consensus metrics
    core_metrics.insert(
        CONSENSUS_PROPOSALS_COUNT.into(),
        get_counter_metric(&aptos_consensus::counters::PROPOSALS_COUNT),
    );
    core_metrics.insert(
        CONSENSUS_LAST_COMMITTED_ROUND.into(),
        get_gauge_metric(&aptos_consensus::counters::LAST_COMMITTED_ROUND),
    );
    core_metrics.insert(
        CONSENSUS_TIMEOUT_COUNT.into(),
        get_counter_metric(&aptos_consensus::counters::TIMEOUT_COUNT),
    );
    
    // Enhanced consensus metrics
    core_metrics.insert(
        CONSENSUS_LAST_COMMITTED_VERSION.into(),
        get_gauge_metric(&aptos_consensus::counters::LAST_COMMITTED_VERSION),
    );
    core_metrics.insert(
        CONSENSUS_COMMITTED_BLOCKS_COUNT.into(),
        get_counter_metric(&aptos_consensus::counters::COMMITTED_BLOCKS_COUNT),
    );
    core_metrics.insert(
        CONSENSUS_COMMITTED_TXNS_COUNT.into(),
        get_counter_vec_metric(&aptos_consensus::counters::COMMITTED_TXNS_COUNT),
    );
    
    // Get the current round
    core_metrics.insert(
        CONSENSUS_CURRENT_ROUND.into(),
        get_gauge_metric(&aptos_consensus::counters::CURRENT_ROUND),
    );
    
    // Get the round timeout in milliseconds
    core_metrics.insert(
        CONSENSUS_ROUND_TIMEOUT_MS.into(),
        get_gauge_metric(&aptos_consensus::counters::ROUND_TIMEOUT_MS),
    );
    
    // Get sync info messages count
    core_metrics.insert(
        CONSENSUS_SYNC_INFO_MSG_SENT_COUNT.into(),
        get_counter_metric(&aptos_consensus::counters::SYNC_INFO_MSGS_SENT_COUNT),
    );

    // Get wait duration histogram values (sum and count)
    core_metrics.insert(
        CONSENSUS_WAIT_DURATION_MS.into(),
        get_histogram_values(&aptos_consensus::counters::WAIT_DURATION_MS),
    );
}

/// Collects the mempool metrics and appends it to the given map
fn collect_mempool_metrics(core_metrics: &mut BTreeMap<String, String>) {
    core_metrics.insert(
        MEMPOOL_CORE_MEMPOOL_INDEX_SIZE.into(),
        aptos_mempool::counters::CORE_MEMPOOL_INDEX_SIZE
            .with_label_values(&["system_ttl"])
            .get()
            .to_string(),
    );
    
    // Add additional mempool metrics for transaction processing
    core_metrics.insert(
        "mempool_txns_processed_success".into(),
        aptos_mempool::counters::SHARED_MEMPOOL_TRANSACTIONS_PROCESSED
            .with_label_values(&["success", "local"])
            .get()
            .to_string(),
    );
    
    core_metrics.insert(
        "mempool_txns_processed_total".into(),
        aptos_mempool::counters::SHARED_MEMPOOL_TRANSACTIONS_PROCESSED
            .with_label_values(&["received", "local"])
            .get()
            .to_string(),
    );
    
    // Get average transaction broadcast size from HistogramVec
    let broadcast_size = &aptos_mempool::counters::SHARED_MEMPOOL_TRANSACTION_BROADCAST_SIZE;
    let mut total_sum = 0.0;
    let mut total_count = 0.0;
    
    // Sum up values across all label combinations
    for label_values in broadcast_size.get_metric_with_label_values(&["success"]).iter() {
        total_sum += label_values.get_sample_sum();
        total_count += label_values.get_sample_count() as f64;
    }
    
    let avg_broadcast_size = if total_count > 0.0 {
        total_sum / total_count
    } else {
        0.0
    };
    core_metrics.insert("mempool_avg_txn_broadcast_size".into(), avg_broadcast_size.to_string());
    
    // Get pending transaction count in mempool
    core_metrics.insert(
        "mempool_pending_txns".into(),
        aptos_mempool::counters::CORE_MEMPOOL_INDEX_SIZE
            .with_label_values(&["system_ttl"])
            .get()
            .to_string(),
    );
}

/// Collects the REST metrics and appends it to the given map
fn collect_rest_metrics(core_metrics: &mut BTreeMap<String, String>) {
    let rest_response_metrics = Lazy::get(&aptos_api::metrics::RESPONSE_STATUS)
        .map_or(Vec::new(), |metrics| metrics.collect());
    let rest_response_count = sum_all_histogram_counts(&rest_response_metrics);
    core_metrics.insert(REST_RESPONSE_COUNT.into(), rest_response_count.to_string());
}

/// Collects the state sync metrics and appends it to the given map
fn collect_state_sync_metrics(
    core_metrics: &mut BTreeMap<String, String>,
    node_config: &NodeConfig,
) {
    let state_sync_driver_config = node_config.state_sync.state_sync_driver;

    // Get the state sync code version
    core_metrics.insert(STATE_SYNC_CODE_VERSION.into(), "2".into());

    core_metrics.insert(
        STATE_SYNC_SYNCED_EPOCH.into(),
        aptos_state_sync_driver::metrics::STORAGE_SYNCHRONIZER_OPERATIONS
            .with_label_values(&[StorageSynchronizerOperations::SyncedEpoch.get_label()])
            .get()
            .to_string(),
    );
    core_metrics.insert(
        STATE_SYNC_SYNCED_VERSION.into(),
        aptos_state_sync_driver::metrics::STORAGE_SYNCHRONIZER_OPERATIONS
            .with_label_values(&[StorageSynchronizerOperations::Synced.get_label()])
            .get()
            .to_string(),
    );
    core_metrics.insert(
        STATE_SYNC_BOOTSTRAP_MODE.into(),
        state_sync_driver_config
            .bootstrapping_mode
            .to_label()
            .into(),
    );
    core_metrics.insert(
        STATE_SYNC_CONTINUOUS_SYNC_MODE.into(),
        state_sync_driver_config
            .continuous_syncing_mode
            .to_label()
            .into(),
    );
}

/// Collects the storage metrics and appends it to the given map
fn collect_storage_metrics(core_metrics: &mut BTreeMap<String, String>) {
    // Helper function to safely get metric value
    let get_metric_value = |metric_name: &str, label_values: &[&str]| -> String {
        if let Some(metric) = Lazy::get(&aptos_db::metrics::PRUNER_VERSIONS) {
            if let Ok(m) = metric.get_metric_with_label_values(label_values) {
                return m.get().to_string();
            }
        }
        "0".to_string()
    };

    // Get basic storage metrics
    if let Some(ledger_version) = Lazy::get(&aptos_db::metrics::LEDGER_VERSION) {
        core_metrics.insert(
            STORAGE_LEDGER_VERSION.into(),
            ledger_version.get().to_string(),
        );
    }
    
    // Get pruner metrics safely
    core_metrics.insert(
        STORAGE_MIN_READABLE_LEDGER_VERSION.into(),
        get_metric_value("pruner_versions", &["ledger_pruner", "min_readable"]),
    );
    core_metrics.insert(
        STORAGE_MIN_READABLE_STATE_MERKLE_VERSION.into(),
        get_metric_value("pruner_versions", &["state_merkle_pruner", "min_readable"]),
    );
    core_metrics.insert(
        STORAGE_MIN_READABLE_STATE_KV_VERSION.into(),
        get_metric_value("pruner_versions", &["state_kv_pruner", "min_readable"]),
    );

    // Add storage latency metrics safely
    if let Some(get_latency) = Lazy::get(&aptos_db::metrics::APTOS_SCHEMADB_GET_LATENCY_SECONDS) {
        if let Ok(get_txn_histogram) = get_latency.get_metric_with_label_values(&["transaction_schema"]) {
            let avg_get_txn_latency = if get_txn_histogram.get_sample_count() > 0 {
                get_txn_histogram.get_sample_sum() / get_txn_histogram.get_sample_count() as f64
            } else {
                0.0
            };
            core_metrics.insert("storage_get_transaction_latency_s".into(), avg_get_txn_latency.to_string());
        }
    }

    // Add commit latency metrics safely
    if let Some(commit_histogram) = Lazy::get(&aptos_db::metrics::APTOS_STORAGE_SERVICE_COMMIT_LATENCY_SECONDS) {
        let avg_commit_latency = if commit_histogram.get_sample_count() > 0 {
            commit_histogram.get_sample_sum() / commit_histogram.get_sample_count() as f64
        } else {
            0.0
        };
        core_metrics.insert("storage_commit_latency_s".into(), avg_commit_latency.to_string());
    }

    // Add save transactions latency safely
    if let Some(save_txns_histogram) = Lazy::get(&aptos_db::metrics::APTOS_STORAGE_SERVICE_SAVE_TRANSACTIONS_LATENCY_SECONDS) {
        let avg_save_txns_latency = if save_txns_histogram.get_sample_count() > 0 {
            save_txns_histogram.get_sample_sum() / save_txns_histogram.get_sample_count() as f64
        } else {
            0.0
        };
        core_metrics.insert("storage_save_transactions_latency_s".into(), avg_save_txns_latency.to_string());
    }
}

/// Collects the telemetry metrics and appends it to the given map
fn collect_telemetry_metrics(core_metrics: &mut BTreeMap<String, String>) {
    // Get failure metrics, defaulting to empty vec if not initialized
    let telemetry_failure_metrics = Lazy::get(&crate::metrics::APTOS_TELEMETRY_FAILURE)
        .map_or(Vec::new(), |metrics| metrics.collect());
    let telemetry_failure_count = utils::sum_all_gauges(&telemetry_failure_metrics);
    core_metrics.insert(
        TELEMETRY_FAILURE_COUNT.into(),
        telemetry_failure_count.to_string(),
    );

    // Get success metrics, defaulting to empty vec if not initialized
    let telemetry_success_metrics = Lazy::get(&crate::metrics::APTOS_TELEMETRY_SUCCESS)
        .map_or(Vec::new(), |metrics| metrics.collect());
    let telemetry_success_count = utils::sum_all_gauges(&telemetry_success_metrics);
    core_metrics.insert(
        TELEMETRY_SUCCESS_COUNT.into(),
        telemetry_success_count.to_string(),
    );
}
