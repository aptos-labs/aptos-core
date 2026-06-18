// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::ungrouped::mixed_compatible_emit_job;
use crate::{suites::realistic_environment::realistic_env_max_load_test, TestCommand};
use aptos_forge::{
    prometheus_metrics::LatencyBreakdownSlice,
    success_criteria::{
        LatencyBreakdownThreshold, LatencyType, MetricsThreshold, StateProgressThreshold,
        SuccessCriteria, SystemMetricsThreshold,
    },
    EmitJobMode, EmitJobRequest, ForgeConfig, NodeResourceOverride,
};
use aptos_sdk::types::on_chain_config::{
    BlockGasLimitType, FeatureFlag, Features, OnChainConsensusConfig, OnChainExecutionConfig,
};
use aptos_testcases::{
    compatibility_test::SimpleValidatorUpgrade, framework_upgrade::FrameworkUpgrade,
    multi_region_network_test::MultiRegionNetworkEmulationTest, transaction_tracing_test,
    transaction_tracing_test::TransactionTracingTest, two_traffics_test::TwoTrafficsTest,
    CompositeNetworkTest,
};
use std::{num::NonZeroUsize, sync::Arc, time::Duration};

/// Attempts to match the test name to a land-blocking test
pub(crate) fn get_land_blocking_test(
    test_name: &str,
    duration: Duration,
    test_cmd: &TestCommand,
) -> Option<ForgeConfig> {
    let test = match test_name {
        "land_blocking" | "realistic_env_max_load" => {
            realistic_env_max_load_test(duration, test_cmd, 7, 0, 3)
        },
        "compat" => compat(),
        "framework_upgrade" => framework_upgrade(),
        "transaction_tracing_test" => transaction_tracing_test(duration, test_cmd),
        "transaction_tracing_retries_test" => transaction_tracing_retries_test(duration, test_cmd),
        _ => return None, // The test name does not match a land-blocking test
    };
    Some(test)
}

fn compatibility_test_features() -> Features {
    let mut features = Features::default();
    // Keep mixed-version forge tests aligned with mainnet/testnet, where this
    // feature is not enabled yet. When enabled, old and new binaries can
    // disagree on the hotness portion of transaction outputs.
    features.disable(FeatureFlag::HOTNESS_IN_EPILOGUE);
    features
}

fn set_compatibility_test_features(helm_values: &mut serde_yaml::Value) {
    helm_values["chain"]["initial_features_override"] =
        serde_yaml::to_value(compatibility_test_features()).expect("must serialize");
}

pub(crate) fn compat() -> ForgeConfig {
    ForgeConfig::default()
        .with_suite_name("compat".into())
        .with_initial_validator_count(NonZeroUsize::new(4).unwrap())
        .add_network_test(SimpleValidatorUpgrade)
        .with_success_criteria(SuccessCriteria::new(5000).add_wait_for_catchup_s(240))
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            helm_values["chain"]["epoch_duration_secs"] =
                SimpleValidatorUpgrade::EPOCH_DURATION_SECS.into();
            set_compatibility_test_features(helm_values);
        }))
}

pub(crate) fn framework_upgrade() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(4).unwrap())
        .add_network_test(FrameworkUpgrade)
        .with_success_criteria(SuccessCriteria::new(5000).add_wait_for_catchup_s(240))
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            helm_values["chain"]["epoch_duration_secs"] =
                FrameworkUpgrade::EPOCH_DURATION_SECS.into();
            set_compatibility_test_features(helm_values);
        }))
        // Genesis enables chunky DKG (decryption) by default. This test upgrades
        // only half the validators to the new binary, which would emit
        // BlockMetadataExt::V3 while the still-old half emits V2 — divergent
        // metadata that stalls consensus. Force chunky DKG off on every node via
        // the local seqnum override (local > on-chain genesis seqnum of 0), so
        // the chain stays on V1/V2 across the mixed-binary upgrade window. The
        // V2->V3 switch is exercised separately once all binaries are upgraded.
        .with_validator_override_node_config_fn(Arc::new(|config, _| {
            config.chunky_dkg_override_seq_num = 1;
        }))
        .with_emit_job(mixed_compatible_emit_job())
}

/// Transaction tracing test: exactly the same as land-blocking
/// (realistic_env_max_load_test with 7 validators, 0 VFNs, 3 PFNs)
/// plus 500 TPS traced traffic from pre-generated accounts.
///
/// The only additions on top of land-blocking:
///   - TransactionTracingTest wrapper around TwoTrafficsTest (adds 500 TPS traced traffic)
///   - Tracing config enabled on all validators with sender_allowlist
///
/// Tracing layout for overhead comparison:
///   - Validator 0: receives 500 TPS traced traffic → produces TxnTrace entries
///   - Validators 1-6: no traced traffic → only is_enabled()/should_trace() overhead
pub(crate) fn transaction_tracing_test(duration: Duration, test_cmd: &TestCommand) -> ForgeConfig {
    let ha_proxy = if let TestCommand::K8sSwarm(k8s) = test_cmd {
        k8s.enable_haproxy
    } else {
        false
    };

    let num_validators = 7;
    let num_vfns = 0;
    let num_pfns = 3;

    let duration_secs = duration.as_secs();
    let long_running = duration_secs >= 2400;

    let resource_override = if long_running {
        NodeResourceOverride {
            storage_gib: Some(1000),
            ..NodeResourceOverride::default()
        }
    } else {
        NodeResourceOverride::default()
    };

    let mut success_criteria = SuccessCriteria::new(85)
        .add_system_metrics_threshold(SystemMetricsThreshold::new(
            MetricsThreshold::new(25.0, 15),
            MetricsThreshold::new_gb(16.0 + 8.0 * (duration_secs as f64 / 3600.0), 20),
        ))
        .add_no_restarts()
        .add_wait_for_catchup_s((duration.as_secs() / 10).max(60))
        .add_latency_threshold(3.6, LatencyType::P50)
        .add_latency_threshold(4.8, LatencyType::P70)
        .add_chain_progress(StateProgressThreshold {
            max_non_epoch_no_progress_secs: 15.0,
            max_epoch_no_progress_secs: 16.0,
            max_non_epoch_round_gap: 4,
            max_epoch_round_gap: 4,
        });

    if !ha_proxy {
        success_criteria = success_criteria.add_latency_breakdown_threshold(
            LatencyBreakdownThreshold::new_with_breach_pct(
                vec![
                    (LatencyBreakdownSlice::MempoolToBlockCreation, 0.35 + 3.25),
                    (LatencyBreakdownSlice::ConsensusProposalToOrdered, 0.85),
                    (LatencyBreakdownSlice::ConsensusOrderedToCommit, 1.0),
                ],
                5,
            ),
        )
    }

    let mempool_backlog = if ha_proxy { 14000 } else { 19000 };

    // Same TwoTrafficsTest as land-blocking, wrapped with TransactionTracingTest
    // which adds 500 TPS traced traffic from pre-generated accounts.
    let inner_load_test = TwoTrafficsTest {
        inner_traffic: EmitJobRequest::default()
            .mode(EmitJobMode::MaxLoad { mempool_backlog })
            .init_gas_price_multiplier(20),
        inner_success_criteria: SuccessCriteria::new(
            if ha_proxy {
                7000
            } else if long_running {
                11000
            } else {
                10000
            },
        ),
    };
    let tracing_test = TransactionTracingTest {
        inner: Box::new(inner_load_test),
    };

    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(num_validators).unwrap())
        .with_initial_fullnode_count(num_vfns)
        .add_network_test(CompositeNetworkTest::new(
            MultiRegionNetworkEmulationTest::mainnet_calibrated_for_validator_count(num_validators),
            tracing_test,
        ))
        .with_genesis_helm_config_fn(Arc::new(move |helm_values| {
            helm_values["chain"]["epoch_duration_secs"] =
                (if long_running { 600 } else { 300 }).into();
            helm_values["chain"]["on_chain_consensus_config"] =
                serde_yaml::to_value(OnChainConsensusConfig::default_for_genesis())
                    .expect("must serialize");
            helm_values["chain"]["on_chain_execution_config"] =
                serde_yaml::to_value(OnChainExecutionConfig::default_for_genesis())
                    .expect("must serialize");
        }))
        .with_validator_override_node_config_fn(Arc::new(|config, _| {
            // Same as land-blocking
            config.base.enable_validator_pfn_connections = true;
            // Enable tracing with pre-generated accounts
            config.transaction_tracing.enabled = true;
            config.transaction_tracing.batch_sample_rate = 1.0;
            config.transaction_tracing.txn_sample_rate = 1.0;
            config.transaction_tracing.filter.sender_allowlist =
                transaction_tracing_test::traced_account_addresses();
        }))
        .with_pfn_override_node_config_fn(Arc::new(|config, _| {
            config.base.enable_validator_pfn_connections = true;
            config.consensus_observer.observer_enabled = true;
            config
                .consensus_observer
                .observer_fallback_progress_threshold_ms = 30_000;
            config
                .consensus_observer
                .observer_fallback_sync_lag_threshold_ms = 45_000;
        }))
        .with_emit_job(
            EmitJobRequest::default()
                .mode(EmitJobMode::ConstTps { tps: 100 })
                .gas_price(5 * aptos_global_constants::GAS_UNIT_PRICE)
                .latency_polling_interval(Duration::from_millis(100)),
        )
        .with_success_criteria(success_criteria)
        .with_validator_resource_override(resource_override)
        .with_fullnode_resource_override(resource_override)
        .with_num_pfns(num_pfns)
}

/// Variant of `transaction_tracing_test` that lowers the on-chain
/// `effective_block_gas_limit` from the genesis default (200_000) to 100_000.
/// Everything else (validator count, multi-region emulation, inner MaxLoad
/// traffic, 500 TPS traced traffic, sampling, allowlist) matches the main
/// tracing test so the comparison is apples-to-apples. The halved limit makes
/// SkipRest fire more often → many txns end up as `Retry`, producing
/// multi-attempt TxnTrace entries for verifying the structured retry-path
/// log shape under realistic load.
pub(crate) fn transaction_tracing_retries_test(
    duration: Duration,
    test_cmd: &TestCommand,
) -> ForgeConfig {
    let ha_proxy = if let TestCommand::K8sSwarm(k8s) = test_cmd {
        k8s.enable_haproxy
    } else {
        false
    };

    let num_validators = 7;
    let num_vfns = 0;
    let num_pfns = 3;

    let duration_secs = duration.as_secs();
    let long_running = duration_secs >= 2400;

    let resource_override = if long_running {
        NodeResourceOverride {
            storage_gib: Some(1000),
            ..NodeResourceOverride::default()
        }
    } else {
        NodeResourceOverride::default()
    };

    // Halved committable rate vs. main tracing test (block gas limit cut in
    // half) → halve the success criteria TPS bars accordingly.
    let success_criteria = SuccessCriteria::new(40)
        .add_system_metrics_threshold(SystemMetricsThreshold::new(
            MetricsThreshold::new(25.0, 15),
            MetricsThreshold::new_gb(16.0 + 8.0 * (duration_secs as f64 / 3600.0), 20),
        ))
        .add_no_restarts()
        .add_wait_for_catchup_s((duration.as_secs() / 10).max(60))
        .add_chain_progress(StateProgressThreshold {
            max_non_epoch_no_progress_secs: 30.0,
            max_epoch_no_progress_secs: 30.0,
            max_non_epoch_round_gap: 8,
            max_epoch_round_gap: 8,
        });

    let mempool_backlog = if ha_proxy { 14000 } else { 19000 };

    let inner_load_test = TwoTrafficsTest {
        inner_traffic: EmitJobRequest::default()
            .mode(EmitJobMode::MaxLoad { mempool_backlog })
            .init_gas_price_multiplier(20),
        inner_success_criteria: SuccessCriteria::new(
            if ha_proxy {
                3500
            } else if long_running {
                5500
            } else {
                5000
            },
        ),
    };
    let tracing_test = TransactionTracingTest {
        inner: Box::new(inner_load_test),
    };

    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(num_validators).unwrap())
        .with_initial_fullnode_count(num_vfns)
        .add_network_test(CompositeNetworkTest::new(
            MultiRegionNetworkEmulationTest::mainnet_calibrated_for_validator_count(num_validators),
            tracing_test,
        ))
        .with_genesis_helm_config_fn(Arc::new(move |helm_values| {
            helm_values["chain"]["epoch_duration_secs"] =
                (if long_running { 600 } else { 300 }).into();
            helm_values["chain"]["on_chain_consensus_config"] =
                serde_yaml::to_value(OnChainConsensusConfig::default_for_genesis())
                    .expect("must serialize");

            // The only divergence from `transaction_tracing_test`: halve the
            // effective block gas limit so SkipRest fires more often and a
            // meaningful fraction of txns hit the Retry path.
            let mut exec_config = OnChainExecutionConfig::default_for_genesis();
            if let OnChainExecutionConfig::V7(ref mut c) = exec_config {
                if let BlockGasLimitType::ComplexLimitV1 {
                    ref mut effective_block_gas_limit,
                    ..
                } = c.block_gas_limit_type
                {
                    *effective_block_gas_limit = 100_000;
                }
            }
            helm_values["chain"]["on_chain_execution_config"] =
                serde_yaml::to_value(exec_config).expect("must serialize");
        }))
        .with_validator_override_node_config_fn(Arc::new(|config, _| {
            config.base.enable_validator_pfn_connections = true;
            config.transaction_tracing.enabled = true;
            config.transaction_tracing.batch_sample_rate = 1.0;
            config.transaction_tracing.txn_sample_rate = 1.0;
            config.transaction_tracing.filter.sender_allowlist =
                transaction_tracing_test::traced_account_addresses();
        }))
        .with_pfn_override_node_config_fn(Arc::new(|config, _| {
            config.base.enable_validator_pfn_connections = true;
            config.consensus_observer.observer_enabled = true;
            config
                .consensus_observer
                .observer_fallback_progress_threshold_ms = 30_000;
            config
                .consensus_observer
                .observer_fallback_sync_lag_threshold_ms = 45_000;
        }))
        .with_emit_job(
            EmitJobRequest::default()
                .mode(EmitJobMode::ConstTps { tps: 100 })
                .gas_price(5 * aptos_global_constants::GAS_UNIT_PRICE)
                .latency_polling_interval(Duration::from_millis(100)),
        )
        .with_success_criteria(success_criteria)
        .with_validator_resource_override(resource_override)
        .with_fullnode_resource_override(resource_override)
        .with_num_pfns(num_pfns)
}
