// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::ungrouped::{
    background_traffic_for_sweep, background_traffic_for_sweep_with_latency,
    RELIABLE_REAL_ENV_PROGRESS_THRESHOLD,
};
use crate::{
    suites::ungrouped::{optimize_for_maximum_throughput, optimize_state_sync_for_throughput},
    TestCommand,
};
use aptos_forge::{
    args::TransactionTypeArg,
    prometheus_metrics::LatencyBreakdownSlice,
    success_criteria::{
        LatencyBreakdownThreshold, LatencyType, MetricsThreshold, StateProgressThreshold,
        SuccessCriteria, SystemMetricsThreshold,
    },
    EmitJobMode, EmitJobRequest, ForgeConfig, NetworkTest, NodeResourceOverride,
};
use aptos_sdk::types::on_chain_config::{
    BlockGasLimitType, OnChainConsensusConfig, OnChainExecutionConfig, TransactionShufflerType,
};
use aptos_testcases::{
    load_vs_perf_benchmark::{LoadVsPerfBenchmark, TransactionWorkload, Workloads},
    multi_region_network_test::MultiRegionNetworkEmulationTest,
    performance_test::PerformanceBenchmark,
    two_traffics_test::TwoTrafficsTest,
    CompositeNetworkTest,
};
use std::{num::NonZeroUsize, sync::Arc, time::Duration};

/// Attempts to match the test name to a realistic-env test
pub(crate) fn get_realistic_env_test(
    test_name: &str,
    duration: Duration,
    test_cmd: &TestCommand,
) -> Option<ForgeConfig> {
    let test = match test_name {
        "realistic_env_max_load_large" => realistic_env_max_load_test(duration, test_cmd, 20, 10),
        "realistic_env_load_sweep" => realistic_env_load_sweep_test(),
        "realistic_env_workload_sweep" => realistic_env_workload_sweep_test(),
        "realistic_env_fairness_workload_sweep" => realistic_env_fairness_workload_sweep(),
        "realistic_env_graceful_workload_sweep" => realistic_env_graceful_workload_sweep(),
        "realistic_env_graceful_overload" => realistic_env_graceful_overload(duration),
        "realistic_network_tuned_for_throughput" => realistic_network_tuned_for_throughput_test(),
        _ => return None, // The test name does not match a realistic-env test
    };
    Some(test)
}

pub(crate) fn realistic_env_sweep_wrap(
    num_validators: usize,
    num_fullnodes: usize,
    test: LoadVsPerfBenchmark,
) -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(num_validators).unwrap())
        .with_initial_fullnode_count(num_fullnodes)
        .with_validator_override_node_config_fn(Arc::new(|config, _| {
            config.execution.processed_transactions_detailed_counters = true;
        }))
        .add_network_test(wrap_with_realistic_env(num_validators, test))
        // Test inherits the main EmitJobRequest, so update here for more precise latency measurements
        .with_emit_job(
            EmitJobRequest::default().latency_polling_interval(Duration::from_millis(100)),
        )
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            // no epoch change.
            helm_values["chain"]["epoch_duration_secs"] = (24 * 3600).into();
        }))
        .with_success_criteria(
            SuccessCriteria::new(0)
                .add_no_restarts()
                .add_wait_for_catchup_s(60)
                .add_chain_progress(RELIABLE_REAL_ENV_PROGRESS_THRESHOLD.clone()),
        )
}

pub(crate) fn realistic_env_load_sweep_test() -> ForgeConfig {
    realistic_env_sweep_wrap(16, 0, LoadVsPerfBenchmark {
        test: Box::new(PerformanceBenchmark),
        workloads: Workloads::TPS(vec![10, 100, 1000, 3000, 5000, 7000]),
        criteria: [
            (9, 0.9, 1.0, 1.2, 0),
            (95, 0.9, 1.1, 1.2, 0),
            (950, 1.2, 1.3, 2.0, 0),
            (2900, 1.4, 2.2, 2.5, 0),
            (4800, 2.0, 2.5, 3.0, 0),
            (6700, 2.5, 3.5, 5.0, 0),
            // TODO add 9k or 10k. Allow some expired transactions (high-load)
        ]
        .into_iter()
        .map(
            |(min_tps, max_lat_p50, max_lat_p90, max_lat_p99, max_expired_tps)| {
                SuccessCriteria::new(min_tps)
                    .add_max_expired_tps(max_expired_tps as f64)
                    .add_max_failed_submission_tps(0.0)
                    .add_latency_threshold(max_lat_p50, LatencyType::P50)
                    .add_latency_threshold(max_lat_p90, LatencyType::P90)
                    .add_latency_threshold(max_lat_p99, LatencyType::P99)
            },
        )
        .collect(),
        background_traffic: background_traffic_for_sweep(5),
    })
}

pub(crate) fn realistic_env_workload_sweep_test() -> ForgeConfig {
    realistic_env_sweep_wrap(7, 3, LoadVsPerfBenchmark {
        test: Box::new(PerformanceBenchmark),
        workloads: Workloads::TRANSACTIONS(vec![
            TransactionWorkload::new(TransactionTypeArg::CoinTransfer, 20000),
            TransactionWorkload::new(TransactionTypeArg::NoOp, 20000).with_num_modules(100),
            TransactionWorkload::new(TransactionTypeArg::ModifyGlobalResource, 6000)
                .with_transactions_per_account(1),
            TransactionWorkload::new(TransactionTypeArg::TokenV2AmbassadorMint, 20000)
                .with_unique_senders(),
            // TODO(ibalajiarun): this is disabled due to Forge Stable failure on PosToProposal latency.
            TransactionWorkload::new(TransactionTypeArg::PublishPackage, 200)
                .with_transactions_per_account(1),
        ]),
        // Investigate/improve to make latency more predictable on different workloads
        criteria: [
            (7000, 100, 0.3 + 0.5, 0.5, 0.5),
            (8500, 100, 0.3 + 0.5, 0.5, 0.4),
            (2000, 300, 0.3 + 1.0, 0.6, 1.0),
            (3200, 500, 0.3 + 1.0, 0.7, 0.8),
            // TODO - pos-to-proposal is set to high, until it is calibrated/understood.
            (28, 5, 0.3 + 5.0, 0.7, 1.0),
        ]
        .into_iter()
        .map(
            |(
                min_tps,
                max_expired,
                mempool_to_block_creation,
                proposal_to_ordered,
                ordered_to_commit,
            )| {
                SuccessCriteria::new(min_tps)
                    .add_max_expired_tps(max_expired as f64)
                    .add_max_failed_submission_tps(200.0)
                    .add_latency_breakdown_threshold(LatencyBreakdownThreshold::new_strict(vec![
                        (
                            LatencyBreakdownSlice::MempoolToBlockCreation,
                            mempool_to_block_creation,
                        ),
                        (
                            LatencyBreakdownSlice::ConsensusProposalToOrdered,
                            proposal_to_ordered,
                        ),
                        (
                            LatencyBreakdownSlice::ConsensusOrderedToCommit,
                            ordered_to_commit,
                        ),
                    ]))
            },
        )
        .collect(),
        background_traffic: background_traffic_for_sweep(5),
    })
}

pub(crate) fn realistic_env_fairness_workload_sweep() -> ForgeConfig {
    realistic_env_sweep_wrap(7, 3, LoadVsPerfBenchmark {
        test: Box::new(PerformanceBenchmark),
        workloads: Workloads::TRANSACTIONS(vec![
            // Very high gas
            TransactionWorkload::new(
                TransactionTypeArg::ResourceGroupsGlobalWriteAndReadTag1KB,
                100000,
            ),
            TransactionWorkload::new(TransactionTypeArg::VectorPicture30k, 20000),
            TransactionWorkload::new(TransactionTypeArg::SmartTablePicture1MWith256Change, 4000)
                .with_transactions_per_account(1),
        ]),
        criteria: Vec::new(),
        background_traffic: background_traffic_for_sweep_with_latency(
            &[(2.0, 3.0, 8.0), (0.1, 25.0, 30.0), (0.1, 30.0, 45.0)],
            false,
        ),
    })
}

pub(crate) fn realistic_env_graceful_workload_sweep() -> ForgeConfig {
    realistic_env_sweep_wrap(7, 3, LoadVsPerfBenchmark {
        test: Box::new(PerformanceBenchmark),
        workloads: Workloads::TRANSACTIONS(vec![
            // do account generation first, to fill up a storage a bit.
            TransactionWorkload::new_const_tps(TransactionTypeArg::AccountGeneration, 2 * 7000),
            // Very high gas
            TransactionWorkload::new_const_tps(
                TransactionTypeArg::ResourceGroupsGlobalWriteAndReadTag1KB,
                3 * 1800,
            ),
            TransactionWorkload::new_const_tps(
                TransactionTypeArg::SmartTablePicture1MWith256Change,
                3 * 14,
            ),
            TransactionWorkload::new_const_tps(
                TransactionTypeArg::SmartTablePicture1MWith1KChangeExceedsLimit,
                3 * 12,
            ),
            TransactionWorkload::new_const_tps(TransactionTypeArg::VectorPicture30k, 3 * 150),
            TransactionWorkload::new_const_tps(TransactionTypeArg::ModifyGlobalFlagAggV2, 3 * 3500),
            // publishing package - executes sequentially
            TransactionWorkload::new_const_tps(TransactionTypeArg::PublishPackage, 3 * 150)
                .with_transactions_per_account(1),
        ]),
        criteria: Vec::new(),
        background_traffic: background_traffic_for_sweep_with_latency(
            &[
                (0.1, 4.0, 5.0),
                (0.1, 2.2, 3.0),
                (0.1, 3.5, 5.0),
                (0.1, 4.0, 6.0),
                // TODO - p50 and p90 is set to high, until it is calibrated/understood.
                (0.1, 3.0, 5.0),
                // TODO - p50 and p90 is set to high, until it is calibrated/understood.
                (0.1, 5.0, 10.0),
                // TODO - p50 and p90 is set to high, until it is calibrated/understood.
                (0.1, 3.0, 10.0),
            ],
            true,
        ),
    })
    .with_emit_job(
        EmitJobRequest::default()
            .txn_expiration_time_secs(20)
            .init_gas_price_multiplier(5)
            .init_expiration_multiplier(6.0),
    )
}

pub(crate) fn realistic_env_graceful_overload(duration: Duration) -> ForgeConfig {
    let num_validators = 20;
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(num_validators).unwrap())
        .with_initial_fullnode_count(20)
        .add_network_test(wrap_with_realistic_env(num_validators, TwoTrafficsTest {
            inner_traffic: EmitJobRequest::default()
                .mode(EmitJobMode::ConstTps { tps: 30000 })
                .init_gas_price_multiplier(20),
            inner_success_criteria: SuccessCriteria::new(7500),
        }))
        // First start higher gas-fee traffic, to not cause issues with TxnEmitter setup - account creation
        .with_emit_job(
            EmitJobRequest::default()
                .mode(EmitJobMode::ConstTps { tps: 1000 })
                .gas_price(5 * aptos_global_constants::GAS_UNIT_PRICE),
        )
        .with_validator_override_node_config_fn(Arc::new(|config, _| {
            config.execution.processed_transactions_detailed_counters = true;
        }))
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = 300.into();
        }))
        .with_success_criteria(
            SuccessCriteria::new(900)
                .add_no_restarts()
                .add_wait_for_catchup_s(180) // 3 minutes
                .add_system_metrics_threshold(SystemMetricsThreshold::new(
                    // overload test uses more CPUs than others, so increase the limit
                    // Check that we don't use more than 28 CPU cores for 20% of the time.
                    MetricsThreshold::new(28.0, 20),
                    // TODO(ibalajiarun): Investigate the high utilization and adjust accordingly.
                    // Memory starts around 8GB, and grows around 8GB/hr in this test.
                    // Check that we don't use more than final expected memory for more than 20% of the time.
                    MetricsThreshold::new_gb(8.5 + 8.0 * (duration.as_secs_f64() / 3600.0), 20),
                ))
                .add_latency_threshold(10.0, LatencyType::P50)
                .add_latency_threshold(30.0, LatencyType::P90)
                .add_chain_progress(RELIABLE_REAL_ENV_PROGRESS_THRESHOLD.clone()),
        )
}

pub(crate) fn realistic_env_max_load_test(
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

    // resource override for long_running tests
    let resource_override = if long_running {
        NodeResourceOverride {
            storage_gib: Some(1000), // long running tests need more storage
            ..NodeResourceOverride::default()
        }
    } else {
        NodeResourceOverride::default() // no overrides
    };

    let mut success_criteria = SuccessCriteria::new(85)
        .add_system_metrics_threshold(SystemMetricsThreshold::new(
            // Check that we don't use more than 18 CPU cores for 15% of the time.
            MetricsThreshold::new(25.0, 15),
            // Memory starts around 8GB, and grows around 1.4GB/hr in this test.
            // Check that we don't use more than final expected memory for more than 20% of the time.
            MetricsThreshold::new_gb(8.0 + 1.4 * (duration_secs as f64 / 3600.0), 20),
        ))
        .add_no_restarts()
        .add_wait_for_catchup_s(
            // Give at least 60s for catchup, give 10% of the run for longer durations.
            (duration.as_secs() / 10).max(60),
        )
        .add_latency_threshold(3.6, LatencyType::P50)
        .add_latency_threshold(4.8, LatencyType::P70)
        .add_chain_progress(StateProgressThreshold {
            max_non_epoch_no_progress_secs: 15.0,
            max_epoch_no_progress_secs: 16.0,
            max_non_epoch_round_gap: 4,
            max_epoch_round_gap: 4,
        });

    // If the test is short lived, we should verify that there are no fullnode failures
    if !long_running {
        success_criteria = success_criteria.add_no_fullnode_failures();
    }

    if !ha_proxy {
        success_criteria = success_criteria.add_latency_breakdown_threshold(
            LatencyBreakdownThreshold::new_with_breach_pct(
                vec![
                    // quorum store backpressure is relaxed, so queueing happens here
                    (LatencyBreakdownSlice::MempoolToBlockCreation, 0.35 + 3.25),
                    // can be adjusted down if less backpressure
                    (LatencyBreakdownSlice::ConsensusProposalToOrdered, 0.85),
                    // can be adjusted down if less backpressure
                    (LatencyBreakdownSlice::ConsensusOrderedToCommit, 1.0),
                ],
                5,
            ),
        )
    }

    // Create the test
    let mempool_backlog = if ha_proxy { 28000 } else { 38000 };
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(num_validators).unwrap())
        .with_initial_fullnode_count(num_fullnodes)
        .add_network_test(wrap_with_realistic_env(num_validators, TwoTrafficsTest {
            inner_traffic: EmitJobRequest::default()
                .mode(EmitJobMode::MaxLoad { mempool_backlog })
                .init_gas_price_multiplier(20),
            inner_success_criteria: SuccessCriteria::new(
                if ha_proxy {
                    7000
                } else if long_running {
                    // This is for forge stable
                    11000
                } else {
                    // During land time we want to be less strict, otherwise we flaky fail
                    10000
                },
            ),
        }))
        .with_genesis_helm_config_fn(Arc::new(move |helm_values| {
            // Have single epoch change in land blocking, and a few on long-running
            helm_values["chain"]["epoch_duration_secs"] =
                (if long_running { 600 } else { 300 }).into();
            helm_values["chain"]["on_chain_consensus_config"] =
                serde_yaml::to_value(OnChainConsensusConfig::default_for_genesis())
                    .expect("must serialize");
            helm_values["chain"]["on_chain_execution_config"] =
                serde_yaml::to_value(OnChainExecutionConfig::default_for_genesis())
                    .expect("must serialize");
        }))
        .with_fullnode_override_node_config_fn(Arc::new(|config, _| {
            // Increase the consensus observer fallback thresholds
            config
                .consensus_observer
                .observer_fallback_progress_threshold_ms = 30_000; // 30 seconds
            config
                .consensus_observer
                .observer_fallback_sync_lag_threshold_ms = 45_000; // 45 seconds
        }))
        // First start higher gas-fee traffic, to not cause issues with TxnEmitter setup - account creation
        .with_emit_job(
            EmitJobRequest::default()
                .mode(EmitJobMode::ConstTps { tps: 100 })
                .gas_price(5 * aptos_global_constants::GAS_UNIT_PRICE)
                .latency_polling_interval(Duration::from_millis(100)),
        )
        .with_success_criteria(success_criteria)
        .with_validator_resource_override(resource_override)
        .with_fullnode_resource_override(resource_override)
}

pub(crate) fn realistic_network_tuned_for_throughput_test() -> ForgeConfig {
    // THE MOST COMMONLY USED TUNE-ABLES:
    const USE_CRAZY_MACHINES: bool = false;
    const ENABLE_VFNS: bool = true;
    const VALIDATOR_COUNT: usize = 12;

    // Config is based on these values. The target TPS should be a slight overestimate of
    // the actual throughput to be able to have reasonable queueing but also so throughput
    // will improve as performance improves.
    // Overestimate: causes mempool and/or batch queueing. Underestimate: not enough txns in blocks.
    const TARGET_TPS: usize = 15_000;
    // Overestimate: causes blocks to be too small. Underestimate: causes blocks that are too large.
    // Ideally, want the block size to take 200-250ms of execution time to match broadcast RTT.
    const MAX_TXNS_PER_BLOCK: usize = 3500;
    // Overestimate: causes batch queueing. Underestimate: not enough txns in quorum store.
    // This is validator latency, minus mempool queueing time.
    const VN_LATENCY_S: f64 = 2.5;
    // Overestimate: causes mempool queueing. Underestimate: not enough txns incoming.
    const VFN_LATENCY_S: f64 = 4.0;

    let mut forge_config = ForgeConfig::default()
            .with_initial_validator_count(NonZeroUsize::new(VALIDATOR_COUNT).unwrap())
            .add_network_test(MultiRegionNetworkEmulationTest::default_for_validator_count(VALIDATOR_COUNT))
            .with_emit_job(EmitJobRequest::default().mode(EmitJobMode::MaxLoad {
                mempool_backlog: (TARGET_TPS as f64 * VFN_LATENCY_S) as usize,
            }))
            .with_validator_override_node_config_fn(Arc::new(|config, _| {
                // Increase the state sync chunk sizes (consensus blocks are much larger than 1k)
                optimize_state_sync_for_throughput(config, 15_000);

                optimize_for_maximum_throughput(config, TARGET_TPS, MAX_TXNS_PER_BLOCK, VN_LATENCY_S);

                // Other consensus / Quroum store configs
                config.consensus.quorum_store_pull_timeout_ms = 200;

                // Experimental storage optimizations
                config.storage.rocksdb_configs.enable_storage_sharding = true;

                // Increase the concurrency level
                if USE_CRAZY_MACHINES {
                    config.execution.concurrency_level = 48;
                }
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
                        config_v4.transaction_shuffler_type = TransactionShufflerType::UseCaseAware {
                            sender_spread_factor: 256,
                            platform_use_case_spread_factor: 0,
                            user_use_case_spread_factor: 0,
                        };
                    }
                    OnChainExecutionConfig::V5(config_v5) => {
                        config_v5.block_gas_limit_type = BlockGasLimitType::NoLimit;
                        config_v5.transaction_shuffler_type = TransactionShufflerType::UseCaseAware {
                            sender_spread_factor: 256,
                            platform_use_case_spread_factor: 0,
                            user_use_case_spread_factor: 0,
                        };
                    }
                    OnChainExecutionConfig::V6(config_v6) => {
                        config_v6.block_gas_limit_type = BlockGasLimitType::NoLimit;
                        config_v6.transaction_shuffler_type = TransactionShufflerType::UseCaseAware {
                            sender_spread_factor: 256,
                            platform_use_case_spread_factor: 0,
                            user_use_case_spread_factor: 0,
                        };
                    }
                }
                helm_values["chain"]["on_chain_execution_config"] =
                    serde_yaml::to_value(on_chain_execution_config).expect("must serialize");
            }));

    if ENABLE_VFNS {
        forge_config = forge_config
            .with_initial_fullnode_count(VALIDATOR_COUNT)
            .with_fullnode_override_node_config_fn(Arc::new(|config, _| {
                // Increase the state sync chunk sizes (consensus blocks are much larger than 1k)
                optimize_state_sync_for_throughput(config, 15_000);

                // Experimental storage optimizations
                config.storage.rocksdb_configs.enable_storage_sharding = true;

                // Increase the concurrency level
                if USE_CRAZY_MACHINES {
                    config.execution.concurrency_level = 48;
                }
            }));
    }

    if USE_CRAZY_MACHINES {
        forge_config = forge_config
            .with_validator_resource_override(NodeResourceOverride {
                cpu_cores: Some(58),
                memory_gib: Some(200),
                storage_gib: Some(500), // assuming we're using these large marchines for long-running or expensive tests which need more disk
            })
            .with_fullnode_resource_override(NodeResourceOverride {
                cpu_cores: Some(58),
                memory_gib: Some(200),
                storage_gib: Some(500),
            })
            .with_success_criteria(
                SuccessCriteria::new(25000)
                    .add_no_restarts()
                    /* This test runs at high load, so we need more catchup time */
                    .add_wait_for_catchup_s(120),
                /* Doesn't work without event indices
                .add_chain_progress(RELIABLE_PROGRESS_THRESHOLD.clone()),
                 */
            );
    } else {
        forge_config = forge_config.with_success_criteria(
            SuccessCriteria::new(11000)
                .add_no_restarts()
                /* This test runs at high load, so we need more catchup time */
                .add_wait_for_catchup_s(120),
            /* Doesn't work without event indices
                .add_chain_progress(RELIABLE_PROGRESS_THRESHOLD.clone()),
            */
        );
    }

    forge_config
}

pub fn wrap_with_realistic_env<T: NetworkTest + 'static>(
    num_validators: usize,
    test: T,
) -> CompositeNetworkTest {
    CompositeNetworkTest::new(
        MultiRegionNetworkEmulationTest::default_for_validator_count(num_validators),
        test,
    )
}
