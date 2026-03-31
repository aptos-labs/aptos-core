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
    // Larger batches reduce proof count: at 15K TPS, 250 txns/batch = 60 proofs/sec
    // vs 50 txns/batch = 300 proofs/sec. Keeps proof backlog well under the 140 limit.
    config.consensus.quorum_store.sender_max_batch_txns = 250;
    config.consensus.quorum_store.receiver_max_batch_txns = 250;
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
    // Primary blocks aggregate many proxy blocks, so they can be much larger
    // than individual blocks. Raise the voter-side receiving limit to prevent
    // rejection of aggregated proxy payloads during timeout recovery.
    // generate_proposal_with_proxy_payload() bypasses sending limits.
    config.consensus.max_receiving_block_txns = 50000;
    config.consensus.max_receiving_block_bytes = 30 * 1024 * 1024; // 30MB

    // Proxy consensus block limits: larger blocks compensate for timeout
    // overhead. At 10k TPS with 4 proxy validators, each validator handles
    // ~2500 TPS. With timeouts eating ~30% of rounds, effective block rate
    // is ~30-50 blocks/s. At 500 txns/block = 15-25k TPS throughput,
    // giving comfortable headroom above 10k.
    config.consensus.proxy_consensus_config.max_proxy_block_txns = 500;
    config
        .consensus
        .proxy_consensus_config
        .max_proxy_block_txns_after_filtering = 350;
    config.consensus.proxy_consensus_config.max_proxy_block_bytes = 10 * 1024 * 1024;

    // Budget: allow up to 30 non-empty proxy blocks per primary consumption window
    // (up from default 10). With primary at ~5 blocks/s and proxy at ~50+ rounds/s,
    // each 200ms primary window produces ~10+ proxy rounds. budget=30 ensures all
    // can carry transactions. 30 × 500 txns = 15000 txns/window, well within the
    // 50000 max_receiving_block_txns for primary blocks.
    config
        .consensus
        .proxy_consensus_config
        .target_proxy_blocks_per_primary_round = 30;
    // Raise backpressure thresholds to match the higher buffer target, avoiding
    // spurious throttling when pending_proxy_batches is transiently near 30.
    config
        .consensus
        .proxy_consensus_config
        .backpressure
        .batch_moderate_threshold = 50;
    config
        .consensus
        .proxy_consensus_config
        .backpressure
        .batch_heavy_threshold = 100;
    // Pull validator txns less frequently to avoid slow rounds. The vtxn pool
    // query can take ~25ms when pool is empty; at 100ms timeout that's 25% of
    // the budget. Pull every 50th block instead of every 10th.
    config.consensus.proxy_consensus_config.vtxn_pull_interval = 50;

    // Proxy timeout tuning: with opt proposals enabled + pending_ordering=true,
    // healthy rounds complete in ~10-20ms (5ms intra-region RTT). Use 100ms
    // timeout (the default) so timeouts waste only 100ms instead of 300ms.
    // At 300ms, each timeout wastes ~30x a healthy round. At 100ms, only ~10x.
    // Max exponent 4 caps escalation at 100 × 1.2^4 = 207ms.
    config
        .consensus
        .proxy_consensus_config
        .round_initial_timeout_ms = 100;
    config
        .consensus
        .proxy_consensus_config
        .round_timeout_backoff_max_exponent = 4;

    // Disable mempool failover broadcast to prevent cross-proxy txn sharing.
    // With QS enabled, each proxy validator should only batch transactions
    // submitted directly to it. Failover gossip causes the same txn to appear
    // in different QS batches from different validators → duplicate execution.
    config.mempool.default_failovers = 0;
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
            // Long epoch to avoid epoch transitions during the test.
            helm_values["chain"]["epoch_duration_secs"] = 7200.into();
            helm_values["chain"]["on_chain_consensus_config"] =
                serde_yaml::to_value(OnChainConsensusConfig::V6 {
                    alg: ConsensusAlgorithmConfig::default_for_genesis(),
                    vtxn: ValidatorTxnConfig::default_for_genesis(),
                    window_size: DEFAULT_WINDOW_SIZE,
                    rand_check_enabled: false, // Disable randomness check for proxy-primary
                    proxy_validator_indices: proxy_indices.clone(),
                })
                .expect("must serialize");
            helm_values["chain"]["on_chain_execution_config"] =
                serde_yaml::to_value(OnChainExecutionConfig::default_for_genesis())
                    .expect("must serialize");
        }))
        .with_emit_job(
            EmitJobRequest::default()
                .mode(EmitJobMode::ConstTps { tps: 15000 })
                .gas_price(5 * aptos_global_constants::GAS_UNIT_PRICE)
                .latency_polling_interval(Duration::from_millis(100)),
        )
        .with_success_criteria(
            SuccessCriteria::new(3000)
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
                    rand_check_enabled: false, // Disable randomness check for proxy-primary
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
