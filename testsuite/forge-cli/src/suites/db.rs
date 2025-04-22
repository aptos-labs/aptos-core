// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::ungrouped::PROGRESS_THRESHOLD_20_6;
use aptos_forge::{
    args::TransactionTypeArg, success_criteria::SuccessCriteria, EmitJobMode, EmitJobRequest,
    ForgeConfig, ReplayProtectionType,
};
use aptos_testcases::performance_test::PerformanceBenchmark;
use std::{num::NonZeroUsize, path::PathBuf, sync::Arc};

pub(crate) fn large_db_simple_test() -> ForgeConfig {
    large_db_test(10, 500, 300, "10-validators".to_string())
}

pub(crate) fn large_db_test(
    num_validators: usize,
    target_tps: usize,
    min_avg_tps: usize,
    existing_db_tag: String,
) -> ForgeConfig {
    let config = ForgeConfig::default();
    config
        .with_initial_validator_count(NonZeroUsize::new(num_validators).unwrap())
        .with_initial_fullnode_count(std::cmp::max(2, target_tps / 1000))
        .add_network_test(PerformanceBenchmark)
        .with_existing_db(existing_db_tag.clone())
        .with_validator_override_node_config_fn(Arc::new(move |config, _| {
            config.base.working_dir = Some(PathBuf::from("/opt/aptos/data/checkpoint"));
        }))
        .with_fullnode_override_node_config_fn(Arc::new(move |config, _| {
            config.base.working_dir = Some(PathBuf::from("/opt/aptos/data/checkpoint"));
        }))
        .with_emit_job(
            EmitJobRequest::default()
                .mode(EmitJobMode::ConstTps { tps: target_tps })
                .transaction_mix(vec![
                    (
                        TransactionTypeArg::CoinTransfer.materialize_default(),
                        ReplayProtectionType::SequenceNumber,
                        75,
                    ),
                    (
                        TransactionTypeArg::AccountGeneration.materialize_default(),
                        ReplayProtectionType::SequenceNumber,
                        20,
                    ),
                    (
                        TransactionTypeArg::TokenV1NFTMintAndTransferSequential
                            .materialize_default(),
                        ReplayProtectionType::SequenceNumber,
                        5,
                    ),
                ]),
        )
        .with_success_criteria(
            SuccessCriteria::new(min_avg_tps)
                .add_no_restarts()
                .add_wait_for_catchup_s(30)
                .add_chain_progress(PROGRESS_THRESHOLD_20_6.clone()),
        )
}
