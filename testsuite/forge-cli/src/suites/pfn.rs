// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::ungrouped::RELIABLE_PROGRESS_THRESHOLD;
use velor_config::config::NodeConfig;
use velor_forge::{
    success_criteria::{LatencyType, SuccessCriteria},
    EmitJobMode, EmitJobRequest, ForgeConfig, OverrideNodeConfigFn,
};
use velor_testcases::public_fullnode_performance::PFNPerformance;
use std::{num::NonZeroUsize, sync::Arc, time::Duration};

/// Attempts to match the test name to a PFN test
pub fn get_pfn_test(test_name: &str, duration: Duration) -> Option<ForgeConfig> {
    let test = match test_name {
        "pfn_const_tps" => pfn_const_tps(duration, false, false, true),
        "pfn_const_tps_with_network_chaos" => pfn_const_tps(duration, false, true, false),
        "pfn_const_tps_with_realistic_env" => pfn_const_tps(duration, true, true, false),
        "pfn_performance" => pfn_performance(duration, false, false, true, 7, 1, false),
        "pfn_performance_with_network_chaos" => {
            pfn_performance(duration, false, true, false, 7, 1, false)
        },
        "pfn_performance_with_realistic_env" => {
            pfn_performance(duration, true, true, false, 7, 1, false)
        },
        "pfn_spam_duplicates" => pfn_performance(duration, true, true, true, 7, 7, true),
        _ => return None, // The test name does not match a PFN test
    };
    Some(test)
}

/// This test runs a constant-TPS benchmark where the network includes
/// PFNs, and the transactions are submitted to the PFNs. This is useful
/// for measuring latencies when the system is not saturated.
///
/// Note: If `add_cpu_chaos` is true, CPU chaos is enabled on the entire swarm.
/// Likewise, if `add_network_emulation` is true, network chaos is enabled.
fn pfn_const_tps(
    duration: Duration,
    add_cpu_chaos: bool,
    add_network_emulation: bool,
    epoch_changes: bool,
) -> ForgeConfig {
    let epoch_duration_secs = if epoch_changes {
        300 // 5 minutes
    } else {
        60 * 60 * 2 // 2 hours; avoid epoch changes which can introduce noise
    };

    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(7).unwrap())
        .with_initial_fullnode_count(7)
        .with_emit_job(EmitJobRequest::default().mode(EmitJobMode::ConstTps { tps: 5000 }))
        .add_network_test(PFNPerformance::new(
            7,
            add_cpu_chaos,
            add_network_emulation,
            Some(Arc::new(|config: &mut NodeConfig, _| {
                config.indexer_db_config.enable_event = true;
            })),
        ))
        .with_genesis_helm_config_fn(Arc::new(move |helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = epoch_duration_secs.into();
        }))
        .with_success_criteria(
            SuccessCriteria::new(4500)
                .add_no_restarts()
                .add_max_expired_tps(0.0)
                .add_max_failed_submission_tps(0.0)
                // Percentile thresholds are estimated and should be revisited.
                .add_latency_threshold(3.5, LatencyType::P50)
                .add_latency_threshold(4.5, LatencyType::P90)
                .add_latency_threshold(5.5, LatencyType::P99)
                .add_wait_for_catchup_s(
                    // Give at least 60s for catchup and at most 10% of the run
                    (duration.as_secs() / 10).max(60),
                )
                .add_chain_progress(RELIABLE_PROGRESS_THRESHOLD.clone()),
        )
}

/// This test runs a performance benchmark where the network includes
/// PFNs, and the transactions are submitted to the PFNs. This is useful
/// for measuring maximum throughput and latencies.
///
/// Note: If `add_cpu_chaos` is true, CPU chaos is enabled on the entire swarm.
/// Likewise, if `add_network_emulation` is true, network chaos is enabled.
fn pfn_performance(
    duration: Duration,
    add_cpu_chaos: bool,
    add_network_emulation: bool,
    epoch_changes: bool,
    num_validators: usize,
    num_pfns: usize,
    broadcast_to_all_vfns: bool,
) -> ForgeConfig {
    // Determine the minimum expected TPS
    let min_expected_tps = if broadcast_to_all_vfns { 2500 } else { 4500 };
    let epoch_duration_secs = if epoch_changes {
        300 // 5 minutes
    } else {
        60 * 60 * 2 // 2 hours; avoid epoch changes which can introduce noise
    };

    let config_override_fn = if broadcast_to_all_vfns {
        let f: OverrideNodeConfigFn = Arc::new(move |pfn_config: &mut NodeConfig, _| {
            pfn_config.mempool.default_failovers = num_validators;
            for network in &mut pfn_config.full_node_networks {
                network.max_outbound_connections = num_validators;
            }
        });
        Some(f)
    } else {
        None
    };

    // Create the forge config
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(num_validators).unwrap())
        .with_initial_fullnode_count(num_validators)
        .add_network_test(PFNPerformance::new(
            num_pfns as u64,
            add_cpu_chaos,
            add_network_emulation,
            config_override_fn,
        ))
        .with_genesis_helm_config_fn(Arc::new(move |helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = epoch_duration_secs.into();
        }))
        .with_success_criteria(
            SuccessCriteria::new(min_expected_tps)
                .add_no_restarts()
                .add_wait_for_catchup_s(
                    // Give at least 60s for catchup and at most 10% of the run
                    (duration.as_secs() / 10).max(60),
                )
                .add_chain_progress(RELIABLE_PROGRESS_THRESHOLD.clone()),
        )
}
