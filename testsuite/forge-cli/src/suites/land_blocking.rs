// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::ungrouped::mixed_compatible_emit_job;
use crate::{suites::realistic_environment::realistic_env_max_load_test, TestCommand};
use aptos_forge::{
    success_criteria::{LatencyBreakdownThreshold, LatencyType, StateProgressThreshold, SuccessCriteria, SystemMetricsThreshold},
    EmitJobMode, EmitJobRequest, ForgeConfig,
};
use aptos_sdk::types::on_chain_config::{OnChainConsensusConfig, OnChainExecutionConfig};
use aptos_testcases::{
    compatibility_test::SimpleValidatorUpgrade,
    framework_upgrade::FrameworkUpgrade,
    multi_region_network_test::MultiRegionNetworkEmulationTest,
    two_traffics_test::TwoTrafficsTest,
    CompositeNetworkTest,
    transaction_tracing_test, transaction_tracing_test::TransactionTracingTest,
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
            realistic_env_max_load_test(duration, test_cmd, 7, 2)
        },
        "compat" => compat(),
        "framework_upgrade" => framework_upgrade(),
        "transaction_tracing_test" => transaction_tracing_test(duration, test_cmd),
        _ => return None, // The test name does not match a land-blocking test
    };
    Some(test)
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
        }))
        .with_emit_job(mixed_compatible_emit_job())
}

/// Transaction tracing test: same topology as land-blocking (7 validators, 2 fullnodes,
/// geo-distributed, MaxLoad traffic) plus 500 TPS traced traffic from pre-generated accounts.
///
/// Tracing layout for overhead comparison:
///   - Validator 0: tracing ON + receives 500 TPS traced traffic → produces TxnTrace entries
///   - Validators 1-6: tracing ON + no traced traffic → only is_enabled()/should_trace() overhead
/// Compare validator 0's mempool/QS latency against validators 1-6 to measure overhead.
pub(crate) fn transaction_tracing_test(duration: Duration, test_cmd: &TestCommand) -> ForgeConfig {
    let ha_proxy = if let TestCommand::K8sSwarm(k8s) = test_cmd {
        k8s.enable_haproxy
    } else {
        false
    };

    let num_validators = 7;
    let num_fullnodes = 2;

    let mempool_backlog = if ha_proxy { 28000 } else { 38000 };

    let success_criteria = SuccessCriteria::new(if ha_proxy { 7000 } else { 10000 })
        .add_system_metrics_threshold(SystemMetricsThreshold::new(
            aptos_forge::success_criteria::MetricsThreshold::new(25.0, 15),
            aptos_forge::success_criteria::MetricsThreshold::new_gb(
                16.0 + 8.0 * (duration.as_secs() as f64 / 3600.0),
                20,
            ),
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

    // Compose: geo-distribution → tracing wrapper → TwoTrafficsTest (MaxLoad)
    let inner_load_test = TwoTrafficsTest {
        inner_traffic: EmitJobRequest::default()
            .mode(EmitJobMode::MaxLoad { mempool_backlog })
            .init_gas_price_multiplier(20),
        inner_success_criteria: SuccessCriteria::new(if ha_proxy { 7000 } else { 10000 }),
    };
    let tracing_test = TransactionTracingTest {
        inner: Box::new(inner_load_test),
    };

    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(num_validators).unwrap())
        .with_initial_fullnode_count(num_fullnodes)
        .add_network_test(CompositeNetworkTest::new(
            MultiRegionNetworkEmulationTest::default_for_validator_count(num_validators),
            tracing_test,
        ))
        // Enable tracing on all validators with the traced sender allowlist.
        // Only validator 0 receives traced traffic (500 TPS) → it produces
        // TxnTrace entries. Validators 1-6 run is_enabled() + should_trace()
        // on the same allowlist but don't match (emitter uses different accounts)
        // → measures the overhead of the filter check on untraced traffic.
        .with_validator_override_node_config_fn(Arc::new(|config, _| {
            config.transaction_tracing.enabled = true;
            config.transaction_tracing.batch_sample_rate = 1.0;
            config.transaction_tracing.txn_sample_rate = 1.0;
            config.transaction_tracing.sender_allowlist =
                transaction_tracing_test::traced_account_addresses()
                    .into_iter()
                    .collect();
        }))
        .with_genesis_helm_config_fn(Arc::new(move |helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = 300.into();
            helm_values["chain"]["on_chain_consensus_config"] =
                serde_yaml::to_value(OnChainConsensusConfig::default_for_genesis())
                    .expect("must serialize");
            helm_values["chain"]["on_chain_execution_config"] =
                serde_yaml::to_value(OnChainExecutionConfig::default_for_genesis())
                    .expect("must serialize");
        }))
        .with_emit_job(
            EmitJobRequest::default()
                .mode(EmitJobMode::ConstTps { tps: 100 })
                .gas_price(5 * aptos_global_constants::GAS_UNIT_PRICE)
                .latency_polling_interval(Duration::from_millis(100)),
        )
        // The outer traffic is a low-TPS probe (100 TPS). The real throughput
        // criteria (10K+ TPS) are on the inner traffic via inner_success_criteria.
        // Use minimal TPS bar here; keep chain progress checks.
        .with_success_criteria(
            SuccessCriteria::new(50)
                .add_no_restarts()
                .add_chain_progress(StateProgressThreshold {
                    max_non_epoch_no_progress_secs: 15.0,
                    max_epoch_no_progress_secs: 16.0,
                    max_non_epoch_round_gap: 4,
                    max_epoch_round_gap: 4,
                }),
        )
        .with_num_pfns(1)
}
