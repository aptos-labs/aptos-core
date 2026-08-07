// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Forge tests that exercise the encrypted-transaction mempool / decryption pipeline.

use super::realistic_environment::wrap_with_realistic_env;
use aptos_forge::{
    args::TransactionTypeArg,
    success_criteria::{LatencyType, StateProgressThreshold, SuccessCriteria},
    EmitJobMode, EmitJobRequest, ForgeConfig,
};
use aptos_sdk::types::on_chain_config::{
    FeatureFlag, Features, OnChainChunkyDKGConfig, OnChainConsensusConfig, OnChainExecutionConfig,
    OnChainRandomnessConfig,
};
use aptos_testcases::{
    encrypted_mainnet_test::EncryptedMainnetTest, two_traffics_test::TwoTrafficsTest,
};
use std::{num::NonZeroUsize, sync::Arc, time::Duration};

/// Pinned commit of `aptos-labs/aptos-networks` for the devnet trusted setup blobs used by the
/// load tests that don't need to assert ceiling behavior.
const DEVNET_TRUSTED_SETUP_COMMIT: &str = "8cfc400bc1e42a232b5b36cde779a5b71d4d275b";

/// Pinned commit of `aptos-labs/aptos-networks` for the mainnet trusted setup blobs. The mainnet
/// digest key is provisioned for a fixed number of rounds; the mainnet test asserts that ceiling.
const MAINNET_TRUSTED_SETUP_COMMIT: &str = "a041dcec2c9f8522e08ee29d3100f2519e8d8d43";

/// Attempts to match the test name to an encrypted-mempool test.
pub(crate) fn get_encrypted_mempool_test(
    test_name: &str,
    duration: Duration,
) -> Option<ForgeConfig> {
    let test = match test_name {
        "realistic_env_max_load_encrypted" => realistic_env_max_load_encrypted_test(duration),
        "realistic_env_max_load_encrypted_mix" => {
            realistic_env_max_load_encrypted_mix_test(duration)
        },
        "realistic_env_max_load_encrypted_mainnet" => {
            realistic_env_max_load_encrypted_mainnet_test(duration)
        },
        _ => return None,
    };
    Some(test)
}

pub(crate) fn realistic_env_max_load_encrypted_test(duration: Duration) -> ForgeConfig {
    let num_validators = 5;
    let num_fullnodes = 1;
    let num_pfns = 3;
    let mempool_backlog = 1600;

    let success_criteria = SuccessCriteria::new(15)
        .add_no_restarts()
        .add_wait_for_catchup_s((duration.as_secs() / 10).max(60))
        .add_latency_threshold(5.0, LatencyType::P50)
        .add_latency_threshold(7.0, LatencyType::P70)
        .add_chain_progress(StateProgressThreshold {
            max_non_epoch_no_progress_secs: 20.0,
            max_epoch_no_progress_secs: 20.0,
            max_non_epoch_round_gap: 6,
            max_epoch_round_gap: 6,
        });

    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(num_validators).unwrap())
        .with_initial_fullnode_count(num_fullnodes)
        .with_num_pfns(num_pfns)
        .add_network_test(wrap_with_realistic_env(num_validators, TwoTrafficsTest {
            inner_traffic: EmitJobRequest::default()
                .mode(EmitJobMode::MaxLoad { mempool_backlog })
                .init_gas_price_multiplier(20)
                .encrypt_transactions(true),
            inner_success_criteria: SuccessCriteria::new(300),
        }))
        .with_genesis_helm_config_fn(Arc::new(default_encrypted_genesis_helm_fn(300)))
        .with_digest_key_blob_url(devnet_url("digest_key.bin"))
        .with_public_parameters_blob_url(devnet_url("pp.bin"))
        .with_validator_override_node_config_fn(Arc::new(|config, _| {
            apply_encrypted_validator_overrides(config);
        }))
        .with_fullnode_override_node_config_fn(Arc::new(|config, _| {
            config.api.allow_encrypted_txns_submission = true;
        }))
        .with_pfn_override_node_config_fn(Arc::new(|config, _| {
            config.api.allow_encrypted_txns_submission = true;
        }))
        .with_emit_job(
            EmitJobRequest::default()
                .mode(EmitJobMode::ConstTps { tps: 100 })
                .gas_price(5 * aptos_global_constants::GAS_UNIT_PRICE)
                .latency_polling_interval(Duration::from_millis(100)),
        )
        .with_success_criteria(success_criteria)
}

pub(crate) fn realistic_env_max_load_encrypted_mix_test(duration: Duration) -> ForgeConfig {
    let num_validators = 5;
    let num_fullnodes = 1;
    let mempool_backlog = 38000;

    let success_criteria = SuccessCriteria::new(15)
        .add_no_restarts()
        .add_wait_for_catchup_s((duration.as_secs() / 10).max(60))
        .add_latency_threshold(5.0, LatencyType::P50)
        .add_latency_threshold(7.0, LatencyType::P70)
        .add_chain_progress(StateProgressThreshold {
            max_non_epoch_no_progress_secs: 20.0,
            max_epoch_no_progress_secs: 20.0,
            max_non_epoch_round_gap: 6,
            max_epoch_round_gap: 6,
        });

    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(num_validators).unwrap())
        .with_initial_fullnode_count(num_fullnodes)
        .add_network_test(wrap_with_realistic_env(num_validators, TwoTrafficsTest {
            inner_traffic: EmitJobRequest::default()
                .mode(EmitJobMode::MaxLoad { mempool_backlog })
                .init_gas_price_multiplier(20)
                .transaction_mix(vec![
                    (
                        TransactionTypeArg::EncryptedCoinTransfer.materialize_default(),
                        1,
                    ),
                    (TransactionTypeArg::CoinTransfer.materialize_default(), 9),
                ]),
            inner_success_criteria: SuccessCriteria::new(300),
        }))
        .with_genesis_helm_config_fn(Arc::new(default_encrypted_genesis_helm_fn(300)))
        .with_digest_key_blob_url(devnet_url("digest_key.bin"))
        .with_public_parameters_blob_url(devnet_url("pp.bin"))
        .with_validator_override_node_config_fn(Arc::new(|config, _| {
            apply_encrypted_validator_overrides(config);
        }))
        .with_fullnode_override_node_config_fn(Arc::new(|config, _| {
            config.api.allow_encrypted_txns_submission = true;
        }))
        .with_emit_job(
            EmitJobRequest::default()
                .mode(EmitJobMode::ConstTps { tps: 100 })
                .gas_price(5 * aptos_global_constants::GAS_UNIT_PRICE)
                .latency_polling_interval(Duration::from_millis(100)),
        )
        .with_success_criteria(success_criteria)
}

/// Asserts that mainnet's fixed-round trusted setup actually gets exhausted under sustained
/// encrypted-txn load. Genesis starts with a short epoch so chunky DKG completes quickly; the test
/// then runs a governance script to extend `epoch_duration_secs` to 24h, keeping the chain in a
/// single working epoch for the rest of the run. After load, the test asserts that the
/// `trusted_setup_exhausted` decryption-pipeline counter crossed a non-trivial threshold.
pub(crate) fn realistic_env_max_load_encrypted_mainnet_test(duration: Duration) -> ForgeConfig {
    let num_validators = 5;
    let num_fullnodes = 1;
    let num_pfns = 3;
    let mempool_backlog = 1600;

    let success_criteria = SuccessCriteria::new(15)
        .add_no_restarts()
        .add_wait_for_catchup_s((duration.as_secs() / 10).max(60))
        .add_latency_threshold(5.0, LatencyType::P50)
        .add_latency_threshold(7.0, LatencyType::P70)
        .add_chain_progress(StateProgressThreshold {
            max_non_epoch_no_progress_secs: 20.0,
            max_epoch_no_progress_secs: 20.0,
            max_non_epoch_round_gap: 6,
            max_epoch_round_gap: 6,
        });

    let inner = TwoTrafficsTest {
        inner_traffic: EmitJobRequest::default()
            .mode(EmitJobMode::MaxLoad { mempool_backlog })
            .init_gas_price_multiplier(20)
            .encrypt_transactions(true),
        inner_success_criteria: SuccessCriteria::new(300),
    };

    let wrapper = EncryptedMainnetTest {
        inner,
        dkg_wait_timeout: Duration::from_secs(900),
        new_epoch_duration_secs: 24 * 3600,
        // The mainnet digest key is provisioned for 216k rounds. Once `block.round()` crosses
        // that ceiling, every encrypted txn in subsequent blocks is marked TrustedSetupExhausted.
        // We require at least 1k such markings as evidence the ceiling was actually reached
        // (not just that the run ended one block past it).
        min_trusted_setup_exhausted: 1_000,
    };

    ForgeConfig::default()
        .with_initial_validator_count(NonZeroUsize::new(num_validators).unwrap())
        .with_initial_fullnode_count(num_fullnodes)
        .with_num_pfns(num_pfns)
        .add_network_test(wrap_with_realistic_env(num_validators, wrapper))
        .with_genesis_helm_config_fn(Arc::new(default_encrypted_genesis_helm_fn(300)))
        .with_digest_key_blob_url(mainnet_url("digest_key.bin"))
        .with_public_parameters_blob_url(mainnet_url("pp.bin"))
        .with_validator_override_node_config_fn(Arc::new(|config, _| {
            apply_encrypted_validator_overrides(config);
        }))
        .with_fullnode_override_node_config_fn(Arc::new(|config, _| {
            config.api.allow_encrypted_txns_submission = true;
        }))
        .with_pfn_override_node_config_fn(Arc::new(|config, _| {
            config.api.allow_encrypted_txns_submission = true;
        }))
        .with_emit_job(
            EmitJobRequest::default()
                .mode(EmitJobMode::ConstTps { tps: 100 })
                .gas_price(5 * aptos_global_constants::GAS_UNIT_PRICE)
                .latency_polling_interval(Duration::from_millis(100)),
        )
        .with_success_criteria(success_criteria)
}

fn devnet_url(file: &str) -> String {
    format!(
        "https://github.com/aptos-labs/aptos-networks/raw/{}/devnet/{}",
        DEVNET_TRUSTED_SETUP_COMMIT, file
    )
}

fn mainnet_url(file: &str) -> String {
    format!(
        "https://github.com/aptos-labs/aptos-networks/raw/{}/mainnet/{}",
        MAINNET_TRUSTED_SETUP_COMMIT, file
    )
}

fn default_encrypted_genesis_helm_fn(
    epoch_duration_secs: u64,
) -> impl Fn(&mut serde_yaml::Value) + Send + Sync + 'static {
    move |helm_values: &mut serde_yaml::Value| {
        helm_values["chain"]["epoch_duration_secs"] = epoch_duration_secs.into();
        helm_values["chain"]["on_chain_consensus_config"] =
            serde_yaml::to_value(OnChainConsensusConfig::default_for_genesis())
                .expect("must serialize");
        helm_values["chain"]["on_chain_execution_config"] =
            serde_yaml::to_value(OnChainExecutionConfig::default_for_genesis())
                .expect("must serialize");
        helm_values["chain"]["randomness_config_override"] =
            serde_yaml::to_value(OnChainRandomnessConfig::default_enabled())
                .expect("must serialize");
        helm_values["chain"]["chunky_dkg_config_override"] =
            serde_yaml::to_value(OnChainChunkyDKGConfig::default_enabled())
                .expect("must serialize");
        let mut features = Features::default();
        features.enable(FeatureFlag::ENCRYPTED_TRANSACTIONS);
        helm_values["chain"]["initial_features_override"] =
            serde_yaml::to_value(features).expect("must serialize");
    }
}

fn apply_encrypted_validator_overrides(config: &mut aptos_config::config::NodeConfig) {
    config.api.allow_encrypted_txns_submission = true;
    config.consensus.quorum_store.enable_batch_v2_tx = true;
    config.consensus.quorum_store.enable_batch_v2_rx = true;
    config.consensus.quorum_store.enable_opt_qs_v2_payload_tx = true;
    config.consensus.quorum_store.enable_opt_qs_v2_payload_rx = true;
    config.consensus_observer.enable_v2_message_sending = true;
    config.consensus.digest_key_blob_path =
        Some("/opt/aptos/data/trusted-setup/digest_key.bin".into());
    config.consensus.public_parameters_blob_path =
        Some("/opt/aptos/data/trusted-setup/pp.bin".into());
}
