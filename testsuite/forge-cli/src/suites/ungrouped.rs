// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::{
    db::large_db_simple_test,
    realistic_workloads::{
        individual_workload_tests, mainnet_like_simulation_test, workload_mix_test,
        workload_vs_perf_benchmark,
    },
    state_sync::{
        state_sync_config_apply_transaction_outputs, state_sync_config_execute_transactions,
    },
};
use crate::{suites::realistic_environment::wrap_with_realistic_env, TestCommand};
use anyhow::Result;
use aptos_cached_packages::aptos_stdlib;
use aptos_config::config::{ConsensusConfig, MempoolConfig, NodeConfig};
use aptos_forge::{
    args::TransactionTypeArg,
    emitter::NumAccountsMode,
    prometheus_metrics::LatencyBreakdownSlice,
    success_criteria::{
        LatencyBreakdownThreshold, LatencyType, MetricsThreshold, StateProgressThreshold,
        SuccessCriteria, SystemMetricsThreshold,
    },
    AdminContext, AdminTest, AptosContext, AptosTest, EmitJobMode, EmitJobRequest, ForgeConfig,
    NetworkContext, NetworkContextSynchronizer, NetworkTest, NodeResourceOverride, Test,
    WorkflowProgress,
};
use aptos_logger::info;
use aptos_rest_client::Client as RestClient;
use aptos_sdk::{
    move_types::account_address::AccountAddress,
    types::on_chain_config::{OnChainConsensusConfig, OnChainExecutionConfig},
};
use aptos_testcases::{
    self,
    consensus_reliability_tests::ChangingWorkingQuorumTest,
    forge_setup_test::ForgeSetupTest,
    framework_upgrade::FrameworkUpgrade,
    fullnode_reboot_stress_test::FullNodeRebootStressTest,
    generate_traffic,
    load_vs_perf_benchmark::{BackgroundTraffic, LoadVsPerfBenchmark, Workloads},
    modifiers::CpuChaosTest,
    multi_region_network_test::MultiRegionNetworkEmulationTest,
    network_bandwidth_test::NetworkBandwidthTest,
    network_loss_test::NetworkLossTest,
    network_partition_test::NetworkPartitionTest,
    performance_test::PerformanceBenchmark,
    quorum_store_onchain_enable_test::QuorumStoreOnChainEnableTest,
    reconfiguration_test::ReconfigurationTest,
    three_region_simulation_test::ThreeRegionSameCloudSimulationTest,
    twin_validator_test::TwinValidatorTest,
    two_traffics_test::TwoTrafficsTest,
    validator_join_leave_test::ValidatorJoinLeaveTest,
    validator_reboot_stress_test::ValidatorRebootStressTest,
    CompositeNetworkTest,
};
use async_trait::async_trait;
use futures::{stream::FuturesUnordered, StreamExt as _};
use once_cell::sync::Lazy;
use std::{
    self, env,
    num::NonZeroUsize,
    ops::DerefMut as _,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};
use tokio::{runtime::Runtime, select};
use url::Url;

pub fn get_ungrouped_test(test_name: &str) -> Option<ForgeConfig> {
    // Otherwise, check the test name against the ungrouped test suites
    match test_name {
        // Consensus
        "consensus_only_realistic_env_max_tps" => Some(run_consensus_only_realistic_env_max_tps()),
        "different_node_speed_and_reliability_test" => {
            Some(different_node_speed_and_reliability_test())
        },
        "quorum_store_reconfig_enable_test" => Some(quorum_store_reconfig_enable_test()),
        "epoch_changer_performance" => Some(epoch_changer_performance()),
        "changing_working_quorum_test" => Some(changing_working_quorum_test()),
        "changing_working_quorum_test_high_load" => Some(changing_working_quorum_test_high_load()),
        // not scheduled on continuous
        "load_vs_perf_benchmark" => Some(load_vs_perf_benchmark()),
        // maximizing number of rounds and epochs within a given time, to stress test consensus
        // so using small constant traffic, small blocks and fast rounds, and short epochs.
        // reusing changing_working_quorum_test just for invariants/asserts, but with max_down_nodes = 0.
        "consensus_stress_test" => Some(consensus_stress_test()),
        // not scheduled on continuous
        "large_test_only_few_nodes_down" => Some(large_test_only_few_nodes_down()),

        // System tests
        "gather_metrics" => Some(gather_metrics()),
        "setup_test" => Some(setup_test()),
        "config" => Some(reconfiguration_test()),

        // Db
        "large_db_simple_test" => Some(large_db_simple_test()),

        // Network tests
        "network_bandwidth" => Some(network_bandwidth()),
        "network_partition" => Some(network_partition()),
        "twin_validator_test" => Some(twin_validator_test()),
        "validator_reboot_stress_test" => Some(validator_reboot_stress_test()),
        "validators_join_and_leave" => Some(validators_join_and_leave()),
        "single_vfn_perf" => Some(single_vfn_perf()),
        "fullnode_reboot_stress_test" => Some(fullnode_reboot_stress_test()),

        // Workloads
        "account_creation" | "nft_mint" | "publishing" | "module_loading"
        | "write_new_resource" => Some(individual_workload_tests(test_name.into())),
        "mainnet_like_simulation_test" => Some(mainnet_like_simulation_test()),
        "workload_mix" => Some(workload_mix_test()),
        "workload_vs_perf_benchmark" => Some(workload_vs_perf_benchmark()),
        _ => None,
    }
}

// common metrics thresholds:
pub static SYSTEM_12_CORES_5GB_THRESHOLD: Lazy<SystemMetricsThreshold> = Lazy::new(|| {
    SystemMetricsThreshold::new(
        // Check that we don't use more than 12 CPU cores for 30% of the time.
        MetricsThreshold::new(12.0, 30),
        // Check that we don't use more than 5 GB of memory for 30% of the time.
        MetricsThreshold::new_gb(5.0, 30),
    )
});
pub static SYSTEM_12_CORES_10GB_THRESHOLD: Lazy<SystemMetricsThreshold> = Lazy::new(|| {
    SystemMetricsThreshold::new(
        // Check that we don't use more than 12 CPU cores for 30% of the time.
        MetricsThreshold::new(12.0, 30),
        // Check that we don't use more than 10 GB of memory for 30% of the time.
        MetricsThreshold::new_gb(10.0, 30),
    )
});

pub static RELIABLE_PROGRESS_THRESHOLD: Lazy<StateProgressThreshold> =
    Lazy::new(|| StateProgressThreshold {
        max_non_epoch_no_progress_secs: 10.0,
        max_epoch_no_progress_secs: 10.0,
        max_non_epoch_round_gap: 4,
        max_epoch_round_gap: 4,
    });

pub static PROGRESS_THRESHOLD_20_6: Lazy<StateProgressThreshold> =
    Lazy::new(|| StateProgressThreshold {
        max_non_epoch_no_progress_secs: 20.0,
        max_epoch_no_progress_secs: 20.0,
        max_non_epoch_round_gap: 6,
        max_epoch_round_gap: 6,
    });

pub static RELIABLE_REAL_ENV_PROGRESS_THRESHOLD: Lazy<StateProgressThreshold> =
    Lazy::new(|| StateProgressThreshold {
        max_non_epoch_no_progress_secs: 30.0,
        max_epoch_no_progress_secs: 30.0,
        max_non_epoch_round_gap: 10,
        max_epoch_round_gap: 10,
    });

/// Provides a forge config that runs the swarm forever (unless killed)
pub fn run_forever() -> ForgeConfig {
    ForgeConfig::default()
        .add_admin_test(GetMetadata)
        .with_genesis_module_bundle(aptos_cached_packages::head_release_bundle().clone())
        .add_aptos_test(RunForever)
}

pub fn local_test_suite() -> ForgeConfig {
    ForgeConfig::default()
        .add_aptos_test(FundAccount)
        .add_aptos_test(TransferCoins)
        .add_admin_test(GetMetadata)
        .add_network_test(RestartValidator)
        .add_network_test(EmitTransaction)
        .with_genesis_module_bundle(aptos_cached_packages::head_release_bundle().clone())
}

pub fn k8s_test_suite() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(30).unwrap())
        .add_aptos_test(FundAccount)
        .add_aptos_test(TransferCoins)
        .add_admin_test(GetMetadata)
        .add_network_test(EmitTransaction)
        .add_network_test(FrameworkUpgrade)
        .add_network_test(PerformanceBenchmark)
}

fn mempool_config_practically_non_expiring(mempool_config: &mut MempoolConfig) {
    mempool_config.capacity = 3_000_000;
    mempool_config.capacity_bytes = (3_u64 * 1024 * 1024 * 1024) as usize;
    mempool_config.capacity_per_user = 100_000;
    mempool_config.system_transaction_timeout_secs = 5 * 60 * 60;
    mempool_config.system_transaction_gc_interval_ms = 5 * 60 * 60_000;
}

fn run_consensus_only_realistic_env_max_tps() -> ForgeConfig {
    let num_validators = 20;
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(num_validators).unwrap())
        .with_emit_job(
            EmitJobRequest::default()
                .mode(EmitJobMode::MaxLoad {
                    mempool_backlog: 300000,
                })
                .txn_expiration_time_secs(5 * 60),
        )
        .add_network_test(CompositeNetworkTest::new(
            MultiRegionNetworkEmulationTest::default_for_validator_count(num_validators),
            CpuChaosTest::default(),
        ))
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            // no epoch change.
            helm_values["chain"]["epoch_duration_secs"] = (24 * 3600).into();
        }))
        .with_validator_override_node_config_fn(Arc::new(|config, _| {
            optimize_for_maximum_throughput(config, 20_000, 4_500, 3.0);
            state_sync_config_execute_transactions(&mut config.state_sync);
        }))
        // TODO(ibalajiarun): tune these success critiera after we have a better idea of the test behavior
        .with_success_criteria(
            SuccessCriteria::new(10000)
                .add_no_restarts()
                .add_wait_for_catchup_s(240)
                .add_chain_progress(PROGRESS_THRESHOLD_20_6.clone()),
        )
}

fn quorum_store_backlog_txn_limit_count(
    config: &mut NodeConfig,
    target_tps: usize,
    vn_latency: f64,
) {
    config
        .consensus
        .quorum_store
        .back_pressure
        .backlog_txn_limit_count = (target_tps as f64 * vn_latency) as u64;
    config
        .consensus
        .quorum_store
        .back_pressure
        .dynamic_max_txn_per_s = 4000;
}

pub fn optimize_for_maximum_throughput(
    config: &mut NodeConfig,
    target_tps: usize,
    max_txns_per_block: usize,
    vn_latency: f64,
) {
    mempool_config_practically_non_expiring(&mut config.mempool);

    config.consensus.max_sending_block_txns_after_filtering = max_txns_per_block as u64;
    config.consensus.max_sending_block_txns = config
        .consensus
        .max_sending_block_txns
        .max(max_txns_per_block as u64 * 3 / 2);
    config.consensus.max_receiving_block_txns =
        (config.consensus.max_sending_block_txns as f64 * 4.0 / 3.0) as u64;
    config.consensus.max_sending_block_bytes = 10 * 1024 * 1024;
    config.consensus.max_receiving_block_bytes = 12 * 1024 * 1024;
    config.consensus.pipeline_backpressure = vec![];
    config.consensus.chain_health_backoff = vec![];

    quorum_store_backlog_txn_limit_count(config, target_tps, vn_latency);

    config.consensus.quorum_store.sender_max_batch_txns = 500;
    config
        .consensus
        .min_max_txns_in_block_after_filtering_from_backpressure =
        2 * config.consensus.quorum_store.sender_max_batch_txns as u64;
    config.consensus.quorum_store.sender_max_batch_bytes = 4 * 1024 * 1024;
    config.consensus.quorum_store.sender_max_num_batches = 100;
    config.consensus.quorum_store.sender_max_total_txns = 4000;
    config.consensus.quorum_store.sender_max_total_bytes = 8 * 1024 * 1024;
    config.consensus.quorum_store.receiver_max_batch_txns = 1000;
    config.consensus.quorum_store.receiver_max_batch_bytes = 8 * 1024 * 1024;
    config.consensus.quorum_store.receiver_max_num_batches = 200;
    config.consensus.quorum_store.receiver_max_total_txns = 8000;
    config.consensus.quorum_store.receiver_max_total_bytes = 16 * 1024 * 1024;
}

fn twin_validator_test() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(7).unwrap())
        .with_initial_fullnode_count(5)
        .add_network_test(TwinValidatorTest)
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = 300.into();
        }))
        .with_success_criteria(
            SuccessCriteria::new(5500)
                .add_no_restarts()
                .add_wait_for_catchup_s(60)
                .add_system_metrics_threshold(SYSTEM_12_CORES_5GB_THRESHOLD.clone())
                .add_chain_progress(RELIABLE_PROGRESS_THRESHOLD.clone()),
        )
}

fn different_node_speed_and_reliability_test() -> ForgeConfig {
    changing_working_quorum_test_helper(
        20,
        120,
        70,
        50,
        true,
        false,
        false,
        ChangingWorkingQuorumTest {
            min_tps: 30,
            always_healthy_nodes: 6,
            max_down_nodes: 5,
            num_large_validators: 3,
            add_execution_delay: true,
            check_period_s: 27,
        },
    )
}

fn large_test_only_few_nodes_down() -> ForgeConfig {
    changing_working_quorum_test_helper(
        60,
        120,
        100,
        70,
        false,
        false,
        false,
        ChangingWorkingQuorumTest {
            min_tps: 50,
            always_healthy_nodes: 40,
            max_down_nodes: 10,
            num_large_validators: 0,
            add_execution_delay: false,
            check_period_s: 27,
        },
    )
}

fn changing_working_quorum_test_high_load() -> ForgeConfig {
    changing_working_quorum_test_helper(
        16,
        120,
        500,
        300,
        false,
        true,
        true,
        ChangingWorkingQuorumTest {
            min_tps: 50,
            always_healthy_nodes: 0,
            max_down_nodes: 16,
            num_large_validators: 0,
            add_execution_delay: false,
            // Use longer check duration, as we are bringing enough nodes
            // to require state-sync to catch up to have consensus.
            check_period_s: 53,
        },
    )
}

fn changing_working_quorum_test() -> ForgeConfig {
    changing_working_quorum_test_helper(
        16,
        120,
        100,
        70,
        true,
        true,
        true,
        ChangingWorkingQuorumTest {
            min_tps: 15,
            always_healthy_nodes: 0,
            max_down_nodes: 16,
            num_large_validators: 0,
            add_execution_delay: false,
            // Use longer check duration, as we are bringing enough nodes
            // to require state-sync to catch up to have consensus.
            check_period_s: 53,
        },
    )
}

fn consensus_stress_test() -> ForgeConfig {
    changing_working_quorum_test_helper(
        10,
        60,
        100,
        80,
        true,
        false,
        false,
        ChangingWorkingQuorumTest {
            min_tps: 50,
            always_healthy_nodes: 10,
            max_down_nodes: 0,
            num_large_validators: 0,
            add_execution_delay: false,
            check_period_s: 27,
        },
    )
}

fn background_emit_request(high_gas_price: bool) -> EmitJobRequest {
    let mut result = EmitJobRequest::default()
        .num_accounts_mode(NumAccountsMode::TransactionsPerAccount(1))
        .mode(EmitJobMode::ConstTps { tps: 10 });
    if high_gas_price {
        result = result.gas_price(5 * aptos_global_constants::GAS_UNIT_PRICE);
    }
    result
}

pub fn background_traffic_for_sweep(num_cases: usize) -> Option<BackgroundTraffic> {
    Some(BackgroundTraffic {
        traffic: background_emit_request(true),
        criteria: std::iter::repeat(9.5)
            .take(num_cases)
            .map(|min_tps| {
                SuccessCriteria::new_float(min_tps)
                    .add_max_expired_tps(0.1)
                    .add_max_failed_submission_tps(0.0)
            })
            .collect(),
    })
}

pub fn background_traffic_for_sweep_with_latency(
    criteria_expired_p50_and_p90: &[(f64, f32, f32)],
    high_gas_price: bool,
) -> Option<BackgroundTraffic> {
    Some(BackgroundTraffic {
        traffic: background_emit_request(high_gas_price),
        criteria: criteria_expired_p50_and_p90
            .iter()
            .map(|(expired, p50, p90)| {
                SuccessCriteria::new_float(9.5)
                    .add_max_expired_tps(*expired)
                    .add_max_failed_submission_tps(0.0)
                    .add_latency_threshold(*p50, LatencyType::P50)
                    .add_latency_threshold(*p90, LatencyType::P90)
            })
            .collect(),
    })
}

fn load_vs_perf_benchmark() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(20).unwrap())
        .with_initial_fullnode_count(10)
        .add_network_test(LoadVsPerfBenchmark {
            test: Box::new(PerformanceBenchmark),
            workloads: Workloads::TPS(vec![
                200, 1000, 3000, 5000, 7000, 7500, 8000, 9000, 10000, 12000, 15000,
            ]),
            criteria: Vec::new(),
            background_traffic: None,
        })
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

pub(crate) fn single_cluster_test(
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
    let mempool_backlog = if ha_proxy { 300 } else { 300 };
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(num_validators).unwrap())
        .with_initial_fullnode_count(num_fullnodes)
        .add_network_test(TwoTrafficsTest {
            inner_traffic: EmitJobRequest::default()
                .mode(EmitJobMode::MaxLoad { mempool_backlog })
                .init_gas_price_multiplier(20),
            inner_success_criteria: SuccessCriteria::new(
                if ha_proxy {
                    100
                } else if long_running {
                    // This is for forge stable
                    100
                } else {
                    // During land time we want to be less strict, otherwise we flaky fail
                    100
                },
            ),
        })
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
        .with_emit_job(
            EmitJobRequest::default()
                .mode(EmitJobMode::ConstTps { tps: 10 })
                .gas_price(5 * aptos_global_constants::GAS_UNIT_PRICE),
        )
        .with_success_criteria(success_criteria)
        .with_validator_resource_override(resource_override)
        .with_fullnode_resource_override(resource_override)
}

pub fn mixed_emit_job() -> EmitJobRequest {
    EmitJobRequest::default()
        .mode(EmitJobMode::MaxLoad {
            mempool_backlog: 10000,
        })
        .transaction_mix(vec![
            // To test both variants, make module publish with such frequency, so that there are
            // similar number of sequential and parallel blocks.
            // For other transactions, make more expensive transactions somewhat rarer.
            (
                TransactionTypeArg::AccountGeneration.materialize_default(),
                10000,
            ),
            (
                TransactionTypeArg::CoinTransfer.materialize_default(),
                10000,
            ),
            (TransactionTypeArg::PublishPackage.materialize_default(), 3),
            (
                TransactionTypeArg::Batch100Transfer.materialize_default(),
                100,
            ),
            (
                TransactionTypeArg::VectorPicture30k.materialize_default(),
                100,
            ),
            (
                TransactionTypeArg::SmartTablePicture30KWith200Change.materialize(
                    1,
                    true,
                    WorkflowProgress::when_done_default(),
                ),
                100,
            ),
            (
                TransactionTypeArg::TokenV2AmbassadorMint.materialize_default(),
                10000,
            ),
            (
                TransactionTypeArg::ModifyGlobalResource.materialize_default(),
                1000,
            ),
            (
                TransactionTypeArg::ModifyGlobalResourceAggV2.materialize_default(),
                1000,
            ),
            (
                TransactionTypeArg::ModifyGlobalFlagAggV2.materialize_default(),
                1000,
            ),
            (
                TransactionTypeArg::ModifyGlobalBoundedAggV2.materialize_default(),
                1000,
            ),
            (
                TransactionTypeArg::ResourceGroupsGlobalWriteTag1KB.materialize_default(),
                1000,
            ),
            (
                TransactionTypeArg::ResourceGroupsGlobalWriteAndReadTag1KB.materialize_default(),
                1000,
            ),
            (
                TransactionTypeArg::TokenV1NFTMintAndTransferSequential.materialize_default(),
                1000,
            ),
            (
                TransactionTypeArg::TokenV1FTMintAndTransfer.materialize_default(),
                10000,
            ),
        ])
}

// framework_usecases can have new features, so might fail publishing.
pub fn mixed_compatible_emit_job() -> EmitJobRequest {
    EmitJobRequest::default()
        .mode(EmitJobMode::MaxLoad {
            mempool_backlog: 10000,
        })
        .transaction_mix(vec![
            // To test both variants, make module publish with such frequency, so that there are
            // similar number of sequential and parallel blocks.
            // For other transactions, make more expensive transactions somewhat rarer.
            (
                TransactionTypeArg::AccountGeneration.materialize_default(),
                10000,
            ),
            (
                TransactionTypeArg::CoinTransfer.materialize_default(),
                10000,
            ),
            (TransactionTypeArg::PublishPackage.materialize_default(), 3),
            (
                TransactionTypeArg::Batch100Transfer.materialize_default(),
                100,
            ),
            (
                TransactionTypeArg::VectorPicture30k.materialize_default(),
                100,
            ),
            (
                TransactionTypeArg::SmartTablePicture30KWith200Change.materialize(
                    1,
                    true,
                    WorkflowProgress::when_done_default(),
                ),
                100,
            ),
            (
                TransactionTypeArg::TokenV2AmbassadorMint.materialize_default(),
                10000,
            ),
            // (
            //     TransactionTypeArg::ModifyGlobalResource.materialize_default(),
            //     1000,
            // ),
            // (
            //     TransactionTypeArg::ModifyGlobalResourceAggV2.materialize_default(),
            //     1000,
            // ),
            // (
            //     TransactionTypeArg::ModifyGlobalFlagAggV2.materialize_default(),
            //     1000,
            // ),
            // (
            //     TransactionTypeArg::ModifyGlobalBoundedAggV2.materialize_default(),
            //     1000,
            // ),
            // (
            //     TransactionTypeArg::ResourceGroupsGlobalWriteTag1KB.materialize_default(),
            //     1000,
            // ),
            // (
            //     TransactionTypeArg::ResourceGroupsGlobalWriteAndReadTag1KB.materialize_default(),
            //     1000,
            // ),
            // (
            //     TransactionTypeArg::TokenV1NFTMintAndTransferSequential.materialize_default(),
            //     1000,
            // ),
            // (
            //     TransactionTypeArg::TokenV1FTMintAndTransfer.materialize_default(),
            //     10000,
            // ),
        ])
}

fn fullnode_reboot_stress_test() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(7).unwrap())
        .with_initial_fullnode_count(7)
        .add_network_test(FullNodeRebootStressTest)
        .with_emit_job(EmitJobRequest::default().mode(EmitJobMode::ConstTps { tps: 5000 }))
        .with_success_criteria(SuccessCriteria::new(2000).add_wait_for_catchup_s(600))
}

fn validator_reboot_stress_test() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(7).unwrap())
        .with_initial_fullnode_count(1)
        .add_network_test(ValidatorRebootStressTest {
            num_simultaneously: 2,
            down_time_secs: 5.0,
            pause_secs: 5.0,
        })
        .with_success_criteria(SuccessCriteria::new(2000).add_wait_for_catchup_s(600))
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = 120.into();
        }))
}

fn apply_config_for_quorum_store_single_node(config: &mut NodeConfig) {
    config
        .consensus
        .quorum_store
        .back_pressure
        .dynamic_max_txn_per_s = 5500;
}

fn single_vfn_perf() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(1).unwrap())
        .with_initial_fullnode_count(1)
        .add_network_test(PerformanceBenchmark)
        .with_success_criteria(
            SuccessCriteria::new(5000)
                .add_no_restarts()
                .add_wait_for_catchup_s(240),
        )
        .with_validator_override_node_config_fn(Arc::new(|config, _| {
            apply_config_for_quorum_store_single_node(config);
        }))
}

fn network_bandwidth() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(8).unwrap())
        .add_network_test(NetworkBandwidthTest)
}

fn network_partition() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(10).unwrap())
        .add_network_test(NetworkPartitionTest)
        .with_success_criteria(
            SuccessCriteria::new(2500)
                .add_no_restarts()
                .add_wait_for_catchup_s(240),
        )
        .with_validator_override_node_config_fn(Arc::new(|config, _| {
            apply_config_for_quorum_store_single_node(config);
        }))
}

fn epoch_changer_performance() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(5).unwrap())
        .with_initial_fullnode_count(2)
        .add_network_test(PerformanceBenchmark)
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = 60.into();
        }))
}

/// The config for running a validator join and leave test.
fn validators_join_and_leave() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(20).unwrap())
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = 60.into();
            helm_values["chain"]["allow_new_validators"] = true.into();
        }))
        .add_network_test(ValidatorJoinLeaveTest)
        .with_success_criteria(
            SuccessCriteria::new(5000)
                .add_no_restarts()
                .add_wait_for_catchup_s(240)
                .add_system_metrics_threshold(SYSTEM_12_CORES_10GB_THRESHOLD.clone())
                .add_chain_progress(RELIABLE_PROGRESS_THRESHOLD.clone()),
        )
}

/// Optimizes the state sync configs for throughput.
/// `max_chunk_size` is the maximum number of transactions to include in a chunk.
pub fn optimize_state_sync_for_throughput(node_config: &mut NodeConfig, max_chunk_size: u64) {
    let max_chunk_bytes = 40 * 1024 * 1024; // 10x the current limit (to prevent execution fallback)

    // Update the chunk sizes for the data client
    let data_client_config = &mut node_config.state_sync.aptos_data_client;
    data_client_config.max_transaction_chunk_size = max_chunk_size;
    data_client_config.max_transaction_output_chunk_size = max_chunk_size;

    // Update the chunk sizes for the storage service
    let storage_service_config = &mut node_config.state_sync.storage_service;
    storage_service_config.max_transaction_chunk_size = max_chunk_size;
    storage_service_config.max_transaction_output_chunk_size = max_chunk_size;

    // Update the chunk bytes for the storage service
    storage_service_config.max_network_chunk_bytes = max_chunk_bytes;
}

pub fn pre_release_suite() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(30).unwrap())
        .add_network_test(NetworkBandwidthTest)
}

pub fn chaos_test_suite(duration: Duration) -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(30).unwrap())
        .add_network_test(NetworkBandwidthTest)
        .add_network_test(ThreeRegionSameCloudSimulationTest)
        .add_network_test(NetworkLossTest)
        .with_success_criteria(
            SuccessCriteria::new(
                if duration > Duration::from_secs(1200) {
                    100
                } else {
                    1000
                },
            )
            .add_no_restarts()
            .add_system_metrics_threshold(SYSTEM_12_CORES_5GB_THRESHOLD.clone()),
        )
}

pub fn changing_working_quorum_test_helper(
    num_validators: usize,
    epoch_duration: usize,
    target_tps: usize,
    min_avg_tps: usize,
    apply_txn_outputs: bool,
    use_chain_backoff: bool,
    allow_errors: bool,
    test: ChangingWorkingQuorumTest,
) -> ForgeConfig {
    let config = ForgeConfig::default();
    let num_large_validators = test.num_large_validators;
    let max_down_nodes = test.max_down_nodes;
    config
        .with_initial_validator_count(NonZeroUsize::new(num_validators).unwrap())
        .with_initial_fullnode_count(
            if max_down_nodes == 0 {
                0
            } else {
                std::cmp::max(2, target_tps / 1000)
            },
        )
        .add_network_test(test)
        .with_genesis_helm_config_fn(Arc::new(move |helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = epoch_duration.into();
            helm_values["genesis"]["validator"]["num_validators_with_larger_stake"] =
                num_large_validators.into();
        }))
        .with_validator_override_node_config_fn(Arc::new(move |config, _| {
            config.api.failpoints_enabled = true;
            let block_size = (target_tps / 4) as u64;

            config.consensus.max_sending_block_txns = block_size;
            config.consensus.max_receiving_block_txns = block_size;
            config.consensus.round_initial_timeout_ms = 500;
            config.consensus.round_timeout_backoff_exponent_base = 1.0;
            config.consensus.quorum_store_poll_time_ms = 100;
            config.consensus.rand_rb_config.backoff_policy_max_delay_ms = 1000;

            let mut min_block_txns = block_size;
            let mut chain_health_backoff = ConsensusConfig::default().chain_health_backoff;
            if use_chain_backoff {
                // Generally if we are stress testing the consensus, we don't want to slow it down.
                chain_health_backoff = vec![];
            } else {
                for (i, item) in chain_health_backoff.iter_mut().enumerate() {
                    // as we have lower TPS, make limits smaller
                    item.max_sending_block_txns_after_filtering_override =
                        (block_size / 2_u64.pow(i as u32 + 1)).max(2);
                    min_block_txns =
                        min_block_txns.min(item.max_sending_block_txns_after_filtering_override);
                    // as we have fewer nodes, make backoff triggered earlier:
                    item.backoff_if_below_participating_voting_power_percentage = 90 - i * 5;
                }
            }
            config.consensus.quorum_store.sender_max_batch_txns = min_block_txns as usize;
            config.consensus.quorum_store.receiver_max_batch_txns = min_block_txns as usize;

            config.consensus.chain_health_backoff = chain_health_backoff;

            // Override the syncing mode of all nodes to use transaction output syncing.
            // TODO(joshlind): remove me once we move back to output syncing by default.
            if apply_txn_outputs {
                state_sync_config_apply_transaction_outputs(&mut config.state_sync);
            }
        }))
        .with_fullnode_override_node_config_fn(Arc::new(move |config, _| {
            // Override the syncing mode of all nodes to use transaction output syncing.
            // TODO(joshlind): remove me once we move back to output syncing by default.
            if apply_txn_outputs {
                state_sync_config_execute_transactions(&mut config.state_sync);
            }
        }))
        .with_emit_job(
            EmitJobRequest::default()
                .mode(EmitJobMode::ConstTps { tps: target_tps })
                .transaction_mix(vec![
                    (TransactionTypeArg::CoinTransfer.materialize_default(), 80),
                    (
                        TransactionTypeArg::AccountGeneration.materialize_default(),
                        20,
                    ),
                ]),
        )
        .with_success_criteria({
            let success_criteria = SuccessCriteria::new(min_avg_tps)
                .add_no_restarts()
                .add_wait_for_catchup_s(30)
                .add_chain_progress({
                    let max_no_progress_secs = if max_down_nodes == 0 {
                        // very aggressive if no nodes are expected to be down
                        3.0
                    } else if max_down_nodes * 3 + 1 + 2 < num_validators {
                        // number of down nodes is at least 2 below the quorum limit, so
                        // we can still be reasonably aggqressive
                        15.0
                    } else {
                        // number of down nodes is close to the quorum limit, so
                        // make a check a bit looser, as state sync might be required
                        // to get the quorum back.
                        40.0
                    };
                    StateProgressThreshold {
                        max_non_epoch_no_progress_secs: max_no_progress_secs,
                        max_epoch_no_progress_secs: max_no_progress_secs,
                        max_non_epoch_round_gap: 60,
                        max_epoch_round_gap: 60,
                    }
                });

            // If errors are allowed, overwrite the success criteria
            if allow_errors {
                success_criteria.allow_errors()
            } else {
                success_criteria
            }
        })
}

fn quorum_store_reconfig_enable_test() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(20).unwrap())
        .with_initial_fullnode_count(20)
        .add_network_test(QuorumStoreOnChainEnableTest {})
        .with_success_criteria(
            SuccessCriteria::new(5000)
                .add_no_restarts()
                .add_wait_for_catchup_s(240)
                .add_system_metrics_threshold(SYSTEM_12_CORES_10GB_THRESHOLD.clone())
                .add_chain_progress(RELIABLE_PROGRESS_THRESHOLD.clone()),
        )
}

/// A simple test that runs the swarm forever. This is useful for
/// local testing (e.g., deploying a local swarm and interacting
/// with it).
#[derive(Debug)]
pub(crate) struct RunForever;

impl Test for RunForever {
    fn name(&self) -> &'static str {
        "run_forever"
    }
}

#[async_trait::async_trait]
impl AptosTest for RunForever {
    async fn run<'t>(&self, _ctx: &mut AptosContext<'t>) -> Result<()> {
        println!("The network has been deployed. Hit Ctrl+C to kill this, otherwise it will run forever.");
        let keep_running = Arc::new(AtomicBool::new(true));
        while keep_running.load(Ordering::Acquire) {
            thread::park();
        }
        Ok(())
    }
}

//TODO Make public test later
#[derive(Debug)]
pub(crate) struct GetMetadata;

impl Test for GetMetadata {
    fn name(&self) -> &'static str {
        "get_metadata"
    }
}

impl AdminTest for GetMetadata {
    fn run(&self, ctx: &mut AdminContext<'_>) -> Result<()> {
        let client = ctx.rest_client();
        let runtime = Runtime::new().unwrap();
        runtime.block_on(client.get_aptos_version()).unwrap();
        runtime.block_on(client.get_ledger_information()).unwrap();

        Ok(())
    }
}

pub async fn check_account_balance(
    client: &RestClient,
    account_address: AccountAddress,
    expected: u64,
) -> Result<()> {
    let balance = client
        .view_apt_account_balance(account_address)
        .await?
        .into_inner();
    assert_eq!(balance, expected);

    Ok(())
}

#[derive(Debug)]
pub(crate) struct FundAccount;

impl Test for FundAccount {
    fn name(&self) -> &'static str {
        "fund_account"
    }
}

#[async_trait::async_trait]
impl AptosTest for FundAccount {
    async fn run<'t>(&self, ctx: &mut AptosContext<'t>) -> Result<()> {
        let client = ctx.client();

        let account = ctx.random_account();
        let amount = 1000;
        ctx.create_user_account(account.public_key()).await?;
        ctx.mint(account.address(), amount).await?;
        check_account_balance(&client, account.address(), amount).await?;

        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct TransferCoins;

impl Test for TransferCoins {
    fn name(&self) -> &'static str {
        "transfer_coins"
    }
}

#[async_trait::async_trait]
impl AptosTest for TransferCoins {
    async fn run<'t>(&self, ctx: &mut AptosContext<'t>) -> Result<()> {
        let client = ctx.client();
        let payer = ctx.random_account();
        let payee = ctx.random_account();
        ctx.create_user_account(payer.public_key()).await?;
        ctx.create_user_account(payee.public_key()).await?;
        ctx.mint(payer.address(), 10000).await?;
        check_account_balance(&client, payer.address(), 10000).await?;

        let transfer_txn = payer.sign_with_transaction_builder(
            ctx.aptos_transaction_factory()
                .payload(aptos_stdlib::aptos_coin_transfer(payee.address(), 10)),
        );
        client.submit_and_wait(&transfer_txn).await?;
        check_account_balance(&client, payee.address(), 10).await?;

        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct RestartValidator;

impl Test for RestartValidator {
    fn name(&self) -> &'static str {
        "restart_validator"
    }
}

#[async_trait]
impl NetworkTest for RestartValidator {
    async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> Result<()> {
        let mut ctx_locker = ctx.ctx.lock().await;
        let ctx = ctx_locker.deref_mut();
        let swarm = ctx.swarm.read().await;
        let node = swarm.validators().next().unwrap();
        node.health_check().await.expect("node health check failed");
        node.stop().await.unwrap();
        println!("Restarting node {}", node.peer_id());
        node.start().await.unwrap();
        tokio::time::sleep(Duration::from_secs(1)).await;
        node.health_check().await.expect("node health check failed");
        Ok(())
    }
}

pub(crate) fn setup_test() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(1).unwrap())
        .with_initial_fullnode_count(1)
        .add_network_test(ForgeSetupTest)
}

pub(crate) fn reconfiguration_test() -> ForgeConfig {
    ForgeConfig::default().add_network_test(ReconfigurationTest)
}

#[derive(Debug)]
pub(crate) struct EmitTransaction;

impl Test for EmitTransaction {
    fn name(&self) -> &'static str {
        "emit_transaction"
    }
}

#[async_trait]
impl NetworkTest for EmitTransaction {
    async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> Result<()> {
        let mut ctx_locker = ctx.ctx.lock().await;
        let ctx = ctx_locker.deref_mut();
        let duration = Duration::from_secs(10);
        let all_validators = ctx
            .swarm
            .read()
            .await
            .validators()
            .map(|v| v.peer_id())
            .collect::<Vec<_>>();
        let stats = generate_traffic(ctx, &all_validators, duration)
            .await
            .unwrap();
        ctx.report.report_txn_stats(self.name().to_string(), &stats);
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct Delay {
    pub(crate) seconds: u64,
}

impl Delay {
    pub(crate) fn new(seconds: u64) -> Self {
        Self { seconds }
    }
}

impl Test for Delay {
    fn name(&self) -> &'static str {
        "delay"
    }
}

#[async_trait]
impl NetworkTest for Delay {
    async fn run<'a>(&self, _ctx: NetworkContextSynchronizer<'a>) -> Result<()> {
        info!("forge sleep {}", self.seconds);
        tokio::time::sleep(Duration::from_secs(self.seconds)).await;
        Ok(())
    }
}

pub(crate) fn gather_metrics() -> ForgeConfig {
    ForgeConfig::default()
        .add_network_test(GatherMetrics)
        .add_network_test(Delay::new(180))
        .add_network_test(GatherMetrics)
}

#[derive(Debug)]
pub(crate) struct GatherMetrics;

impl Test for GatherMetrics {
    fn name(&self) -> &'static str {
        "gather_metrics"
    }
}

#[async_trait]
impl NetworkTest for GatherMetrics {
    async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> Result<()> {
        let mut ctx_locker = ctx.ctx.lock().await;
        let ctx = ctx_locker.deref_mut();
        gather_metrics_one(ctx).await;
        Ok(())
    }
}

async fn gather_metrics_one(ctx: &NetworkContext<'_>) {
    let handle = ctx.runtime.handle();
    let outdir = Path::new("/tmp");
    let mut gets = FuturesUnordered::new();
    let now = chrono::prelude::Utc::now()
        .format("%Y%m%d_%H%M%S")
        .to_string();
    {
        let swarm = ctx.swarm.read().await;
        for val in swarm.validators() {
            let mut url = val.inspection_service_endpoint();
            let valname = val.peer_id().to_string();
            url.set_path("metrics");
            let fname = format!("{}.{}.metrics", now, valname);
            let outpath: PathBuf = outdir.join(fname);
            let th = handle.spawn(gather_metrics_to_file(url, outpath));
            gets.push(th);
        }
    }
    // join all the join handles
    while !gets.is_empty() {
        select! {
            _ = gets.next() => {}
        }
    }
}

pub(crate) async fn gather_metrics_to_file(url: Url, outpath: PathBuf) {
    let client = reqwest::Client::new();
    match client.get(url).send().await {
        Ok(response) => {
            let url = response.url().clone();
            let status = response.status();
            if status.is_success() {
                match response.text().await {
                    Ok(text) => match std::fs::write(outpath, text) {
                        Ok(_) => {},
                        Err(err) => {
                            info!("could not write metrics: {}", err);
                        },
                    },
                    Err(err) => {
                        info!("bad metrics GET: {} -> {}", url, err);
                    },
                }
            } else {
                info!("bad metrics GET: {} -> {}", url, status);
            }
        },
        Err(err) => {
            info!("bad metrics GET: {}", err);
        },
    }
}
