use crate::{
    core_metrics::get_core_metrics,
    network_metrics::get_network_metrics,
    system_information::get_system_information,
};
use aptos_config::config::NodeConfig;
use std::collections::BTreeMap;

#[test]
fn test_core_metrics_collection() {
    let node_config = NodeConfig::default();
    let core_metrics = get_core_metrics(&node_config);
    
    assert!(!core_metrics.is_empty(), "Core metrics should not be empty");
    
    for (key, value) in &core_metrics {
        println!("Core metric: {} = {}", key, value);
    }
    
    verify_metric_key_exists(&core_metrics, "consensus_proposals_count");
    verify_metric_key_exists(&core_metrics, "consensus_last_committed_round");
    verify_metric_key_exists(&core_metrics, "consensus_timeout_count");
    verify_metric_key_exists(&core_metrics, "consensus_last_committed_version");
    verify_metric_key_exists(&core_metrics, "consensus_committed_blocks_count");
    verify_metric_key_exists(&core_metrics, "consensus_committed_txns_count");
    verify_metric_key_exists(&core_metrics, "consensus_round_timeout_secs");
    verify_metric_key_exists(&core_metrics, "consensus_sync_info_msg_sent_count");
    verify_metric_key_exists(&core_metrics, "consensus_current_round");
    verify_metric_key_exists(&core_metrics, "consensus_wait_duration_s");
    
    verify_metric_key_exists(&core_metrics, "mempool_core_mempool_index_size");
    verify_metric_key_exists(&core_metrics, "mempool_txns_processed_success");
    verify_metric_key_exists(&core_metrics, "mempool_txns_processed_total");
    verify_metric_key_exists(&core_metrics, "mempool_avg_txn_broadcast_size");
    verify_metric_key_exists(&core_metrics, "mempool_pending_txns");
    
    verify_metric_key_exists(&core_metrics, "storage_ledger_version");
    verify_metric_key_exists(&core_metrics, "storage_min_readable_ledger_version");
    verify_metric_key_exists(&core_metrics, "storage_min_readable_state_merkle_version");
    verify_metric_key_exists(&core_metrics, "storage_min_readable_state_kv_version");
    verify_metric_key_exists(&core_metrics, "storage_get_transaction_latency_s");
    verify_metric_key_exists(&core_metrics, "storage_commit_latency_s");
    verify_metric_key_exists(&core_metrics, "storage_save_transactions_latency_s");
    
    verify_metric_key_exists(&core_metrics, "role_type");
}

#[test]
fn test_network_metrics_collection() {
    let network_metrics = get_network_metrics();
    

    for (key, value) in &network_metrics {
        println!("Network metric: {} = {}", key, value);
    }
}

#[test]
fn test_system_information_collection() {
    let system_information = get_system_information();
    
    assert!(!system_information.is_empty(), "System information should not be empty");
    
    for (key, value) in &system_information {
        println!("System info: {} = {}", key, value);
    }
    
    verify_metric_key_exists(&system_information, "cpu_count");
    verify_metric_key_exists(&system_information, "memory_total");
    verify_metric_key_exists(&system_information, "system_name");
}

#[test]
fn test_all_telemetry_metrics() {
    let node_config = NodeConfig::default();
    
    let core_metrics = get_core_metrics(&node_config);
    let network_metrics = get_network_metrics();
    let system_information = get_system_information();
    
    assert!(!core_metrics.is_empty(), "Core metrics should not be empty");
    assert!(!system_information.is_empty(), "System information should not be empty");
    
    println!(
        "Total metrics collected: {} (Core: {}, Network: {}, System: {})",
        core_metrics.len() + network_metrics.len() + system_information.len(),
        core_metrics.len(),
        network_metrics.len(),
        system_information.len()
    );
}

fn verify_metric_key_exists(metrics: &BTreeMap<String, String>, key: &str) {
    assert!(
        metrics.contains_key(key),
        "Expected metric key '{}' not found in metrics",
        key
    );
} 