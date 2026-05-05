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
    mainnet_mirror_failure_test::{MainnetMirrorFailureTest, MultiRegionChaosNoCleanup},
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
        "mainnet_mirror_max_load_small" => Some(mainnet_mirror_max_load_small_test(duration)),
        "mainnet_mirror_failures" => Some(mainnet_mirror_failures_test(duration)),
        "mainnet_mirror_failures_small" => Some(mainnet_mirror_failures_small_test(duration)),
        _ => None,
    }
}

/// Stratified 21-validator subset used by the `_small` test variants. Picks
/// deterministically from the embedded snapshot so both small tests run
/// against the exact same validator set (one with failure injection, one
/// without). 4 non-Healthy + 17 Healthy distributed proportional to mainnet
/// stake fractions; covers all 6 regions.
fn small_stratified_subset() -> MainnetMirrorSnapshot {
    use aptos_testcases::mainnet_mirror::{AvailabilityClass::*, Region::*};
    MainnetMirrorSnapshot::load_embedded()
        .expect("embedded mainnet validator snapshot failed to parse")
        .stratified_subset(&[
            (StableChronic, UsCentral1, 1),
            (OnlineButFlaky, EuCentral1, 1),
            (OnlineButFlaky, SaEast1, 1),
            (EpisodicSpike, EuWest1, 1),
            (Healthy, Apne1, 1),
            (Healthy, EuCentral1, 4),
            (Healthy, EuWest1, 7),
            (Healthy, CaCentral1, 2),
            (Healthy, UsCentral1, 3),
        ])
}

/// Mainnet-mirror max-load test. Single-cluster forge run with tc-netem
/// simulating mainnet's 6-region topology, sized to mainnet's validator count
/// and stake distribution. No failure injection — provides a baseline to
/// compare `mainnet_mirror_failures` against.
pub(crate) fn mainnet_mirror_max_load_test(duration: Duration) -> ForgeConfig {
    let snapshot = MainnetMirrorSnapshot::load_embedded()
        .expect("embedded mainnet validator snapshot failed to parse");
    build_max_load_test(snapshot, duration, 4000, 2000)
}

/// Small-scale variant of `mainnet_mirror_max_load_test` on the stratified
/// 21-validator subset. Pairs with `mainnet_mirror_failures_small_test`:
/// run both back-to-back to validate the multi-region netem plumbing and
/// then the failpoint-based failure injection on the same validator set.
pub(crate) fn mainnet_mirror_max_load_small_test(duration: Duration) -> ForgeConfig {
    build_max_load_test(small_stratified_subset(), duration, 200, 100)
}

/// Shared body of both max-load tests; differs only by snapshot subset, emit
/// TPS target, and success-criteria TPS floor.
fn build_max_load_test(
    snapshot: MainnetMirrorSnapshot,
    duration: Duration,
    emit_tps: usize,
    min_tps: usize,
) -> ForgeConfig {
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
    let success_criteria = SuccessCriteria::new(min_tps)
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
        .with_emit_job(
            EmitJobRequest::default()
                .mode(EmitJobMode::ConstTps { tps: emit_tps })
                .init_gas_price_multiplier(20),
        )
        .with_genesis_helm_config_fn(Arc::new(move |helm_values| {
            // Long epoch — keep validator-set stable for the full run so we measure
            // the snapshot-derived shape, not partial-snapshot mid-epoch behavior.
            helm_values["chain"]["epoch_duration_secs"] = 7200.into();
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

/// Small-scale variant of `mainnet_mirror_failures_test`: takes a stratified
/// 21-validator subset of the embedded snapshot designed to exercise every
/// failure-injection code path (chronic/flaky/spike) and every region pair
/// in the inter-region netem. Used to validate the failpoint plumbing
/// end-to-end in ~10 min before committing to a full 50-min run on the
/// full 100+ validator snapshot.
pub(crate) fn mainnet_mirror_failures_small_test(duration: Duration) -> ForgeConfig {
    build_failures_test(small_stratified_subset(), duration, 200, 100)
}

/// Mainnet-mirror with failure-pattern injection. Applies per-validator
/// failpoints matching each validator's real mainnet `fp_7d_avg` (chronic /
/// flaky get continuous `consensus::send::any` delay; spike validators get
/// a one-shot 100%-return failpoint at a randomized offset). Loosens success
/// criteria modestly to account for the ~0.3-0.5% failed-round rate this
/// produces.
pub(crate) fn mainnet_mirror_failures_test(duration: Duration) -> ForgeConfig {
    let snapshot = MainnetMirrorSnapshot::load_embedded()
        .expect("embedded mainnet validator snapshot failed to parse");
    build_failures_test(snapshot, duration, 2000, 1500)
}

/// Shared body of both failures tests; differs only by snapshot subset, emit
/// TPS target, and success-criteria TPS floor.
fn build_failures_test(
    snapshot: MainnetMirrorSnapshot,
    duration: Duration,
    emit_tps: usize,
    min_tps: usize,
) -> ForgeConfig {
    let validator_count = snapshot.validator_count();
    let stake_amounts_str = snapshot
        .stake_amounts()
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>()
        .join(",");

    let mut region_config = MultiRegionNetworkEmulationConfig::mainnet_calibrated_six_regions();
    region_config.region_weights = Some(snapshot.region_weights());

    // Failure-pattern injection consumes the same snapshot — `availability_class`
    // determines whether each validator gets continuous packet loss (chronic / flaky)
    // or a one-shot spike event, and `fp_7d_avg` calibrates the loss percentage.
    let failure_test = MainnetMirrorFailureTest::new(snapshot);

    let duration_secs = duration.as_secs();
    let success_criteria = SuccessCriteria::new(min_tps)
        .add_no_restarts() // chaos-mesh loss doesn't restart pods; checks framework-level restarts
        .add_wait_for_catchup_s((duration_secs / 6).max(120))
        .add_chain_progress(StateProgressThreshold {
            // Mildly loosened from no-failure baseline. With chaos-mesh per-validator
            // loss the chain stays at ~98% participating voting power (no pod stops),
            // but chronic validators with 14-38% packet loss occasionally fail to
            // get their proposals through, producing brief round gaps.
            max_non_epoch_no_progress_secs: 30.0,
            max_epoch_no_progress_secs: 30.0,
            max_non_epoch_round_gap: 12,
            max_epoch_round_gap: 12,
        });

    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(validator_count).unwrap())
        .add_network_test(CompositeNetworkTest::new(
            MultiRegionChaosNoCleanup(MultiRegionNetworkEmulationTest::new_with_config(
                region_config,
            )),
            failure_test,
        ))
        .with_emit_job(
            EmitJobRequest::default()
                .mode(EmitJobMode::ConstTps { tps: emit_tps })
                .init_gas_price_multiplier(20),
        )
        .with_genesis_helm_config_fn(Arc::new(move |helm_values| {
            // Long epoch — same as mainnet_mirror_max_load_test, keeps validator
            // set stable so we measure failure-pattern dynamics not reconfig.
            helm_values["chain"]["epoch_duration_secs"] = 7200.into();
            helm_values["chain"]["on_chain_consensus_config"] =
                serde_yaml::to_value(OnChainConsensusConfig::default_for_genesis())
                    .expect("must serialize");
            helm_values["chain"]["on_chain_execution_config"] =
                serde_yaml::to_value(OnChainExecutionConfig::default_for_genesis())
                    .expect("must serialize");
            helm_values["genesis"]["validator"]["stake_amounts"] = stake_amounts_str.clone().into();
        }))
        .with_success_criteria(success_criteria)
}
