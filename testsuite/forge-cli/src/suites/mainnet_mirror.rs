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
    args::TransactionTypeArg,
    success_criteria::{StateProgressThreshold, SuccessCriteria},
    EmitJobMode, EmitJobRequest, ForgeConfig, TransactionType,
};
use aptos_sdk::types::on_chain_config::{
    ConsensusAlgorithmConfig, LeaderReputationType, OnChainConsensusConfig, OnChainExecutionConfig,
    ProposerElectionType,
};
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

/// Mainnet-realistic transaction mix. Mainnet's consistent ~0.7 execution
/// backpressure comes from the mix of light (APT transfer) and heavy
/// (NFT mint, contention writes, large resource ops) transactions. Plain
/// p2p transfers don't push validators hard enough on execution time, so
/// chain stays well under target_block_time_ms and never triggers
/// `aptos_execution_backpressure_on_proposal_triggered`.
///
/// Weights chosen to roughly match observed mainnet activity profile:
///   60% CoinTransfer        (cheap APT transfers — most of mainnet volume)
///   20% TokenV2AmbassadorMint (NFT mints — moderate execution cost)
///   10% ModifyGlobalResource (heavy contention on a shared resource)
///   10% ResourceGroupsGlobalWriteAndReadTag1KB (large resource ops)
fn mainnet_realistic_mix() -> Vec<(TransactionType, usize)> {
    vec![
        (TransactionTypeArg::CoinTransfer.materialize_default(), 60),
        (
            TransactionTypeArg::TokenV2AmbassadorMint.materialize_default(),
            20,
        ),
        (
            TransactionTypeArg::ModifyGlobalResource.materialize_default(),
            10,
        ),
        (
            TransactionTypeArg::ResourceGroupsGlobalWriteAndReadTag1KB.materialize_default(),
            10,
        ),
    ]
}

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

/// Stratified 23-validator subset used by the `_small` test variants. Picks
/// deterministically from the embedded snapshot so both small tests run
/// against the exact same validator set (one with failure injection, one
/// without).
///
/// 7 failure validators (all 4 mainnet StableChronics + 1 OnlineButFlaky in
/// each of apne1/eu-west-1 + 1 EpisodicSpike). Stake-weighted faulty share
/// = 16.83% vs mainnet's 16.03%. apne1-0 (the top batch author, 23% of
/// mainnet batches) is now included as the apne1 flaky pick.
fn small_stratified_subset() -> MainnetMirrorSnapshot {
    use aptos_testcases::mainnet_mirror::{AvailabilityClass::*, Region::*};
    MainnetMirrorSnapshot::load_embedded()
        .expect("embedded mainnet validator snapshot failed to parse")
        .stratified_subset(&[
            // 7 failure validators. Picks resolve to:
            //   (StableChronic, Apne1)         → hashport (0x312c22e7), rate 14.3%
            //   (StableChronic, EuCentral1)    → rockrpc.net,           rate 11.2%
            //   (StableChronic, UsCentral1)×2  → sirouk (0x50b27eee, 13.7%) +
            //                                    herd.run (0x973891f5,  17.5%)
            //   (OnlineButFlaky, Apne1)        → val0.apne1-0 (Aptos Labs, 3.6%,
            //                                    top batch author 23% of mainnet)
            //   (OnlineButFlaky, EuWest1)      → Stakely, rate 1.7%
            //   (EpisodicSpike,  EuWest1)      → val0.euwe6-1 (Aptos Labs), burst
            //
            // Chronic configured rates are clamped at 10% by
            // FailurePatternConfig::max_continuous_pct so they sit at the
            // leader-reputation failure_threshold_percent boundary and stay in
            // the leader rotation rather than getting demoted to failed_weight.
            (StableChronic, Apne1, 1),
            (StableChronic, EuCentral1, 1),
            (StableChronic, UsCentral1, 2),
            (OnlineButFlaky, Apne1, 1),
            (OnlineButFlaky, EuWest1, 1),
            (EpisodicSpike, EuWest1, 1),
            // 16 healthy validators distributed proportional to mainnet region
            // stake. EuCentral1 dropped 5→4 (rockrpc takes a chronic slot in
            // that region); other healthy slots unchanged.
            (Healthy, CaCentral1, 2),
            (Healthy, EuCentral1, 4),
            (Healthy, EuWest1, 7),
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
    // Heavy mix lowers achievable TPS but matches mainnet's exec profile and
    // reliably triggers ~0.7 execution backpressure. min_tps lowered from the
    // pre-mix 2000 to 700 — heavy txns sustain less throughput than plain
    // transfers at the same exec headroom.
    build_max_load_test(snapshot, duration, 2000, 700, true)
}

/// Small-scale variant of `mainnet_mirror_max_load_test` on the stratified
/// 21-validator subset. Same heavy mix and TPS targets as the full test —
/// chain TPS is per-block (same blocks committed regardless of validator
/// count), so the per-validator exec profile and backpressure dynamics
/// reproduce on the smaller subset. Pairs with
/// `mainnet_mirror_failures_small_test` for fast (~10 min) mainnet-shape
/// validation before committing to the full 30+ min run.
pub(crate) fn mainnet_mirror_max_load_small_test(duration: Duration) -> ForgeConfig {
    build_max_load_test(small_stratified_subset(), duration, 2000, 700, true)
}

/// Shared body of both max-load tests; differs only by snapshot subset, emit
/// TPS target, success-criteria TPS floor, and whether to use the
/// mainnet-realistic transaction mix (true = full, false = small).
fn build_max_load_test(
    snapshot: MainnetMirrorSnapshot,
    duration: Duration,
    emit_tps: usize,
    min_tps: usize,
    heavy_mix: bool,
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
        .with_emit_job({
            let mut req = EmitJobRequest::default()
                .mode(EmitJobMode::ConstTps { tps: emit_tps })
                .init_gas_price_multiplier(20);
            if heavy_mix {
                req = req.transaction_mix(mainnet_realistic_mix());
            }
            req
        })
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
    // Same heavy mix and TPS targets as the full test — small variant is a
    // mainnet-shape simulator at faster wall-clock, not just a plumbing test.
    // Chain TPS doesn't scale with validator count (every validator commits
    // the same blocks), so per-block exec profile and the resulting
    // backpressure dynamics reproduce on the 21-validator subset.
    build_failures_test(small_stratified_subset(), duration, 1400, 600, true)
}

/// Mainnet-mirror with failure-pattern injection. Applies per-validator
/// failpoints matching each validator's real mainnet `fp_7d_avg` (chronic /
/// flaky get continuous targeted-failpoint delay; spike validators get a
/// one-shot 100%-return failpoint at a randomized offset). Uses the
/// mainnet-realistic transaction mix so block execution time matches mainnet
/// and `aptos_execution_backpressure_on_proposal_triggered` activates around
/// 0.7 the way it does on real mainnet (plain p2p transfers don't push exec
/// hard enough to trigger backpressure). Loosens success criteria modestly
/// to account for the ~0.3-0.5% failed-round rate this produces.
pub(crate) fn mainnet_mirror_failures_test(duration: Duration) -> ForgeConfig {
    let snapshot = MainnetMirrorSnapshot::load_embedded()
        .expect("embedded mainnet validator snapshot failed to parse");
    // emit_tps 1400: 1500 drove exec backpressure to ~0.8 (mainnet ~0.76),
    // 1200 was a touch under-driven, 1400 is the sweet spot. min_tps stays
    // at 600 — well above the achievable ceiling.
    build_failures_test(snapshot, duration, 1400, 600, true)
}

/// Shared body of both failures tests; differs only by snapshot subset, emit
/// TPS target, success-criteria TPS floor, and whether to use the
/// mainnet-realistic transaction mix (true = full, false = small).
fn build_failures_test(
    snapshot: MainnetMirrorSnapshot,
    duration: Duration,
    emit_tps: usize,
    min_tps: usize,
    heavy_mix: bool,
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
        .with_validator_override_node_config_fn(Arc::new(|config, _| {
            // MainnetMirrorFailureTest injects per-validator failpoints via the
            // /v1/-/set_failpoint API. The image is built with --features failpoints,
            // but the endpoint also requires this config knob — without it every
            // set_failpoint call returns 500 "Failpoints are not enabled at a config level".
            config.api.failpoints_enabled = true;
        }))
        .with_emit_job({
            let mut req = EmitJobRequest::default()
                .mode(EmitJobMode::ConstTps { tps: emit_tps })
                .init_gas_price_multiplier(20);
            if heavy_mix {
                req = req.transaction_mix(mainnet_realistic_mix());
            }
            req
        })
        .with_genesis_helm_config_fn(Arc::new(move |helm_values| {
            // Long epoch — same as mainnet_mirror_max_load_test, keeps validator
            // set stable so we measure failure-pattern dynamics not reconfig.
            helm_values["chain"]["epoch_duration_secs"] = 7200.into();
            // V2 leader-reputation tuning: wider window + tighter failure
            // threshold catches chronic/flaky validators that default
            // (10× multiplier, 10% threshold) misses.
            helm_values["chain"]["on_chain_consensus_config"] =
                serde_yaml::to_value(v2_tuned_consensus_config()).expect("must serialize");
            helm_values["chain"]["on_chain_execution_config"] =
                serde_yaml::to_value(OnChainExecutionConfig::default_for_genesis())
                    .expect("must serialize");
            helm_values["genesis"]["validator"]["stake_amounts"] = stake_amounts_str.clone().into();
        }))
        .with_success_criteria(success_criteria)
}

/// Builds the genesis on-chain consensus config with V2 leader-reputation
/// parameters tuned for the mainnet-mirror failures suite.
///
/// V2 tuning vs `default_for_genesis`:
/// * `proposer_window_num_validators_multiplier`: 10 → **100**. With 107
///   mainnet validators that gives a ~10,700-block window (~9 min wall
///   clock at 20 blocks/sec) — enough samples to see sub-10% failure
///   rates with tight σ (~2% on a 4% true rate).
/// * `failure_threshold_percent`: 10 → **5**. Catches mainnet
///   OnlineButFlaky validators (real rates 0.5-3%) that default 10%
///   threshold misses, while leaving chronics (capped at 10% by the
///   failure-injection code) at the boundary so they still rotate.
fn v2_tuned_consensus_config() -> OnChainConsensusConfig {
    let mut cfg = OnChainConsensusConfig::default_for_genesis();
    let alg = match &mut cfg {
        OnChainConsensusConfig::V5 { alg, .. } => alg,
        other => panic!(
            "default consensus config is expected to be V5, got {:?}",
            other
        ),
    };
    let main = match alg {
        ConsensusAlgorithmConfig::JolteonV2 { main, .. } => main,
        _ => panic!("default consensus algorithm is expected to be JolteonV2"),
    };
    let v2_base = match &mut main.proposer_election_type {
        ProposerElectionType::LeaderReputation(LeaderReputationType::ProposerAndVoterV2(c)) => c,
        other => panic!("expected ProposerAndVoterV2 default, got {:?}", other),
    };
    v2_base.proposer_window_num_validators_multiplier = 100;
    v2_base.failure_threshold_percent = 5;
    cfg
}
