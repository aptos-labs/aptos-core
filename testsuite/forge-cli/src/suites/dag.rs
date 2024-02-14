// Copyright © Aptos Foundation

use crate::{wrap_with_realistic_env, TestCommand};
use aptos_forge::{
    success_criteria::{LatencyType, StateProgressThreshold, SuccessCriteria},
    EmitJobMode, EmitJobRequest, ForgeConfig,
};
use aptos_sdk::types::on_chain_config::{
    ConsensusAlgorithmConfig, DagConsensusConfigV1, OnChainConsensusConfig, OnChainExecutionConfig,
    ValidatorTxnConfig,
};
use aptos_testcases::two_traffics_test::TwoTrafficsTest;
use std::{num::NonZeroUsize, sync::Arc, time::Duration};

pub fn get_dag_test(
    test_name: &str,
    duration: Duration,
    test_cmd: &TestCommand,
) -> Option<ForgeConfig> {
    get_dag_on_realistic_env_test(test_name, duration, test_cmd)
}

/// Attempts to match the test name to a dag-realistic-env test
fn get_dag_on_realistic_env_test(
    test_name: &str,
    duration: Duration,
    test_cmd: &TestCommand,
) -> Option<ForgeConfig> {
    let test = match test_name {
        "dag_realistic_env_max_load" => dag_realistic_env_max_load_test(duration, test_cmd, 7, 7),
        _ => return None, // The test name does not match a dag realistic-env test
    };
    Some(test)
}

fn dag_realistic_env_max_load_test(
    duration: Duration,
    test_cmd: &TestCommand,
    num_validators: usize,
    num_fullnodes: usize,
) -> ForgeConfig {
    // Check if HAProxy is enabled
    let ha_proxy = if let TestCommand::K8sSwarm(k8s) = test_cmd {
        k8s.enable_haproxy
    } else {
        false
    };

    // Determine if this is a long running test
    let duration_secs = duration.as_secs();
    let long_running = duration_secs >= 2400;

    // Create the test
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(num_validators).unwrap())
        .with_initial_fullnode_count(num_fullnodes)
        .add_network_test(wrap_with_realistic_env(TwoTrafficsTest {
            inner_traffic: EmitJobRequest::default()
                .mode(EmitJobMode::MaxLoad {
                    mempool_backlog: 30000,
                })
                .init_gas_price_multiplier(20),
            inner_success_criteria: SuccessCriteria::new(
                if ha_proxy {
                    2000
                } else if long_running {
                    // This is for forge stable
                    2500
                } else {
                    // During land time we want to be less strict, otherwise we flaky fail
                    2800
                },
            ),
        }))
        .with_genesis_helm_config_fn(Arc::new(move |helm_values| {
            let onchain_consensus_config = OnChainConsensusConfig::V3 {
                alg: ConsensusAlgorithmConfig::DAG(DagConsensusConfigV1::default()),
                vtxn: ValidatorTxnConfig::default_for_genesis(),
            };

            // Have single epoch change in land blocking, and a few on long-running
            helm_values["chain"]["epoch_duration_secs"] =
                (if long_running { 600 } else { 300 }).into();
            helm_values["chain"]["on_chain_consensus_config"] =
                serde_yaml::to_value(onchain_consensus_config).expect("must serialize");
            helm_values["chain"]["on_chain_execution_config"] =
                serde_yaml::to_value(OnChainExecutionConfig::default_for_genesis())
                    .expect("must serialize");
        }))
        // First start higher gas-fee traffic, to not cause issues with TxnEmitter setup - account creation
        .with_emit_job(
            EmitJobRequest::default()
                .mode(EmitJobMode::ConstTps { tps: 100 })
                .gas_price(5 * aptos_global_constants::GAS_UNIT_PRICE)
                .latency_polling_interval(Duration::from_millis(100)),
        )
        .with_success_criteria(
            SuccessCriteria::new(95)
                .add_no_restarts()
                .add_wait_for_catchup_s(
                    // Give at least 60s for catchup, give 10% of the run for longer durations.
                    (duration.as_secs() / 10).max(60),
                )
                .add_latency_threshold(4.0, LatencyType::P50)
                .add_chain_progress(StateProgressThreshold {
                    max_no_progress_secs: 15.0,
                    max_round_gap: 4,
                }),
        )
}
