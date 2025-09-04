// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::ungrouped::changing_working_quorum_test_helper;
use velor_config::config::{BootstrappingMode, ContinuousSyncingMode, StateSyncConfig};
use velor_forge::{
    args::TransactionTypeArg, success_criteria::SuccessCriteria, EmitJobMode, EmitJobRequest,
    ForgeConfig,
};
use velor_testcases::{
    consensus_reliability_tests::ChangingWorkingQuorumTest,
    state_sync_performance::{
        StateSyncFullnodeFastSyncPerformance, StateSyncFullnodePerformance,
        StateSyncValidatorPerformance,
    },
};
use std::{num::NonZeroUsize, sync::Arc};

/// Attempts to match the test name to a state sync test
pub fn get_state_sync_test(test_name: &str) -> Option<ForgeConfig> {
    let test = match test_name {
        "state_sync_perf_fullnodes_apply_outputs" => state_sync_perf_fullnodes_apply_outputs(),
        "state_sync_perf_fullnodes_execute_transactions" => {
            state_sync_perf_fullnodes_execute_transactions()
        },
        "state_sync_perf_fullnodes_fast_sync" => state_sync_perf_fullnodes_fast_sync(),
        "state_sync_perf_validators" => state_sync_perf_validators(),
        "state_sync_failures_catching_up" => state_sync_failures_catching_up(),
        "state_sync_slow_processing_catching_up" => state_sync_slow_processing_catching_up(),

        _ => return None, // The test name does not match a state sync test
    };
    Some(test)
}

pub fn state_sync_config_execute_transactions(state_sync_config: &mut StateSyncConfig) {
    state_sync_config.state_sync_driver.bootstrapping_mode =
        BootstrappingMode::ExecuteTransactionsFromGenesis;
    state_sync_config.state_sync_driver.continuous_syncing_mode =
        ContinuousSyncingMode::ExecuteTransactions;
}

pub fn state_sync_config_apply_transaction_outputs(state_sync_config: &mut StateSyncConfig) {
    state_sync_config.state_sync_driver.bootstrapping_mode =
        BootstrappingMode::ApplyTransactionOutputsFromGenesis;
    state_sync_config.state_sync_driver.continuous_syncing_mode =
        ContinuousSyncingMode::ApplyTransactionOutputs;
}

pub fn state_sync_config_fast_sync(state_sync_config: &mut StateSyncConfig) {
    state_sync_config.state_sync_driver.bootstrapping_mode =
        BootstrappingMode::DownloadLatestStates;
    state_sync_config.state_sync_driver.continuous_syncing_mode =
        ContinuousSyncingMode::ApplyTransactionOutputs;
}

/// A default config for running various state sync performance tests
pub fn state_sync_perf_fullnodes_config() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(4).unwrap())
        .with_initial_fullnode_count(4)
}

/// The config for running a state sync performance test when applying
/// transaction outputs in fullnodes.
fn state_sync_perf_fullnodes_apply_outputs() -> ForgeConfig {
    state_sync_perf_fullnodes_config()
        .add_network_test(StateSyncFullnodePerformance)
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = 600.into();
        }))
        .with_fullnode_override_node_config_fn(Arc::new(|config, _| {
            state_sync_config_apply_transaction_outputs(&mut config.state_sync);
        }))
        .with_success_criteria(SuccessCriteria::new(9000))
}

/// The config for running a state sync performance test when executing
/// transactions in fullnodes.
fn state_sync_perf_fullnodes_execute_transactions() -> ForgeConfig {
    state_sync_perf_fullnodes_config()
        .add_network_test(StateSyncFullnodePerformance)
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = 600.into();
        }))
        .with_fullnode_override_node_config_fn(Arc::new(|config, _| {
            state_sync_config_execute_transactions(&mut config.state_sync);
        }))
        .with_success_criteria(SuccessCriteria::new(5000))
}

/// The config for running a state sync performance test when fast syncing
/// to the latest epoch.
fn state_sync_perf_fullnodes_fast_sync() -> ForgeConfig {
    state_sync_perf_fullnodes_config()
        .add_network_test(StateSyncFullnodeFastSyncPerformance)
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = 180.into(); // Frequent epochs
        }))
        .with_emit_job(
            EmitJobRequest::default()
                .mode(EmitJobMode::MaxLoad {
                    mempool_backlog: 30000,
                })
                .transaction_type(TransactionTypeArg::AccountGeneration.materialize_default()), // Create many state values
        )
        .with_fullnode_override_node_config_fn(Arc::new(|config, _| {
            state_sync_config_fast_sync(&mut config.state_sync);
        }))
}

/// The config for running a state sync performance test when applying
/// transaction outputs in failed validators.
fn state_sync_perf_validators() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(7).unwrap())
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = 600.into();
        }))
        .with_validator_override_node_config_fn(Arc::new(|config, _| {
            state_sync_config_apply_transaction_outputs(&mut config.state_sync);
        }))
        .add_network_test(StateSyncValidatorPerformance)
        .with_success_criteria(SuccessCriteria::new(5000))
}

fn state_sync_failures_catching_up() -> ForgeConfig {
    changing_working_quorum_test_helper(
        7,
        300,
        3000,
        2500,
        true,
        false,
        false,
        ChangingWorkingQuorumTest {
            min_tps: 1500,
            always_healthy_nodes: 2,
            max_down_nodes: 1,
            num_large_validators: 2,
            add_execution_delay: false,
            check_period_s: 27,
        },
    )
}

fn state_sync_slow_processing_catching_up() -> ForgeConfig {
    changing_working_quorum_test_helper(
        7,
        300,
        3000,
        2500,
        true,
        true,
        false,
        ChangingWorkingQuorumTest {
            min_tps: 750,
            always_healthy_nodes: 2,
            max_down_nodes: 0,
            num_large_validators: 2,
            add_execution_delay: true,
            check_period_s: 57,
        },
    )
}
