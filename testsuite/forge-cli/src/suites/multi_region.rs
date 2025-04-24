// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::suites::{
    state_sync::state_sync_config_execute_transactions,
    ungrouped::{
        PROGRESS_THRESHOLD_20_6, RELIABLE_PROGRESS_THRESHOLD, SYSTEM_12_CORES_10GB_THRESHOLD,
    },
};
use aptos_config::{
    config::{DiscoveryMethod, Identity, NetworkConfig},
    network_id::NetworkId,
};
use aptos_forge::{
    success_criteria::SuccessCriteria, EmitJobMode, EmitJobRequest, ForgeConfig, NetworkTest,
    NodeResourceOverride,
};
use aptos_testcases::{
    modifiers::{ExecutionDelayConfig, ExecutionDelayTest},
    multi_region_network_test::{
        MultiRegionNetworkEmulationConfig, MultiRegionNetworkEmulationTest,
    },
    performance_test::{ConsensusOnlyBenchmark, PerformanceBenchmark},
    three_region_simulation_test::ThreeRegionSameCloudSimulationTest,
    CompositeNetworkTest,
};
use std::{num::NonZeroUsize, sync::Arc, time::Duration};

/// Attempts to match the test name to a multi-region test
pub(crate) fn get_multi_region_test(
    test_name: &str,
    duration: Duration,
    num_fullnodes: usize,
) -> Option<ForgeConfig> {
    let test = match test_name {
        "multiregion_benchmark_test" => multiregion_benchmark_test(duration, num_fullnodes),
        "three_region_simulation" => three_region_simulation(),
        "three_region_simulation_with_different_node_speed" => {
            three_region_simulation_with_different_node_speed()
        },
        _ => return None, // The test name does not match a multi-region test
    };
    Some(test)
}

/// This test runs a network test in a real multi-region setup. It configures
/// genesis and node helm values to enable certain configurations needed to run in
/// the multiregion forge cluster.
pub(crate) fn multiregion_benchmark_test(duration: Duration, num_fullnodes: usize) -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(10).unwrap())
        .add_network_test(ConsensusOnlyBenchmark {
            test_time: duration,
            // Repurpose num_fullnodes to set concurrency
            concurrency: num_fullnodes,
        })
        .with_validator_override_node_config_fn(Arc::new(|config, base| {
            base.validator_network2 = Some(NetworkConfig::default());
            base.validator_network3 = Some(NetworkConfig::default());

            let mut net2config = NetworkConfig::default();
            net2config.listen_address = "/ip4/0.0.0.0/tcp/12000".parse().unwrap();
            net2config.mutual_authentication = true;
            net2config.identity =
                Identity::from_file("/opt/aptos/genesis/validator-identity.yaml".into());
            net2config.discovery_method = DiscoveryMethod::Onchain;
            net2config.network_id = NetworkId::Validator;
            config.validator_network2 = Some(net2config);

            let mut net3config = NetworkConfig::default();
            net3config.listen_address = "/ip4/0.0.0.0/tcp/12001".parse().unwrap();
            net3config.mutual_authentication = true;
            net3config.identity =
                Identity::from_file("/opt/aptos/genesis/validator-identity.yaml".into());
            net3config.discovery_method = DiscoveryMethod::Onchain;
            net3config.network_id = NetworkId::Validator;
            config.validator_network3 = Some(net3config);

            config.consensus.quorum_store.sender_max_batch_txns = 300;
            config.consensus.quorum_store.sender_max_total_txns = 300;
            config.consensus.quorum_store.memory_quota = 300_000_000;
            config.consensus.quorum_store.db_quota = 400_000_000;

            config.consensus.quorum_store.opt_qs_minimum_batch_age_usecs =
                Duration::from_millis(30).as_micros() as u64;
        }))
        .with_validator_resource_override(NodeResourceOverride {
            cpu_cores: Some(62),
            memory_gib: Some(240),
            storage_gib: Some(2500),
        })
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            // Have single epoch change in land blocking
            helm_values["chain"]["epoch_duration_secs"] = 3600.into();
        }))
}

pub(crate) fn three_region_simulation_with_different_node_speed() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(30).unwrap())
        .with_initial_fullnode_count(30)
        .with_emit_job(EmitJobRequest::default().mode(EmitJobMode::ConstTps { tps: 5000 }))
        .add_network_test(CompositeNetworkTest::new(
            ExecutionDelayTest {
                add_execution_delay: ExecutionDelayConfig {
                    inject_delay_node_fraction: 0.5,
                    inject_delay_max_transaction_percentage: 40,
                    inject_delay_per_transaction_ms: 2,
                },
            },
            ThreeRegionSameCloudSimulationTest,
        ))
        .with_validator_override_node_config_fn(Arc::new(|config, _| {
            config.api.failpoints_enabled = true;
        }))
        .with_fullnode_override_node_config_fn(Arc::new(|config, _| {
            state_sync_config_execute_transactions(&mut config.state_sync);
        }))
        .with_success_criteria(
            SuccessCriteria::new(1000)
                .add_no_restarts()
                .add_wait_for_catchup_s(240)
                .add_chain_progress(PROGRESS_THRESHOLD_20_6.clone()),
        )
}

pub(crate) fn three_region_simulation() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(12).unwrap())
        .with_initial_fullnode_count(12)
        .with_emit_job(EmitJobRequest::default().mode(EmitJobMode::ConstTps { tps: 5000 }))
        .add_network_test(ThreeRegionSameCloudSimulationTest)
        // TODO(rustielin): tune these success criteria after we have a better idea of the test behavior
        .with_success_criteria(
            SuccessCriteria::new(3000)
                .add_no_restarts()
                .add_wait_for_catchup_s(240)
                .add_chain_progress(PROGRESS_THRESHOLD_20_6.clone()),
        )
}

pub fn wrap_with_two_region_env<T: NetworkTest + 'static>(test: T) -> CompositeNetworkTest {
    CompositeNetworkTest::new(
        MultiRegionNetworkEmulationTest::new_with_config(
            MultiRegionNetworkEmulationConfig::two_region(),
        ),
        test,
    )
}
