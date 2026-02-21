// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Forge test suites for Proxy Primary Consensus.

use aptos_forge::{
    success_criteria::{StateProgressThreshold, SuccessCriteria},
    EmitJobMode, EmitJobRequest, ForgeConfig,
};
use aptos_sdk::types::on_chain_config::{
    ConsensusAlgorithmConfig, OnChainConsensusConfig, OnChainExecutionConfig, ValidatorTxnConfig,
    DEFAULT_WINDOW_SIZE,
};
use aptos_testcases::{
    proxy_primary_test::{ProxyPrimaryNetworkEmulation, ProxyPrimaryTrafficTest},
    CompositeNetworkTest,
};
use std::{num::NonZeroUsize, sync::Arc, time::Duration};

/// Get a proxy consensus test by name.
pub fn get_proxy_test(test_name: &str) -> Option<ForgeConfig> {
    let test = match test_name {
        "proxy_primary_test" => proxy_primary_remote_test(),
        "proxy_primary_local_test" => proxy_primary_local_test(),
        _ => return None,
    };
    Some(test)
}

/// Remote test: 7 validators (4 proxy + 3 primary-only), multi-region network emulation.
///
/// Network topology:
/// - 4 proxy validators co-located in eu-west2 (~5ms intra-region)
/// - 3 non-proxy validators geo-distributed (us-east4, as-northeast1, as-southeast1)
/// - All traffic submitted to proxy validators only
fn proxy_primary_remote_test() -> ForgeConfig {
    let num_validators = 7;
    let num_proxy: usize = 4;
    let proxy_indices: Vec<u16> = (0..num_proxy as u16).collect();

    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(num_validators).unwrap())
        .with_initial_fullnode_count(0)
        .add_network_test(CompositeNetworkTest::new(
            ProxyPrimaryNetworkEmulation::new(num_proxy),
            ProxyPrimaryTrafficTest {
                num_proxy_validators: num_proxy,
                inner_traffic: EmitJobRequest::default()
                    .mode(EmitJobMode::MaxLoad {
                        mempool_backlog: 20000,
                    })
                    .init_gas_price_multiplier(20),
                inner_success_criteria: SuccessCriteria::new(500),
            },
        ))
        .with_genesis_helm_config_fn(Arc::new(move |helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = 300.into();
            helm_values["chain"]["on_chain_consensus_config"] =
                serde_yaml::to_value(OnChainConsensusConfig::V6 {
                    alg: ConsensusAlgorithmConfig::default_for_genesis(),
                    vtxn: ValidatorTxnConfig::default_for_genesis(),
                    window_size: DEFAULT_WINDOW_SIZE,
                    rand_check_enabled: true,
                    proxy_validator_indices: proxy_indices.clone(),
                })
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
        .with_success_criteria(
            SuccessCriteria::new(50)
                .add_no_restarts()
                .add_wait_for_catchup_s(120)
                .add_chain_progress(StateProgressThreshold {
                    max_non_epoch_no_progress_secs: 30.0,
                    max_epoch_no_progress_secs: 30.0,
                    max_non_epoch_round_gap: 8,
                    max_epoch_round_gap: 8,
                }),
        )
}

/// Local test: 4 validators (1 proxy), no network emulation (for debugging).
fn proxy_primary_local_test() -> ForgeConfig {
    let num_validators = 4;
    let num_proxy: usize = 1;
    let proxy_indices: Vec<u16> = (0..num_proxy as u16).collect();

    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(num_validators).unwrap())
        .with_initial_fullnode_count(0)
        .add_network_test(ProxyPrimaryTrafficTest {
            num_proxy_validators: num_proxy,
            inner_traffic: EmitJobRequest::default()
                .mode(EmitJobMode::MaxLoad {
                    mempool_backlog: 5000,
                })
                .init_gas_price_multiplier(20),
            inner_success_criteria: SuccessCriteria::new(100),
        })
        .with_genesis_helm_config_fn(Arc::new(move |helm_values| {
            helm_values["chain"]["epoch_duration_secs"] = 300.into();
            helm_values["chain"]["on_chain_consensus_config"] =
                serde_yaml::to_value(OnChainConsensusConfig::V6 {
                    alg: ConsensusAlgorithmConfig::default_for_genesis(),
                    vtxn: ValidatorTxnConfig::default_for_genesis(),
                    window_size: DEFAULT_WINDOW_SIZE,
                    rand_check_enabled: true,
                    proxy_validator_indices: proxy_indices.clone(),
                })
                .expect("must serialize");
            helm_values["chain"]["on_chain_execution_config"] =
                serde_yaml::to_value(OnChainExecutionConfig::default_for_genesis())
                    .expect("must serialize");
        }))
        .with_emit_job(
            EmitJobRequest::default()
                .mode(EmitJobMode::ConstTps { tps: 100 })
                .gas_price(5 * aptos_global_constants::GAS_UNIT_PRICE),
        )
        .with_success_criteria(
            SuccessCriteria::new(50)
                .add_no_restarts()
                .add_wait_for_catchup_s(60),
        )
}
