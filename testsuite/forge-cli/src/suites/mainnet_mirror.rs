// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Forge suite that mirrors today's mainnet validator distribution: real
//! per-region counts, real per-validator stake, and tc-netem latency calibrated
//! to the same six-region topology.
//!
//! Reads `mainnet-mirror-data/mainnet_validator_snapshot.json` (produced by
//! `scripts/pull_mainnet_validator_snapshot.py`) and:
//!  - sets the validator count to the snapshot's active count
//!  - injects per-validator stake amounts via the genesis helm chart
//!    (requires the `genesis.validator.stake_amounts` knob added in this branch)
//!  - configures `MultiRegionNetworkEmulationTest` with `region_weights` derived
//!    from the snapshot, overriding the canned `mainnet_calibrated_six_regions`
//!    weights which drift from real mainnet over time.

use aptos_forge::{
    success_criteria::{StateProgressThreshold, SuccessCriteria},
    EmitJobMode, EmitJobRequest, ForgeConfig,
};
use aptos_sdk::types::on_chain_config::{OnChainConsensusConfig, OnChainExecutionConfig};
use aptos_testcases::{
    mainnet_mirror::MainnetMirrorSnapshot,
    multi_region_network_test::{
        MultiRegionNetworkEmulationConfig, MultiRegionNetworkEmulationTest,
    },
    performance_test::PerformanceBenchmark,
    CompositeNetworkTest,
};
use std::{num::NonZeroUsize, sync::Arc, time::Duration};

/// Attempts to match the test name to a mainnet-mirror test.
pub(crate) fn get_mainnet_mirror_test(test_name: &str, duration: Duration) -> Option<ForgeConfig> {
    match test_name {
        "mainnet_mirror_max_load" => Some(mainnet_mirror_max_load_test(duration)),
        _ => None,
    }
}

/// Mainnet-mirror max-load test. Single-cluster forge run with tc-netem
/// simulating mainnet's 6-region topology, sized to mainnet's validator count
/// and stake distribution.
pub(crate) fn mainnet_mirror_max_load_test(duration: Duration) -> ForgeConfig {
    let snapshot = MainnetMirrorSnapshot::load_embedded()
        .expect("embedded mainnet validator snapshot failed to parse");

    let validator_count = snapshot.validator_count();
    let stake_amounts: Vec<u64> = snapshot.stake_amounts();
    let stake_amounts_str = stake_amounts
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>()
        .join(",");

    // Build the multi-region tc-netem config from the snapshot. We start from
    // mainnet_calibrated_six_regions() to keep the per-region link stats and
    // intra/inter-region netem parameters, but override the validator-to-region
    // weighting with the live snapshot.
    let mut region_config = MultiRegionNetworkEmulationConfig::mainnet_calibrated_six_regions();
    region_config.region_weights = Some(snapshot.region_weights());

    let duration_secs = duration.as_secs();
    let success_criteria = SuccessCriteria::new(2000)
        .add_no_restarts()
        .add_wait_for_catchup_s((duration_secs / 10).max(60))
        .add_chain_progress(StateProgressThreshold {
            max_non_epoch_no_progress_secs: 15.0,
            max_epoch_no_progress_secs: 16.0,
            max_non_epoch_round_gap: 6,
            max_epoch_round_gap: 6,
        });

    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(validator_count).unwrap())
        .add_network_test(CompositeNetworkTest::new(
            MultiRegionNetworkEmulationTest::new_with_config(region_config),
            PerformanceBenchmark,
        ))
        // Required for chaos-mesh NetworkChaos to apply: enables headless ClusterIP
        // services so chaos-mesh can resolve pod-level DNS for tc-netem injection.
        // Without this, MultiRegionNetworkEmulationTest's chaos sits in "selecting"
        // forever and the framework's `ensure_chaos_experiments_active` retries
        // until the 30-min timeout, never starting the emit job (mempool stays empty).
        .with_multi_region_config()
        .with_emit_job(
            EmitJobRequest::default()
                .mode(EmitJobMode::ConstTps { tps: 4000 })
                .init_gas_price_multiplier(20),
        )
        .with_genesis_helm_config_fn(Arc::new(move |helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = 300.into();
            helm_values["chain"]["on_chain_consensus_config"] =
                serde_yaml::to_value(OnChainConsensusConfig::default_for_genesis())
                    .expect("must serialize");
            helm_values["chain"]["on_chain_execution_config"] =
                serde_yaml::to_value(OnChainExecutionConfig::default_for_genesis())
                    .expect("must serialize");
            // Per-validator stake — consumed by terraform/helm/genesis/files/genesis.sh
            // (STAKE_AMOUNTS_STRING env var, parsed into a bash array).
            helm_values["genesis"]["validator"]["stake_amounts"] = stake_amounts_str.clone().into();
        }))
        .with_success_criteria(success_criteria)
}
