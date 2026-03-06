// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_config::config::NodeConfig;
use aptos_forge::{
    test_config::{ForgeTestConfig, TestCodeComponents},
    EmitJobMode, EmitJobRequest,
};
use aptos_testcases::{
    compatibility_test::SimpleValidatorUpgrade,
    consensus_reliability_tests::ChangingWorkingQuorumTest,
    framework_upgrade::FrameworkUpgrade,
    fullnode_reboot_stress_test::FullNodeRebootStressTest,
    load_vs_perf_benchmark::{LoadVsPerfBenchmark, TransactionWorkload, Workloads},
    modifiers::CpuChaosTest,
    multi_region_network_test::MultiRegionNetworkEmulationTest,
    performance_test::PerformanceBenchmark,
    public_fullnode_performance::PFNPerformance,
    three_region_simulation_test::ThreeRegionSameCloudSimulationTest,
    two_traffics_test::TwoTrafficsTest,
    CompositeNetworkTest,
};
use std::{collections::HashMap, sync::Arc};

use crate::suites::{
    realistic_environment::wrap_with_realistic_env,
    ungrouped::{
        background_traffic_for_sweep, background_traffic_for_sweep_with_latency,
        optimize_for_maximum_throughput, optimize_state_sync_for_throughput,
    },
};

use aptos_forge::{
    args::TransactionTypeArg,
    prometheus_metrics::LatencyBreakdownSlice,
    success_criteria::{
        LatencyBreakdownThreshold, LatencyType, SuccessCriteria,
    },
};

type TestFactory = Box<dyn Fn(&ForgeTestConfig) -> TestCodeComponents + Send + Sync>;

pub struct TestRegistry {
    tests: HashMap<String, TestFactory>,
}

impl TestRegistry {
    pub fn new() -> Self {
        Self {
            tests: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: &str, factory: TestFactory) {
        self.tests.insert(name.to_string(), factory);
    }

    pub fn get_with_config(&self, name: &str, config: &ForgeTestConfig) -> Option<TestCodeComponents> {
        self.tests.get(name).map(|factory| factory(config))
    }

    /// Build the default registry with all known test types
    pub fn build_default() -> Self {
        let mut registry = Self::new();

        // === Simple test types (no extra closures needed) ===

        registry.register("performance_benchmark", Box::new(|_| TestCodeComponents {
            network_tests: vec![Box::new(PerformanceBenchmark)],
            ..Default::default()
        }));

        registry.register("simple_validator_upgrade", Box::new(|_| TestCodeComponents {
            network_tests: vec![Box::new(SimpleValidatorUpgrade)],
            ..Default::default()
        }));

        registry.register("framework_upgrade", Box::new(|_| TestCodeComponents {
            network_tests: vec![Box::new(FrameworkUpgrade)],
            ..Default::default()
        }));

        registry.register("fullnode_reboot_stress_test", Box::new(|_| TestCodeComponents {
            network_tests: vec![Box::new(FullNodeRebootStressTest)],
            ..Default::default()
        }));

        registry.register("three_region_simulation", Box::new(|_| TestCodeComponents {
            network_tests: vec![Box::new(ThreeRegionSameCloudSimulationTest)],
            ..Default::default()
        }));

        // === Tests requiring realistic env wrapping ===

        registry.register("two_traffics_realistic_env", Box::new(|config| {
            let num_validators = config.initial_validator_count;
            let extra = config.extra.as_ref();

            let inner_mempool_backlog = extra
                .and_then(|e| e["inner_mempool_backlog"].as_u64())
                .map(|v| v as usize)
                .unwrap_or(38000);
            let inner_min_tps = extra
                .and_then(|e| e["inner_min_tps"].as_u64())
                .map(|v| v as usize)
                .unwrap_or(10000);
            let inner_gas_price_multiplier = extra
                .and_then(|e| e["inner_gas_price_multiplier"].as_u64())
                .unwrap_or(20);

            TestCodeComponents {
                network_tests: vec![Box::new(wrap_with_realistic_env(
                    num_validators,
                    TwoTrafficsTest {
                        inner_traffic: EmitJobRequest::default()
                            .mode(EmitJobMode::MaxLoad {
                                mempool_backlog: inner_mempool_backlog,
                            })
                            .init_gas_price_multiplier(inner_gas_price_multiplier),
                        inner_success_criteria: aptos_forge::success_criteria::SuccessCriteria::new(
                            inner_min_tps,
                        ),
                    },
                ))],
                ..Default::default()
            }
        }));

        registry.register("two_traffics_realistic_env_const_tps", Box::new(|config| {
            let num_validators = config.initial_validator_count;
            let extra = config.extra.as_ref();

            let inner_tps = extra
                .and_then(|e| e["inner_tps"].as_u64())
                .map(|v| v as usize)
                .unwrap_or(30000);
            let inner_min_tps = extra
                .and_then(|e| e["inner_min_tps"].as_u64())
                .map(|v| v as usize)
                .unwrap_or(7500);
            let inner_gas_price_multiplier = extra
                .and_then(|e| e["inner_gas_price_multiplier"].as_u64())
                .unwrap_or(20);

            TestCodeComponents {
                network_tests: vec![Box::new(wrap_with_realistic_env(
                    num_validators,
                    TwoTrafficsTest {
                        inner_traffic: EmitJobRequest::default()
                            .mode(EmitJobMode::ConstTps { tps: inner_tps })
                            .init_gas_price_multiplier(inner_gas_price_multiplier),
                        inner_success_criteria: aptos_forge::success_criteria::SuccessCriteria::new(
                            inner_min_tps,
                        ),
                    },
                ))],
                ..Default::default()
            }
        }));

        // === Consensus tests ===

        registry.register("consensus_only_realistic_env", Box::new(|config| {
            let num_validators = config.initial_validator_count;
            let extra = config.extra.as_ref();

            let target_tps = extra
                .and_then(|e| e["target_tps"].as_u64())
                .map(|v| v as usize)
                .unwrap_or(20_000);
            let max_txns_per_block = extra
                .and_then(|e| e["max_txns_per_block"].as_u64())
                .map(|v| v as usize)
                .unwrap_or(4_500);
            let vn_latency = extra
                .and_then(|e| e["vn_latency"].as_f64())
                .unwrap_or(3.0);

            TestCodeComponents {
                network_tests: vec![Box::new(CompositeNetworkTest::new(
                    MultiRegionNetworkEmulationTest::default_for_validator_count(num_validators),
                    CpuChaosTest::default(),
                ))],
                extra_validator_override_fn: Some(Arc::new(move |config, _| {
                    optimize_for_maximum_throughput(config, target_tps, max_txns_per_block, vn_latency);
                    crate::suites::state_sync::state_sync_config_execute_transactions(
                        &mut config.state_sync,
                    );
                })),
                ..Default::default()
            }
        }));

        // === Changing working quorum test ===

        registry.register("changing_working_quorum", Box::new(|config| {
            let extra = config.extra.as_ref();

            let min_tps = extra
                .and_then(|e| e["min_tps"].as_u64())
                .map(|v| v as usize)
                .unwrap_or(15);
            let always_healthy_nodes = extra
                .and_then(|e| e["always_healthy_nodes"].as_u64())
                .map(|v| v as usize)
                .unwrap_or(0);
            let max_down_nodes = extra
                .and_then(|e| e["max_down_nodes"].as_u64())
                .map(|v| v as usize)
                .unwrap_or(16);
            let num_large_validators = extra
                .and_then(|e| e["num_large_validators"].as_u64())
                .map(|v| v as usize)
                .unwrap_or(0);
            let add_execution_delay = extra
                .and_then(|e| e["add_execution_delay"].as_bool())
                .unwrap_or(false);
            let check_period_s = extra
                .and_then(|e| e["check_period_s"].as_u64())
                .map(|v| v as usize)
                .unwrap_or(53);

            TestCodeComponents {
                network_tests: vec![Box::new(ChangingWorkingQuorumTest {
                    min_tps,
                    always_healthy_nodes,
                    max_down_nodes,
                    num_large_validators,
                    add_execution_delay,
                    check_period_s,
                })],
                ..Default::default()
            }
        }));

        // === Multi-region benchmark ===

        registry.register("multiregion_benchmark", Box::new(|_| TestCodeComponents {
            network_tests: vec![Box::new(PerformanceBenchmark)],
            ..Default::default()
        }));

        // === PFN tests ===

        registry.register("pfn_const_tps", Box::new(|config| {
            let extra = config.extra.as_ref();

            let num_pfns = extra
                .and_then(|e| e["num_pfns"].as_u64())
                .unwrap_or(7);
            let add_cpu_chaos = extra
                .and_then(|e| e["add_cpu_chaos"].as_bool())
                .unwrap_or(false);
            let add_network_emulation = extra
                .and_then(|e| e["add_network_emulation"].as_bool())
                .unwrap_or(true);

            TestCodeComponents {
                network_tests: vec![Box::new(PFNPerformance::new(
                    num_pfns,
                    add_cpu_chaos,
                    add_network_emulation,
                    Some(Arc::new(|config: &mut NodeConfig, _| {
                        config.indexer_db_config.enable_event = true;
                    })),
                ))],
                ..Default::default()
            }
        }));

        // === Load vs perf sweep tests ===

        registry.register("load_sweep_realistic_env", Box::new(|config| {
            let num_validators = config.initial_validator_count;
            TestCodeComponents {
                network_tests: vec![Box::new(wrap_with_realistic_env(
                    num_validators,
                    LoadVsPerfBenchmark {
                        test: Box::new(PerformanceBenchmark),
                        workloads: Workloads::TPS(vec![10, 100, 1000, 3000, 5000, 7000]),
                        criteria: [
                            (9, 0.9, 1.0, 1.2, 0),
                            (95, 0.9, 1.1, 1.2, 0),
                            (950, 1.2, 1.3, 2.0, 0),
                            (2900, 1.4, 2.2, 2.5, 0),
                            (4800, 2.0, 2.5, 3.0, 0),
                            (6700, 2.5, 3.5, 5.0, 0),
                        ]
                        .into_iter()
                        .map(|(min_tps, max_lat_p50, max_lat_p90, max_lat_p99, max_expired_tps)| {
                            SuccessCriteria::new(min_tps)
                                .add_max_expired_tps(max_expired_tps as f64)
                                .add_max_failed_submission_tps(0.0)
                                .add_latency_threshold(max_lat_p50, LatencyType::P50)
                                .add_latency_threshold(max_lat_p90, LatencyType::P90)
                                .add_latency_threshold(max_lat_p99, LatencyType::P99)
                        })
                        .collect(),
                        background_traffic: background_traffic_for_sweep(5),
                    },
                ))],
                extra_validator_override_fn: Some(Arc::new(|config, _| {
                    config.execution.processed_transactions_detailed_counters = true;
                })),
                ..Default::default()
            }
        }));

        registry.register("workload_sweep_realistic_env", Box::new(|config| {
            let num_validators = config.initial_validator_count;
            TestCodeComponents {
                network_tests: vec![Box::new(wrap_with_realistic_env(
                    num_validators,
                    LoadVsPerfBenchmark {
                        test: Box::new(PerformanceBenchmark),
                        workloads: Workloads::TRANSACTIONS(vec![
                            TransactionWorkload::new(TransactionTypeArg::CoinTransfer, 20000),
                            TransactionWorkload::new(TransactionTypeArg::NoOp, 20000).with_num_modules(100),
                            TransactionWorkload::new(TransactionTypeArg::ModifyGlobalResource, 6000)
                                .with_transactions_per_account(1),
                            TransactionWorkload::new(TransactionTypeArg::TokenV2AmbassadorMint, 20000)
                                .with_unique_senders(),
                            TransactionWorkload::new(TransactionTypeArg::PublishPackage, 200)
                                .with_transactions_per_account(1),
                        ]),
                        criteria: [
                            (7000, 100, 0.3 + 0.5, 0.5, 0.5),
                            (8500, 100, 0.3 + 0.5, 0.5, 0.4),
                            (2000, 300, 0.3 + 1.0, 0.6, 1.0),
                            (3200, 500, 0.3 + 1.0, 0.7, 0.8),
                            (28, 5, 0.3 + 5.0, 0.7, 1.0),
                        ]
                        .into_iter()
                        .map(|(min_tps, max_expired, mempool_to_block, proposal_to_ordered, ordered_to_commit)| {
                            SuccessCriteria::new(min_tps)
                                .add_max_expired_tps(max_expired as f64)
                                .add_max_failed_submission_tps(200.0)
                                .add_no_restarts()
                                .add_latency_breakdown_threshold(LatencyBreakdownThreshold::new_strict(vec![
                                    (LatencyBreakdownSlice::MempoolToBlockCreation, mempool_to_block),
                                    (LatencyBreakdownSlice::ConsensusProposalToOrdered, proposal_to_ordered),
                                    (LatencyBreakdownSlice::ConsensusOrderedToCommit, ordered_to_commit),
                                ]))
                        })
                        .collect(),
                        background_traffic: background_traffic_for_sweep(5),
                    },
                ))],
                extra_validator_override_fn: Some(Arc::new(|config, _| {
                    config.execution.processed_transactions_detailed_counters = true;
                })),
                ..Default::default()
            }
        }));

        registry.register("orderbook_workload_sweep_realistic_env", Box::new(|config| {
            let num_validators = config.initial_validator_count;
            TestCodeComponents {
                network_tests: vec![Box::new(wrap_with_realistic_env(
                    num_validators,
                    LoadVsPerfBenchmark {
                        test: Box::new(PerformanceBenchmark),
                        workloads: Workloads::TRANSACTIONS(vec![
                            TransactionWorkload::new(TransactionTypeArg::OrderBookBalancedMatches25Pct1Market, 1000),
                            TransactionWorkload::new(TransactionTypeArg::OrderBookBalancedMatches25Pct50Markets, 5000),
                            TransactionWorkload::new(TransactionTypeArg::OrderBookBalancedMatches80Pct1Market, 1000),
                            TransactionWorkload::new(TransactionTypeArg::OrderBookBalancedMatches80Pct50Markets, 5000),
                            TransactionWorkload::new(TransactionTypeArg::OrderBookBalancedSizeSkewed80Pct1Market, 1000),
                            TransactionWorkload::new(TransactionTypeArg::OrderBookBalancedSizeSkewed80Pct50Markets, 5000),
                            TransactionWorkload::new(TransactionTypeArg::OrderBookNoMatches1Market, 1000),
                            TransactionWorkload::new(TransactionTypeArg::OrderBookNoMatches50Markets, 5000),
                        ]),
                        criteria: [
                            (350, 100, 0.3 + 1.0, 0.4, 0.2),
                            (1700, 100, 0.3 + 1.0, 0.4, 0.5),
                            (350, 300, 0.3 + 1.0, 0.4, 0.2),
                            (2000, 500, 0.3 + 1.0, 0.4, 0.5),
                            (320, 5, 0.3 + 1.0, 0.4, 0.25),
                            (1500, 5, 0.3 + 1.5, 0.4, 0.5),
                            (320, 100, 0.3 + 1.0, 0.4, 0.2),
                            (1700, 100, 0.3 + 1.0, 0.4, 0.7),
                        ]
                        .into_iter()
                        .map(|(min_tps, max_expired, mempool_to_block, proposal_to_ordered, ordered_to_commit)| {
                            SuccessCriteria::new(min_tps)
                                .add_max_expired_tps(max_expired as f64)
                                .add_max_failed_submission_tps(200.0)
                                .add_no_restarts()
                                .add_latency_breakdown_threshold(LatencyBreakdownThreshold::new_strict(vec![
                                    (LatencyBreakdownSlice::MempoolToBlockCreation, mempool_to_block),
                                    (LatencyBreakdownSlice::ConsensusProposalToOrdered, proposal_to_ordered),
                                    (LatencyBreakdownSlice::ConsensusOrderedToCommit, ordered_to_commit),
                                ]))
                        })
                        .collect(),
                        background_traffic: background_traffic_for_sweep(5),
                    },
                ))],
                extra_validator_override_fn: Some(Arc::new(|config, _| {
                    config.execution.processed_transactions_detailed_counters = true;
                })),
                ..Default::default()
            }
        }));

        registry.register("fairness_workload_sweep_realistic_env", Box::new(|config| {
            let num_validators = config.initial_validator_count;
            TestCodeComponents {
                network_tests: vec![Box::new(wrap_with_realistic_env(
                    num_validators,
                    LoadVsPerfBenchmark {
                        test: Box::new(PerformanceBenchmark),
                        workloads: Workloads::TRANSACTIONS(vec![
                            TransactionWorkload::new(TransactionTypeArg::ResourceGroupsGlobalWriteAndReadTag1KB, 100000),
                            TransactionWorkload::new(TransactionTypeArg::VectorPicture30k, 20000),
                            TransactionWorkload::new(TransactionTypeArg::SmartTablePicture1MWith256Change, 4000)
                                .with_transactions_per_account(1),
                        ]),
                        criteria: Vec::new(),
                        background_traffic: background_traffic_for_sweep_with_latency(
                            &[(2.0, 3.0, 8.0), (0.1, 25.0, 30.0), (0.1, 30.0, 45.0)],
                            false,
                        ),
                    },
                ))],
                extra_validator_override_fn: Some(Arc::new(|config, _| {
                    config.execution.processed_transactions_detailed_counters = true;
                })),
                ..Default::default()
            }
        }));

        registry.register("graceful_workload_sweep_realistic_env", Box::new(|config| {
            let num_validators = config.initial_validator_count;
            TestCodeComponents {
                network_tests: vec![Box::new(wrap_with_realistic_env(
                    num_validators,
                    LoadVsPerfBenchmark {
                        test: Box::new(PerformanceBenchmark),
                        workloads: Workloads::TRANSACTIONS(vec![
                            TransactionWorkload::new_const_tps(TransactionTypeArg::AccountGeneration, 2 * 7000),
                            TransactionWorkload::new_const_tps(TransactionTypeArg::ResourceGroupsGlobalWriteAndReadTag1KB, 3 * 1800),
                            TransactionWorkload::new_const_tps(TransactionTypeArg::SmartTablePicture1MWith256Change, 3 * 14),
                            TransactionWorkload::new_const_tps(TransactionTypeArg::SmartTablePicture1MWith1KChangeExceedsLimit, 3 * 12),
                            TransactionWorkload::new_const_tps(TransactionTypeArg::VectorPicture30k, 3 * 150),
                            TransactionWorkload::new_const_tps(TransactionTypeArg::ModifyGlobalFlagAggV2, 3 * 3500),
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
                                (0.1, 3.0, 5.0),
                                (0.1, 5.0, 10.0),
                                (0.1, 3.0, 10.0),
                            ],
                            true,
                        ),
                    },
                ))],
                extra_validator_override_fn: Some(Arc::new(|config, _| {
                    config.execution.processed_transactions_detailed_counters = true;
                })),
                ..Default::default()
            }
        }));

        // === Throughput tuned test ===

        registry.register("realistic_network_tuned_for_throughput", Box::new(|config| {
            let num_validators = config.initial_validator_count;
            let extra = config.extra.as_ref();

            let target_tps = extra
                .and_then(|e| e["target_tps"].as_u64())
                .map(|v| v as usize)
                .unwrap_or(15_000);
            let max_txns_per_block = extra
                .and_then(|e| e["max_txns_per_block"].as_u64())
                .map(|v| v as usize)
                .unwrap_or(3_500);
            let vn_latency = extra
                .and_then(|e| e["vn_latency"].as_f64())
                .unwrap_or(2.5);

            TestCodeComponents {
                network_tests: vec![Box::new(
                    MultiRegionNetworkEmulationTest::default_for_validator_count(num_validators),
                )],
                extra_validator_override_fn: Some(Arc::new(move |config, _| {
                    optimize_state_sync_for_throughput(config, 15_000);
                    optimize_for_maximum_throughput(config, target_tps, max_txns_per_block, vn_latency);
                    config.consensus.quorum_store_pull_timeout_ms = 200;
                    config.storage.rocksdb_configs.enable_storage_sharding = true;
                })),
                extra_fullnode_override_fn: Some(Arc::new(|config, _| {
                    optimize_state_sync_for_throughput(config, 15_000);
                    config.storage.rocksdb_configs.enable_storage_sharding = true;
                })),
                ..Default::default()
            }
        }));

        // === Workload mix test ===

        registry.register("workload_mix", Box::new(|_| TestCodeComponents {
            network_tests: vec![Box::new(PerformanceBenchmark)],
            extra_validator_override_fn: Some(Arc::new(|config, _| {
                config.execution.processed_transactions_detailed_counters = true;
            })),
            ..Default::default()
        }));

        // === Single VFN perf ===

        registry.register("single_vfn_perf", Box::new(|_| TestCodeComponents {
            network_tests: vec![Box::new(PerformanceBenchmark)],
            extra_validator_override_fn: Some(Arc::new(|config, _| {
                config
                    .consensus
                    .quorum_store
                    .back_pressure
                    .dynamic_max_txn_per_s = 5500;
            })),
            ..Default::default()
        }));

        registry
    }
}
