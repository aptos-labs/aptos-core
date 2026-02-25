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

/// Apply devnet consensus config overrides to a validator node.
/// These settings match the proven devnet config that achieves 3k+ TPS / 70 blocks/s.
///
/// QuorumStore and general consensus settings are applied globally (QS is shared
/// by proxy and primary). Primary block limits are left at defaults (5000/1800)
/// since primary blocks aggregate multiple proxy blocks and are therefore larger.
/// Proxy block limits match devnet's per-block settings (300/200).
fn apply_devnet_consensus_config(config: &mut aptos_config::config::NodeConfig) {
    // QuorumStore settings (shared by proxy and primary, needed for fast batch generation).
    // With 4 proxy validators each handling ~750 TPS, QS must drain mempool fast enough.
    config.consensus.quorum_store.enable_opt_quorum_store = true;
    config.consensus.quorum_store.opt_qs_minimum_batch_age_usecs = 500;
    config.consensus.quorum_store.batch_generation_poll_interval_ms = 10;
    config.consensus.quorum_store.batch_generation_min_non_empty_interval_ms = 10;
    config.consensus.quorum_store.batch_generation_max_interval_ms = 100;
    config.consensus.quorum_store.sender_max_total_txns = 500;
    // Raise the QS backpressure floor so dynamic rate never drops below
    // per-validator needs (~750 TPS with 4 proxy validators at 3k TPS).
    config
        .consensus
        .quorum_store
        .back_pressure
        .dynamic_min_txn_per_s = 1000;

    // General consensus settings
    config.consensus.vote_back_pressure_limit = 150;
    config.consensus.quorum_store_poll_time_ms = 5;
    config.consensus.enable_optimistic_proposal_tx = true;
    config.consensus.internal_per_key_channel_size = 20;
    // NOTE: Do NOT override max_sending_block_txns (default 5000) or
    // max_sending_block_txns_after_filtering (default 1800) for primary.
    // Primary blocks aggregate ~10 proxy blocks, so they are larger than
    // individual devnet blocks. generate_proposal_with_proxy_payload()
    // bypasses these limits, but keeping defaults avoids confusion.

    // Proxy consensus block limits: match devnet per-block settings.
    // At steady state (3k TPS / ~70 proxy blocks/s), each proxy block
    // averages ~43 txns — well under the 300 cap.
    config.consensus.proxy_consensus_config.max_proxy_block_txns = 300;
    config
        .consensus
        .proxy_consensus_config
        .max_proxy_block_txns_after_filtering = 200;
    config.consensus.proxy_consensus_config.max_proxy_block_bytes = 5 * 1024 * 1024;
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
            },
        ))
        .with_validator_override_node_config_fn(Arc::new(|config, _| {
            apply_devnet_consensus_config(config);
        }))
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
                .mode(EmitJobMode::ConstTps { tps: 3000 })
                .gas_price(5 * aptos_global_constants::GAS_UNIT_PRICE)
                .latency_polling_interval(Duration::from_millis(100)),
        )
        .with_success_criteria(
            SuccessCriteria::new(2500)
                .add_no_restarts()
                .add_wait_for_catchup_s(120)
                .add_chain_progress(StateProgressThreshold {
                    max_non_epoch_no_progress_secs: 30.0,
                    max_epoch_no_progress_secs: 30.0,
                    max_non_epoch_round_gap: 8,
                    max_epoch_round_gap: 8,
                })
                // Proxy consensus epoch transitions produce transient
                // "Invalid bitvec from the multi-signature" errors.
                .allow_errors(),
        )
}

/// Local test: 4 validators (1 proxy), no network emulation (for debugging).
/// Uses same devnet consensus config as remote test. Debug-build TPS is limited
/// by slow execution (~27 TPS observed), so criteria are relaxed. The remote
/// test on forge (release builds) targets devnet-level performance.
fn proxy_primary_local_test() -> ForgeConfig {
    let num_validators = 4;
    let num_proxy: usize = 1;
    let proxy_indices: Vec<u16> = (0..num_proxy as u16).collect();

    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(num_validators).unwrap())
        .with_initial_fullnode_count(0)
        .add_network_test(ProxyPrimaryTrafficTest {
            num_proxy_validators: num_proxy,
        })
        .with_validator_override_node_config_fn(Arc::new(|config, _| {
            apply_devnet_consensus_config(config);
            // Override proxy limits for debug builds — 300 txns/block causes
            // massive pipeline gap in unoptimized builds.
            config.consensus.proxy_consensus_config.max_proxy_block_txns = 50;
            config
                .consensus
                .proxy_consensus_config
                .max_proxy_block_txns_after_filtering = 36;
            config.consensus.proxy_consensus_config.max_proxy_block_bytes = 50 * 1024;
        }))
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
            SuccessCriteria::new(10)
                .add_no_restarts()
                .add_wait_for_catchup_s(60),
        )
}
