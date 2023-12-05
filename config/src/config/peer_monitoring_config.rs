// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::config::{
    config_optimizer::ConfigOptimizer, config_sanitizer::ConfigSanitizer,
    node_config_loader::NodeType, utils::is_network_perf_test_enabled, Error, NodeConfig,
};
use aptos_types::chain_id::ChainId;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct PeerMonitoringServiceConfig {
    pub enable_peer_monitoring_client: bool, // Whether or not to spawn the monitoring client
    pub latency_monitoring: LatencyMonitoringConfig,
    pub max_concurrent_requests: u64, // Max num of concurrent server tasks
    pub max_network_channel_size: u64, // Max num of pending network messages
    pub max_num_response_bytes: u64,  // Max num of bytes in a (serialized) response
    pub max_request_jitter_ms: u64, // Max amount of jitter (ms) that a request will be delayed for
    pub metadata_update_interval_ms: u64, // The interval (ms) between metadata updates
    pub network_monitoring: NetworkMonitoringConfig,
    pub node_monitoring: NodeMonitoringConfig,
    pub peer_monitor_interval_usec: u64, // The interval (usec) between peer monitor executions
    pub performance_monitoring: PerformanceMonitoringConfig,
}

impl Default for PeerMonitoringServiceConfig {
    fn default() -> Self {
        Self {
            enable_peer_monitoring_client: true,
            latency_monitoring: LatencyMonitoringConfig::default(),
            max_concurrent_requests: 1000,
            max_network_channel_size: 1000,
            max_num_response_bytes: 100 * 1024, // 100 KB
            max_request_jitter_ms: 1000,        // Monitoring requests are very infrequent
            metadata_update_interval_ms: 5000,  // 5 seconds
            network_monitoring: NetworkMonitoringConfig::default(),
            node_monitoring: NodeMonitoringConfig::default(),
            peer_monitor_interval_usec: 1_000_000, // 1 second
            performance_monitoring: PerformanceMonitoringConfig::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct LatencyMonitoringConfig {
    pub latency_ping_interval_ms: u64, // The interval (ms) between latency pings for each peer
    pub latency_ping_timeout_ms: u64,  // The timeout (ms) for each latency ping
    pub max_latency_ping_failures: u64, // Max ping failures before the peer connection fails
    pub max_num_latency_pings_to_retain: usize, // The max latency pings to retain per peer
}

impl Default for LatencyMonitoringConfig {
    fn default() -> Self {
        Self {
            latency_ping_interval_ms: 30_000, // 30 seconds
            latency_ping_timeout_ms: 20_000,  // 20 seconds
            max_latency_ping_failures: 3,
            max_num_latency_pings_to_retain: 10,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct NetworkMonitoringConfig {
    pub network_info_request_interval_ms: u64, // The interval (ms) between network info requests
    pub network_info_request_timeout_ms: u64,  // The timeout (ms) for each network info request
}

impl Default for NetworkMonitoringConfig {
    fn default() -> Self {
        Self {
            network_info_request_interval_ms: 60_000, // 1 minute
            network_info_request_timeout_ms: 10_000,  // 10 seconds
        }
    }
}

// TODO: add support for direct send test mode!

// Note: to enable performance monitoring, the compilation feature "network-perf-test" is required.
// Simply enabling the config values here will not enable performance monitoring.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct PerformanceMonitoringConfig {
    pub enable_direct_send_testing: bool, // Whether or not to enable direct send test mode
    pub direct_send_data_size: u64,       // The amount of data to send in each request
    pub direct_send_interval_usec: u64,   // The interval (microseconds) between requests
    pub enable_rpc_testing: bool,         // Whether or not to enable RPC test mode
    pub rpc_data_size: u64,               // The amount of data to send in each RPC request
    pub rpc_interval_usec: u64,           // The interval (microseconds) between RPC requests
    pub rpc_timeout_ms: u64,              // The timeout (ms) for each RPC request
}

impl Default for PerformanceMonitoringConfig {
    fn default() -> Self {
        Self {
            enable_direct_send_testing: false,    // Disabled by default
            direct_send_data_size: 512 * 1024,    // 512 KB
            direct_send_interval_usec: 2_000_000, // 2 seconds
            enable_rpc_testing: false,            // Disabled by default
            rpc_data_size: 512 * 1024,            // 512 KB
            rpc_interval_usec: 2_000_000,         // 2 seconds
            rpc_timeout_ms: 10_000,               // 10 seconds
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct NodeMonitoringConfig {
    pub node_info_request_interval_ms: u64, // The interval (ms) between node info requests
    pub node_info_request_timeout_ms: u64,  // The timeout (ms) for each node info request
}

impl Default for NodeMonitoringConfig {
    fn default() -> Self {
        Self {
            node_info_request_interval_ms: 20_000, // 20 seconds
            node_info_request_timeout_ms: 10_000,  // 10 seconds
        }
    }
}

impl ConfigSanitizer for PeerMonitoringServiceConfig {
    fn sanitize(
        node_config: &NodeConfig,
        node_type: NodeType,
        chain_id: Option<ChainId>,
    ) -> Result<(), Error> {
        // Sanitize the performance monitoring config
        PerformanceMonitoringConfig::sanitize(node_config, node_type, chain_id)
    }
}

impl ConfigSanitizer for PerformanceMonitoringConfig {
    fn sanitize(
        node_config: &NodeConfig,
        _node_type: NodeType,
        chain_id: Option<ChainId>,
    ) -> Result<(), Error> {
        let sanitizer_name = Self::get_sanitizer_name();
        let performance_monitoring_config =
            &node_config.peer_monitoring_service.performance_monitoring;

        // Verify that performance monitoring is not enabled in mainnet
        let enable_direct_send_testing = performance_monitoring_config.enable_direct_send_testing;
        let enable_rpc_testing = performance_monitoring_config.enable_rpc_testing;
        if let Some(chain_id) = chain_id {
            if chain_id.is_mainnet()
                && (is_network_perf_test_enabled()
                    || enable_direct_send_testing
                    || enable_rpc_testing)
            {
                return Err(Error::ConfigSanitizerFailed(
                    sanitizer_name,
                    "Performance monitoring should not be enabled in mainnet!".into(),
                ));
            };
        }

        // Verify that at least one performance monitoring mode is enabled if the feature exists
        if is_network_perf_test_enabled() && !enable_direct_send_testing && !enable_rpc_testing {
            return Err(Error::ConfigSanitizerFailed(
                sanitizer_name,
                "At least one performance monitoring mode must be enabled!".into(),
            ));
        }

        // Verify that the peer monitor loop interval is valid (with respect to the request intervals)
        if is_network_perf_test_enabled() {
            let peer_monitor_interval_usec = node_config
                .peer_monitoring_service
                .peer_monitor_interval_usec;
            let direct_send_interval_usec = performance_monitoring_config.direct_send_interval_usec;
            let rpc_interval_usec = performance_monitoring_config.rpc_interval_usec;

            // Verify that the peer monitor loop interval is <= the direct send interval
            if enable_direct_send_testing
                && (peer_monitor_interval_usec > direct_send_interval_usec)
            {
                return Err(Error::ConfigSanitizerFailed(
                    sanitizer_name,
                    "The peer monitor loop interval must be <= the direct send interval!".into(),
                ));
            }

            // Verify that the peer monitor loop interval is <= the RPC interval
            if enable_rpc_testing && (peer_monitor_interval_usec > rpc_interval_usec) {
                return Err(Error::ConfigSanitizerFailed(
                    sanitizer_name,
                    "The peer monitor loop interval must be <= the RPC interval!".into(),
                ));
            }
        }

        Ok(())
    }
}

impl ConfigOptimizer for PeerMonitoringServiceConfig {
    fn optimize(
        node_config: &mut NodeConfig,
        local_config_yaml: &Value,
        node_type: NodeType,
        chain_id: Option<ChainId>,
    ) -> Result<bool, Error> {
        let peer_monitoring_config = &mut node_config.peer_monitoring_service;
        let local_monitoring_config_yaml = &local_config_yaml["peer_monitoring_service"];

        // Increase the max number of message bytes if the network-perf-test feature is enabled
        let mut modified_config = false;
        if local_monitoring_config_yaml["max_num_response_bytes"].is_null()
            && is_network_perf_test_enabled()
        {
            peer_monitoring_config.max_num_response_bytes = 10 * 1024 * 1024; // 100 MB
            modified_config = true;
        }

        // Optimize the performance monitoring config
        modified_config = modified_config
            || PerformanceMonitoringConfig::optimize(
                node_config,
                local_config_yaml,
                node_type,
                chain_id,
            )?;

        Ok(modified_config)
    }
}

impl ConfigOptimizer for PerformanceMonitoringConfig {
    fn optimize(
        node_config: &mut NodeConfig,
        local_config_yaml: &Value,
        _node_type: NodeType,
        _chain_id: Option<ChainId>,
    ) -> Result<bool, Error> {
        let performance_monitoring_config =
            &mut node_config.peer_monitoring_service.performance_monitoring;
        let local_performance_config_yaml =
            &local_config_yaml["peer_monitoring_service"]["performance_monitoring"];

        // Enable RPC testing if the network-perf-test feature is enabled
        let mut modified_config = false;
        if local_performance_config_yaml["enable_rpc_testing"].is_null()
            && is_network_perf_test_enabled()
        {
            performance_monitoring_config.enable_rpc_testing = true;
            modified_config = true;
        }

        Ok(modified_config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimize_enable_monitoring_client_mainnet() {
        // Create a node config with the peer monitoring client disabled
        let node_config = create_config_with_disabled_client();

        // Test that the peer monitoring client is not enabled for testnet and mainnet
        for chain_id in &[ChainId::testnet(), ChainId::mainnet()] {
            // Optimize the config and verify no modifications are made
            let modified_config = PeerMonitoringServiceConfig::optimize(
                &mut node_config.clone(),
                &serde_yaml::from_str("{}").unwrap(), // An empty local config,
                NodeType::Validator,
                Some(*chain_id),
            )
            .unwrap();
            assert!(!modified_config);

            // Verify that the peer monitoring client is disabled
            assert!(
                !node_config
                    .peer_monitoring_service
                    .enable_peer_monitoring_client
            );
        }
    }

    #[test]
    fn test_optimize_enable_monitoring_client_no_override() {
        // Create a node config with the peer monitoring client disabled
        let node_config = create_config_with_disabled_client();

        // Create a local config YAML with the peer monitoring client disabled
        let local_config_yaml = serde_yaml::from_str(
            r#"
            peer_monitoring_service:
                enable_peer_monitoring_client: false
            "#,
        )
        .unwrap();

        // Optimize the config and verify no modifications are made
        let modified_config = PeerMonitoringServiceConfig::optimize(
            &mut node_config.clone(),
            &local_config_yaml,
            NodeType::PublicFullnode,
            Some(ChainId::test()),
        )
        .unwrap();
        assert!(!modified_config);

        // Verify that the peer monitoring client is still disabled
        assert!(
            !node_config
                .peer_monitoring_service
                .enable_peer_monitoring_client
        );
    }

    /// Creates a node config with the peer monitoring client disabled
    fn create_config_with_disabled_client() -> NodeConfig {
        NodeConfig {
            peer_monitoring_service: PeerMonitoringServiceConfig {
                enable_peer_monitoring_client: false,
                ..Default::default()
            },
            ..Default::default()
        }
    }
}
