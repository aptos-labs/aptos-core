// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{utils, utils::sum_all_histogram_counts};
use velor_config::config::NodeConfig;
use velor_state_sync_driver::metrics::StorageSynchronizerOperations;
use velor_telemetry_service::types::telemetry::TelemetryEvent;
use prometheus::core::Collector;
use std::collections::BTreeMap;

/// Core metrics event name
const VELOR_NODE_CORE_METRICS: &str = "VELOR_NODE_CORE_METRICS";

/// Core metric keys
const CONSENSUS_LAST_COMMITTED_ROUND: &str = "consensus_last_committed_round";
const CONSENSUS_PROPOSALS_COUNT: &str = "consensus_proposals_count";
const CONSENSUS_TIMEOUT_COUNT: &str = "consensus_timeout_count";
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
        name: VELOR_NODE_CORE_METRICS.into(),
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
    core_metrics.insert(
        CONSENSUS_PROPOSALS_COUNT.into(),
        velor_consensus::counters::PROPOSALS_COUNT.get().to_string(),
    );
    core_metrics.insert(
        CONSENSUS_LAST_COMMITTED_ROUND.into(),
        velor_consensus::counters::LAST_COMMITTED_ROUND
            .get()
            .to_string(),
    );
    core_metrics.insert(
        CONSENSUS_TIMEOUT_COUNT.into(),
        velor_consensus::counters::TIMEOUT_COUNT.get().to_string(),
    );
    //TODO(joshlind): add block tracing and back pressure!
}

/// Collects the mempool metrics and appends it to the given map
fn collect_mempool_metrics(core_metrics: &mut BTreeMap<String, String>) {
    core_metrics.insert(
        MEMPOOL_CORE_MEMPOOL_INDEX_SIZE.into(),
        velor_mempool::counters::CORE_MEMPOOL_INDEX_SIZE
            .with_label_values(&["system_ttl"])
            .get()
            .to_string(),
    );
}

/// Collects the REST metrics and appends it to the given map
fn collect_rest_metrics(core_metrics: &mut BTreeMap<String, String>) {
    let rest_response_metrics = velor_api::metrics::RESPONSE_STATUS.collect();
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
        velor_state_sync_driver::metrics::STORAGE_SYNCHRONIZER_OPERATIONS
            .with_label_values(&[StorageSynchronizerOperations::SyncedEpoch.get_label()])
            .get()
            .to_string(),
    );
    core_metrics.insert(
        STATE_SYNC_SYNCED_VERSION.into(),
        velor_state_sync_driver::metrics::STORAGE_SYNCHRONIZER_OPERATIONS
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
    core_metrics.insert(
        STORAGE_LEDGER_VERSION.into(),
        velor_db::metrics::LEDGER_VERSION.get().to_string(),
    );
    core_metrics.insert(
        STORAGE_MIN_READABLE_LEDGER_VERSION.into(),
        velor_db::metrics::PRUNER_VERSIONS
            .with_label_values(&["ledger_pruner", "min_readable"])
            .get()
            .to_string(),
    );
    core_metrics.insert(
        STORAGE_MIN_READABLE_STATE_MERKLE_VERSION.into(),
        velor_db::metrics::PRUNER_VERSIONS
            .with_label_values(&["state_merkle_pruner", "min_readable"])
            .get()
            .to_string(),
    );
    core_metrics.insert(
        STORAGE_MIN_READABLE_STATE_KV_VERSION.into(),
        velor_db::metrics::PRUNER_VERSIONS
            .with_label_values(&["state_kv_pruner", "min_readable"])
            .get()
            .to_string(),
    );
    // TODO(joshlind): add storage latencies!
}

/// Collects the telemetry metrics and appends it to the given map
fn collect_telemetry_metrics(core_metrics: &mut BTreeMap<String, String>) {
    let telemetry_failure_metrics = crate::metrics::VELOR_TELEMETRY_FAILURE.collect();
    let telemetry_failure_count = utils::sum_all_gauges(&telemetry_failure_metrics);
    core_metrics.insert(
        TELEMETRY_FAILURE_COUNT.into(),
        telemetry_failure_count.to_string(),
    );

    let telemetry_success_metrics = crate::metrics::VELOR_TELEMETRY_SUCCESS.collect();
    let telemetry_success_count = utils::sum_all_gauges(&telemetry_success_metrics);
    core_metrics.insert(
        TELEMETRY_SUCCESS_COUNT.into(),
        telemetry_success_count.to_string(),
    );
}
