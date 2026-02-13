// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Prometheus metrics for Forge test framework.
//!
//! These metrics track cluster spin-up performance and can be pushed to
//! vmagent or any Prometheus-compatible endpoint using the `PUSH_METRICS_ENDPOINT`
//! environment variable.

use aptos_metrics_core::{
    exponential_buckets, register_histogram_vec, register_int_counter_vec, HistogramVec,
    IntCounterVec,
};
use once_cell::sync::Lazy;
use std::time::Instant;

/// Cluster spin-up phase names for metrics labeling.
#[derive(Debug, Clone, Copy)]
pub enum ClusterPhase {
    /// Deleting old K8s resources
    Cleanup,
    /// Installing testnet resources (validators, fullnodes)
    TestnetInstall,
    /// Deploying indexer stack
    IndexerDeploy,
    /// Waiting for nodes to become healthy
    HealthCheck,
    /// Total end-to-end spin-up time
    Total,
}

impl ClusterPhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            ClusterPhase::Cleanup => "cleanup",
            ClusterPhase::TestnetInstall => "testnet_install",
            ClusterPhase::IndexerDeploy => "indexer_deploy",
            ClusterPhase::HealthCheck => "health_check",
            ClusterPhase::Total => "total",
        }
    }
}

/// Histogram for cluster spin-up duration in seconds, by phase.
/// Labels: namespace, phase, success
pub static FORGE_CLUSTER_SPINUP_DURATION_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_forge_cluster_spinup_duration_seconds",
        "Duration of cluster spin-up phases in seconds",
        &["namespace", "phase", "success"],
        // Buckets from 1s to ~17 minutes (1024s)
        exponential_buckets(1.0, 2.0, 11).unwrap()
    )
    .unwrap()
});

/// Counter for cluster spin-up attempts, by phase.
/// Labels: namespace, phase, success
pub static FORGE_CLUSTER_SPINUP_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_forge_cluster_spinup_total",
        "Total count of cluster spin-up phase attempts",
        &["namespace", "phase", "success"]
    )
    .unwrap()
});

/// Records metrics for a cluster spin-up phase.
///
/// # Arguments
/// * `namespace` - K8s namespace for the cluster
/// * `phase` - The spin-up phase being measured
/// * `start` - When the phase started
/// * `success` - Whether the phase succeeded
pub fn record_cluster_spinup_phase(
    namespace: &str,
    phase: ClusterPhase,
    start: Instant,
    success: bool,
) {
    let duration = start.elapsed().as_secs_f64();
    let success_str = if success { "true" } else { "false" };

    FORGE_CLUSTER_SPINUP_DURATION_SECONDS
        .with_label_values(&[namespace, phase.as_str(), success_str])
        .observe(duration);

    FORGE_CLUSTER_SPINUP_TOTAL
        .with_label_values(&[namespace, phase.as_str(), success_str])
        .inc();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_phase_as_str() {
        assert_eq!(ClusterPhase::Cleanup.as_str(), "cleanup");
        assert_eq!(ClusterPhase::TestnetInstall.as_str(), "testnet_install");
        assert_eq!(ClusterPhase::IndexerDeploy.as_str(), "indexer_deploy");
        assert_eq!(ClusterPhase::HealthCheck.as_str(), "health_check");
        assert_eq!(ClusterPhase::Total.as_str(), "total");
    }

    #[test]
    fn test_record_cluster_spinup_phase() {
        let start = Instant::now();
        // Just verify it doesn't panic
        record_cluster_spinup_phase("test-namespace", ClusterPhase::Cleanup, start, true);
        record_cluster_spinup_phase("test-namespace", ClusterPhase::TestnetInstall, start, false);
    }
}
