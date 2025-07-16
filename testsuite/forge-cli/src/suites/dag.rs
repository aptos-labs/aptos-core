// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::{
    realistic_environment::wrap_with_realistic_env, ungrouped::changing_working_quorum_test_helper,
};
use crate::TestCommand;
use aptos_forge::{
    success_criteria::{LatencyType, StateProgressThreshold, SuccessCriteria},
    EmitJobMode, EmitJobRequest, ForgeConfig,
};
use aptos_sdk::types::on_chain_config::{
    BlockGasLimitType, ConsensusAlgorithmConfig, DagConsensusConfigV1, OnChainConsensusConfig,
    OnChainExecutionConfig, TransactionShufflerType, ValidatorTxnConfig, DEFAULT_WINDOW_SIZE,
};
use aptos_testcases::{
    consensus_reliability_tests::ChangingWorkingQuorumTest,
    dag_onchain_enable_test::DagOnChainEnableTest, two_traffics_test::TwoTrafficsTest,
};
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
        "dag_realistic_env_max_load" => dag_realistic_env_max_load_test(duration, test_cmd, 20, 0),
        "dag_changing_working_quorum_test" => dag_changing_working_quorum_test(),
        "dag_reconfig_enable_test" => dag_reconfig_enable_test(),
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
        .add_network_test(wrap_with_realistic_env(num_validators, TwoTrafficsTest {
            inner_traffic: EmitJobRequest::default()
                .mode(EmitJobMode::MaxLoad {
                    mempool_backlog: 50000,
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
        .with_validator_override_node_config_fn(Arc::new(|config, _| {
            config.consensus.max_sending_block_txns = 4000;
            config.consensus.max_sending_block_bytes = 6 * 1024 * 1024;
            config.consensus.max_receiving_block_txns = 10000;
            config.consensus.max_receiving_block_bytes = 7 * 1024 * 1024;
        }))
        .with_genesis_helm_config_fn(Arc::new(move |helm_values| {
            // Have single epoch change in land blocking, and a few on long-running
            helm_values["chain"]["epoch_duration_secs"] =
                (if long_running { 600 } else { 300 }).into();

            let onchain_consensus_config = OnChainConsensusConfig::V4 {
                alg: ConsensusAlgorithmConfig::DAG(DagConsensusConfigV1::default()),
                vtxn: ValidatorTxnConfig::default_for_genesis(),
                window_size: DEFAULT_WINDOW_SIZE
            };

            helm_values["chain"]["on_chain_consensus_config"] =
                serde_yaml::to_value(onchain_consensus_config).expect("must serialize");

            let mut on_chain_execution_config = OnChainExecutionConfig::default_for_genesis();
            // Need to update if the default changes
            match &mut on_chain_execution_config {
                OnChainExecutionConfig::Missing
                | OnChainExecutionConfig::V1(_)
                | OnChainExecutionConfig::V2(_)
                | OnChainExecutionConfig::V3(_) => {
                    unreachable!("Unexpected on-chain execution config type, if OnChainExecutionConfig::default_for_genesis() has been updated, this test must be updated too.")
                }
                OnChainExecutionConfig::V4(config_v4) => {
                    config_v4.block_gas_limit_type = BlockGasLimitType::NoLimit;
                    config_v4.transaction_shuffler_type = TransactionShufflerType::default_for_genesis();
                }
                OnChainExecutionConfig::V5(config_v5) => {
                    config_v5.block_gas_limit_type = BlockGasLimitType::NoLimit;
                    config_v5.transaction_shuffler_type = TransactionShufflerType::default_for_genesis();
                }
                OnChainExecutionConfig::V6(config_v6) => {
                    config_v6.block_gas_limit_type = BlockGasLimitType::NoLimit;
                    config_v6.transaction_shuffler_type = TransactionShufflerType::default_for_genesis();
                }
            }
            helm_values["chain"]["on_chain_execution_config"] =
                serde_yaml::to_value(on_chain_execution_config).expect("must serialize");
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
                    max_non_epoch_no_progress_secs: 15.0,
                    max_epoch_no_progress_secs: 15.0,
                    max_non_epoch_round_gap: 8,
                    max_epoch_round_gap: 8,
                }),
        )
}

fn dag_changing_working_quorum_test() -> ForgeConfig {
    let epoch_duration = 120;
    let num_large_validators = 0;
    let base_config = changing_working_quorum_test_helper(
        16,
        epoch_duration,
        100,
        70,
        true,
        true,
        false,
        ChangingWorkingQuorumTest {
            min_tps: 15,
            always_healthy_nodes: 0,
            max_down_nodes: 16,
            num_large_validators,
            add_execution_delay: false,
            // Use longer check duration, as we are bringing enough nodes
            // to require state-sync to catch up to have consensus.
            check_period_s: 53,
        },
    );

    base_config
        .with_validator_override_node_config_fn(Arc::new(|config, _| {
            config.consensus.max_sending_block_txns = 4000;
            config.consensus.max_sending_block_bytes = 6 * 1024 * 1024;
            config.consensus.max_receiving_block_txns = 10000;
            config.consensus.max_receiving_block_bytes = 7 * 1024 * 1024;
        }))
        .with_genesis_helm_config_fn(Arc::new(move |helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = epoch_duration.into();
            helm_values["genesis"]["validator"]["num_validators_with_larger_stake"] =
                num_large_validators.into();

            let onchain_consensus_config = OnChainConsensusConfig::V4 {
                alg: ConsensusAlgorithmConfig::DAG(DagConsensusConfigV1::default()),
                vtxn: ValidatorTxnConfig::default_for_genesis(),
                window_size: DEFAULT_WINDOW_SIZE,
            };

            helm_values["chain"]["on_chain_consensus_config"] =
                serde_yaml::to_value(onchain_consensus_config).expect("must serialize");
            helm_values["chain"]["on_chain_execution_config"] =
                serde_yaml::to_value(OnChainExecutionConfig::default_for_genesis())
                    .expect("must serialize");
        }))
}

fn dag_reconfig_enable_test() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(20).unwrap())
        .with_initial_fullnode_count(20)
        .add_network_test(DagOnChainEnableTest {})
        .with_validator_override_node_config_fn(Arc::new(|config, _| {
            config.consensus.max_sending_block_txns = 4000;
            config.consensus.max_sending_block_bytes = 6 * 1024 * 1024;
            config.consensus.max_receiving_block_txns = 10000;
            config.consensus.max_receiving_block_bytes = 7 * 1024 * 1024;
        }))
        .with_genesis_helm_config_fn(Arc::new(move |helm_values| {
            let mut on_chain_execution_config = OnChainExecutionConfig::default_for_genesis();
            // Need to update if the default changes
            match &mut on_chain_execution_config {
                    OnChainExecutionConfig::Missing
                    | OnChainExecutionConfig::V1(_)
                    | OnChainExecutionConfig::V2(_)
                    | OnChainExecutionConfig::V3(_) => {
                        unreachable!("Unexpected on-chain execution config type, if OnChainExecutionConfig::default_for_genesis() has been updated, this test must be updated too.")
                    }
                    OnChainExecutionConfig::V4(config_v4) => {
                        config_v4.block_gas_limit_type = BlockGasLimitType::NoLimit;
                    }
                    OnChainExecutionConfig::V5(config_v5) => {
                        config_v5.block_gas_limit_type = BlockGasLimitType::NoLimit;
                    }
                    OnChainExecutionConfig::V6(config_v6) => {
                        config_v6.block_gas_limit_type = BlockGasLimitType::NoLimit;
                    }
            }
            helm_values["chain"]["on_chain_execution_config"] =
                serde_yaml::to_value(on_chain_execution_config).expect("must serialize");
        }))
        .with_success_criteria(
            SuccessCriteria::new(1000)
                .add_no_restarts()
                .add_wait_for_catchup_s(240)
                .add_chain_progress(StateProgressThreshold {
                    max_non_epoch_no_progress_secs: 20.0,
                    max_epoch_no_progress_secs: 20.0,
                    max_non_epoch_round_gap: 20,
                    max_epoch_round_gap: 20,
                }),
        )
}
