// Copyright © Aptos Foundation

use crate::{
    randomness::{get_current_version, get_on_chain_resource_at_version},
    smoke_test_environment::SwarmBuilder,
    utils::get_consensus_config_at_version,
};
use aptos_forge::{Node, Swarm, SwarmExt};
use aptos_logger::info;
use aptos_rest_client::Client;
use aptos_types::on_chain_config::{
    FeatureFlag, Features, OnChainConsensusConfig, ValidatorTxnConfig, Version,
};
use std::{sync::Arc, time::Duration};

/// On-chain config updates should be buffered until next epoch.
/// TODO: include all on-chain configs? Currently it only covers `Features`, `Version`, `ConsensusConfig`.
#[tokio::test]
async fn on_chain_config_updates() {
    let epoch_duration_secs = 60;

    let (mut swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
        .with_num_fullnodes(1)
        .with_aptos()
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;
            conf.allow_new_validators = true;

            // Ensure vtxn is enabled.
            conf.consensus_config.enable_validator_txns();

            let mut features = Features::default();
            // Ensure randomness flag is set.
            features.enable(FeatureFlag::RECONFIGURE_WITH_DKG);
            // Ensure the state of 2 features. Will update them during the test.
            features.enable(FeatureFlag::BN254_STRUCTURES);
            features.disable(FeatureFlag::BLS12_381_STRUCTURES);
            conf.initial_features_override = Some(features);
        }))
        .build_with_cli(0)
        .await;

    let root_addr = swarm.chain_info().root_account().address();
    let root_idx = cli.add_account_with_address_to_cli(swarm.root_key(), root_addr);

    let client_endpoint = swarm.validators().nth(1).unwrap().rest_api_endpoint();
    let client = Client::new(client_endpoint.clone());

    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Waited too long for epoch 2.");

    info!("Now in epoch 2. Make some on-chain config updates.");
    info!("Grouping them into 2 txns and order some after the `reconfigure()` call to make it harder to pass...");
    let (version_resource_v0, features_resource_v0, consensus_config_v0) =
        get_latest_on_chain_config_resources(&client).await;
    let proposed_major_version = version_resource_v0.major + 1000;
    let mut proposed_consensus_config = consensus_config_v0.clone();
    bump_vtxn_count_limit(&mut proposed_consensus_config);
    let proposed_consensus_config_bytes = bcs::to_bytes(&proposed_consensus_config).unwrap();

    // Txn 0: set version to 999; disable feature bn254.
    let txn_0_script = format!(
        r#"
script {{
    use aptos_framework::aptos_governance;
    use aptos_framework::version;
    use std::features;
    fun main(core_resources: &signer) {{
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);
        version::set_for_next_epoch(&framework_signer, {proposed_major_version});
        features::change_feature_flags_for_next_epoch(&framework_signer, vector[], vector[features::get_bn254_strutures_feature()]);
    }}
}}
"#
    );

    // Txn 1: enable feature bls12381; trigger async config; bump vtxn count limit in consensus config.
    let txn_1_script = format!(
        r#"
script {{
    use aptos_framework::consensus_config;
    use aptos_framework::aptos_governance;
    use std::features;
    fun main(core_resources: &signer) {{
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);
        features::change_feature_flags_for_next_epoch(&framework_signer, vector[features::get_bls12_381_strutures_feature()], vector[]);
        aptos_governance::reconfigure(&framework_signer);
        let target_config_bytes = vector{proposed_consensus_config_bytes:?};
        consensus_config::set_for_next_epoch(&framework_signer, target_config_bytes);
    }}
}}
"#
    );

    let txn_0_summary = cli.run_script(root_idx, txn_0_script.as_str()).await;
    info!("txn_0_summary={:?}", txn_0_summary);
    let txn_1_summary = cli.run_script(root_idx, txn_1_script.as_str()).await;
    info!("txn_1_summary={:?}", txn_1_summary);

    info!(
        "After updates are executed, the current on-chain config values should remain unchanged."
    );
    let (version_resource_v1, features_resource_v1, consensus_config_v1) =
        get_latest_on_chain_config_resources(&client).await;
    assert_eq!(version_resource_v0, version_resource_v1);
    assert_eq!(features_resource_v0, features_resource_v1);
    assert_eq!(consensus_config_v0, consensus_config_v1);
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(3, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Waited too long for epoch 3.");

    let (version_resource_v2, features_resource_v2, consensus_config_v2) =
        get_latest_on_chain_config_resources(&client).await;
    assert_eq!(proposed_major_version, version_resource_v2.major);
    assert!(features_resource_v2.is_enabled(FeatureFlag::BLS12_381_STRUCTURES));
    assert!(!features_resource_v2.is_enabled(FeatureFlag::BN254_STRUCTURES));
    assert_eq!(proposed_consensus_config, consensus_config_v2);
}

async fn get_latest_on_chain_config_resources(
    client: &Client,
) -> (Version, Features, OnChainConsensusConfig) {
    let cur_txn_version = get_current_version(client).await;
    tokio::join!(
        get_on_chain_resource_at_version::<Version>(client, cur_txn_version),
        get_on_chain_resource_at_version::<Features>(client, cur_txn_version),
        get_consensus_config_at_version(client, cur_txn_version),
    )
}

fn bump_vtxn_count_limit(consensus_config: &mut OnChainConsensusConfig) {
    match consensus_config {
        OnChainConsensusConfig::V3 {
            vtxn:
                ValidatorTxnConfig::V1 {
                    per_block_limit_txn_count,
                    ..
                },
            ..
        } => {
            *per_block_limit_txn_count += 1;
        },
        _ => unreachable!(),
    }
}
