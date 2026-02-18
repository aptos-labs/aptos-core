// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Forge test suites for Proxy Primary Consensus.

use aptos_forge::{success_criteria::SuccessCriteria, ForgeConfig};
use aptos_testcases::proxy_primary_test::{ProxyPrimaryHappyPathTest, ProxyPrimaryLoadTest};
use std::{num::NonZeroUsize, sync::Arc, time::Duration};

/// Get a proxy consensus test by name.
pub fn get_proxy_test(test_name: &str, _duration: Duration) -> Option<ForgeConfig> {
    let test = match test_name {
        "proxy_primary_happy_path" => proxy_primary_happy_path_test(),
        "proxy_primary_load" => proxy_primary_load_test(),
        _ => return None,
    };
    Some(test)
}

/// Basic happy path test for proxy primary consensus.
///
/// 7 validators with realistic network topology:
/// - 4 proxy validators co-located in EU (eu-west2, ~5ms intra-region)
/// - 3 validators geo-distributed (eu-west6, us-east4, as-southeast1)
/// - Inter-region latencies from real cloud measurements (8.5-106ms one-way)
/// - 3% packet loss, 300 Mbps bandwidth cap between regions
///
/// Uses the same multi-region network emulation framework as the land
/// blocking test (four_region_link_stats.csv).
pub fn proxy_primary_happy_path_test() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(7).unwrap())
        .add_network_test(ProxyPrimaryHappyPathTest::new(4))
        .with_validator_override_node_config_fn(Arc::new(|config, _| {
            config.consensus.enable_proxy_consensus = true;
            // Proxy at 1s (matching original consensus default), primary at 10s
            // to ensure proxy accumulates multiple blocks per primary round.
            config.consensus.proxy_consensus_config.round_initial_timeout_ms = 1000;
            config.consensus.round_initial_timeout_ms = 10000;
        }))
        .with_success_criteria(
            SuccessCriteria::new(10)
                .add_no_restarts()
                .add_wait_for_catchup_s(30),
        )
}

/// Load test for proxy primary consensus.
///
/// 6 validators: 2 proxy+primary, 4 primary-only.
/// No network emulation (all local).
fn proxy_primary_load_test() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(6).unwrap())
        .add_network_test(ProxyPrimaryLoadTest::new(2))
        .with_validator_override_node_config_fn(Arc::new(|config, _| {
            config.consensus.enable_proxy_consensus = true;
            config.consensus.proxy_consensus_config.round_initial_timeout_ms = 2000;
            config.consensus.round_initial_timeout_ms = 10000;
        }))
        // Simple success criteria for local swarm (no Prometheus needed)
        .with_success_criteria(
            SuccessCriteria::new(10) // Low TPS threshold for Phase 1 testing
                .add_no_restarts()
                .add_wait_for_catchup_s(30),
        )
}
