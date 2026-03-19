// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::ungrouped::mixed_compatible_emit_job;
use crate::{suites::realistic_environment::realistic_env_max_load_test, TestCommand};
use aptos_forge::{success_criteria::SuccessCriteria, ForgeConfig};
use aptos_sdk::types::on_chain_config::{BlockGasLimitType, OnChainExecutionConfig};
use aptos_testcases::{
    compatibility_test::SimpleValidatorUpgrade, framework_upgrade::FrameworkUpgrade,
    transaction_tracing_test::TransactionTracingTest,
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
        "transaction_tracing_test" => transaction_tracing_test(),
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

pub(crate) fn transaction_tracing_test() -> ForgeConfig {
    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(4).unwrap())
        .add_network_test(TransactionTracingTest)
        .with_genesis_helm_config_fn(Arc::new(|helm_values| {
            // Set a very small block gas limit to force blocks to be cut early,
            // causing transactions to be retried. This exercises the Executed(Retry)
            // → mark_retry() path in transaction tracing.
            let mut on_chain_execution_config = OnChainExecutionConfig::default_for_genesis();
            match &mut on_chain_execution_config {
                OnChainExecutionConfig::Missing
                | OnChainExecutionConfig::V1(_)
                | OnChainExecutionConfig::V2(_)
                | OnChainExecutionConfig::V3(_) => {
                    unreachable!("Unexpected on-chain execution config type")
                },
                OnChainExecutionConfig::V4(config_v4) => {
                    set_small_block_gas_limit(&mut config_v4.block_gas_limit_type);
                },
                OnChainExecutionConfig::V5(config_v5) => {
                    set_small_block_gas_limit(&mut config_v5.block_gas_limit_type);
                },
                OnChainExecutionConfig::V6(config_v6) => {
                    set_small_block_gas_limit(&mut config_v6.block_gas_limit_type);
                },
                OnChainExecutionConfig::V7(config_v7) => {
                    set_small_block_gas_limit(&mut config_v7.block_gas_limit_type);
                },
            }
            helm_values["chain"]["on_chain_execution_config"] =
                serde_yaml::to_value(on_chain_execution_config).expect("must serialize");
        }))
}

/// Reduce effective_block_gas_limit to trigger some transaction retries while
/// still allowing reasonable TPS. The default is 200000; we lower it to 50000
/// (25% of default) so blocks are cut more frequently, exercising the retry path.
fn set_small_block_gas_limit(block_gas_limit_type: &mut BlockGasLimitType) {
    match block_gas_limit_type {
        BlockGasLimitType::ComplexLimitV1 {
            effective_block_gas_limit,
            ..
        } => {
            *effective_block_gas_limit = 50000;
        },
        _ => {
            *block_gas_limit_type = BlockGasLimitType::Limit(50000);
        },
    }
}
