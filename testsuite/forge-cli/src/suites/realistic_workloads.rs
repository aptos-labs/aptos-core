// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::ungrouped::{
    mixed_emit_job, PROGRESS_THRESHOLD_20_6, RELIABLE_REAL_ENV_PROGRESS_THRESHOLD,
};
use aptos_forge::{
    args::TransactionTypeArg, success_criteria::SuccessCriteria, EmitJobMode, EmitJobRequest,
    EntryPoints, ForgeConfig, TransactionType, WorkflowProgress,
};
use aptos_testcases::{
    load_vs_perf_benchmark::{LoadVsPerfBenchmark, TransactionWorkload, Workloads},
    modifiers::CpuChaosTest,
    multi_region_network_test::MultiRegionNetworkEmulationTest,
    performance_test::PerformanceBenchmark,
    CompositeNetworkTest,
};
use std::{collections::HashMap, num::NonZeroUsize, sync::Arc};

pub(crate) fn individual_workload_tests(test_name: String) -> ForgeConfig {
    let job = EmitJobRequest::default().mode(EmitJobMode::MaxLoad {
        mempool_backlog: 30000,
    });
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(5).unwrap())
        .with_initial_fullnode_count(3)
        .add_network_test(PerformanceBenchmark)
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = 600.into();
        }))
        .with_validator_override_node_config_fn(Arc::new(|config, _| {
            config.execution.processed_transactions_detailed_counters = true;
        }))
        .with_emit_job(
            if test_name == "write_new_resource" {
                let account_creation_type = TransactionType::AccountGeneration {
                    add_created_accounts_to_pool: true,
                    max_account_working_set: 20_000_000,
                    creation_balance: 200_000_000,
                };
                let write_type = TransactionType::CallCustomModules {
                    entry_point: Box::new(EntryPoints::BytesMakeOrChange {
                        data_length: Some(32),
                    }),
                    num_modules: 1,
                    use_account_pool: true,
                };
                job.transaction_mix_per_phase(vec![
                    // warmup
                    vec![(account_creation_type.clone(), 1)],
                    vec![(account_creation_type, 1)],
                    vec![(write_type.clone(), 1)],
                    // cooldown
                    vec![(write_type, 1)],
                ])
            } else {
                job.transaction_type(match test_name.as_str() {
                    "account_creation" => {
                        TransactionTypeArg::AccountGeneration.materialize_default()
                    },
                    "publishing" => TransactionTypeArg::PublishPackage.materialize_default(),
                    "module_loading" => TransactionTypeArg::NoOp.materialize(
                        1000,
                        false,
                        WorkflowProgress::when_done_default(),
                        &HashMap::new(),
                    ),
                    _ => unreachable!("{}", test_name),
                })
            },
        )
        .with_success_criteria(
            SuccessCriteria::new(match test_name.as_str() {
                "account_creation" => 3600,
                "publishing" => 60,
                "write_new_resource" => 3700,
                "module_loading" => 1800,
                _ => unreachable!("{}", test_name),
            })
            .add_no_restarts()
            .add_wait_for_catchup_s(240)
            .add_chain_progress(PROGRESS_THRESHOLD_20_6.clone()),
        )
}

pub(crate) fn workload_vs_perf_benchmark() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(7).unwrap())
        .with_initial_fullnode_count(7)
        .with_validator_override_node_config_fn(Arc::new(|config, _| {
            config.execution.processed_transactions_detailed_counters = true;
        }))
        .add_network_test(LoadVsPerfBenchmark {
            test: Box::new(PerformanceBenchmark),
            workloads: Workloads::TRANSACTIONS(vec![
                TransactionWorkload::new(TransactionTypeArg::NoOp, 20000),
                TransactionWorkload::new(TransactionTypeArg::NoOp, 20000).with_unique_senders(),
                TransactionWorkload::new(TransactionTypeArg::NoOp, 20000).with_num_modules(1000),
                TransactionWorkload::new(TransactionTypeArg::CoinTransfer, 20000)
                    .with_unique_senders(),
                TransactionWorkload::new(TransactionTypeArg::CoinTransfer, 20000)
                    .with_unique_senders(),
                TransactionWorkload::new(TransactionTypeArg::AccountResource32B, 20000)
                    .with_unique_senders(),
                TransactionWorkload::new(TransactionTypeArg::AccountResource1KB, 20000)
                    .with_unique_senders(),
                TransactionWorkload::new(TransactionTypeArg::PublishPackage, 20000)
                    .with_unique_senders(),
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

pub(crate) fn mainnet_like_simulation_test() -> ForgeConfig {
    let num_validators = 20;
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(num_validators).unwrap())
        .with_emit_job(
            EmitJobRequest::default()
                .mode(EmitJobMode::MaxLoad {
                    mempool_backlog: 200_000,
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
        // TODO(ibalajiarun): tune these success critiera after we have a better idea of the test behavior
        .with_success_criteria(
            SuccessCriteria::new(10000)
                .add_no_restarts()
                .add_wait_for_catchup_s(240)
                .add_chain_progress(PROGRESS_THRESHOLD_20_6.clone()),
        )
}

pub(crate) fn workload_mix_test() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(5).unwrap())
        .with_initial_fullnode_count(3)
        .add_network_test(PerformanceBenchmark)
        .with_validator_override_node_config_fn(Arc::new(|config, _| {
            config.execution.processed_transactions_detailed_counters = true;
        }))
        .with_emit_job(mixed_emit_job())
        .with_success_criteria(
            SuccessCriteria::new(3000)
                .add_no_restarts()
                .add_wait_for_catchup_s(240)
                .add_chain_progress(PROGRESS_THRESHOLD_20_6.clone()),
        )
}
