// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::{
    get_encryption_key_resource, verify_chunky_dkg_transcript, wait_for_chunky_dkg_finish,
};
use crate::{smoke_test_environment::SwarmBuilder, utils::get_on_chain_resource};
use aptos_forge::{Node, Swarm, SwarmExt};
use aptos_logger::info;
use aptos_types::{
    dkg::{chunky_dkg::ChunkyDKGState, DKGState},
    on_chain_config::{ChunkyDKGConfigMoveStruct, OnChainRandomnessConfig},
};
use std::{sync::Arc, time::Duration};

/// Enable chunky DKG config and the ENCRYPTED_TRANSACTIONS feature flag at
/// runtime via a governance Move script.  Randomness and validator txns are
/// already enabled at genesis; only the chunky-DKG-specific pieces are turned
/// on dynamically.
#[tokio::test]
async fn chunky_dkg_enable_feature() {
    let epoch_duration_secs = 20;
    let estimated_dkg_latency_secs = 120;

    // Genesis: randomness + validator txns enabled.
    // Chunky DKG config is OFF (default). ENCRYPTED_TRANSACTIONS is OFF.
    let (swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| {
            config.api.failpoints_enabled = true;
            config.api.allow_encrypted_txns_submission = true;
            config.consensus.quorum_store.enable_batch_v2_tx = true;
            config.consensus.quorum_store.enable_batch_v2_rx = true;
            config.consensus.quorum_store.enable_opt_qs_v2_payload_tx = true;
            config.consensus.quorum_store.enable_opt_qs_v2_payload_rx = true;
            config
                .state_sync
                .state_sync_driver
                .enable_auto_bootstrapping = true;
            config
                .state_sync
                .state_sync_driver
                .max_connection_deadline_secs = 3;
        }))
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;
            conf.allow_new_validators = true;
            conf.consensus_config.enable_validator_txns();
            conf.randomness_config_override = Some(OnChainRandomnessConfig::default_enabled());
            // Chunky DKG config defaults to Off. ENCRYPTED_TRANSACTIONS not set.
        }))
        .build_with_cli(0)
        .await;

    let root_addr = swarm.chain_info().root_account().address();
    let root_idx = cli.add_account_with_address_to_cli(swarm.root_key(), root_addr);

    let client_endpoint = swarm.validators().nth(1).unwrap().rest_api_endpoint();
    let client = aptos_rest_client::Client::new(client_endpoint.clone());

    // Wait for epoch 2 so the network is stable.
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(epoch_duration_secs * 3))
        .await
        .expect("Waited too long for epoch 2.");

    // Verify chunky DKG has NOT completed (config is off).
    let chunky_dkg_state = get_on_chain_resource::<ChunkyDKGState>(&client).await;
    assert!(
        chunky_dkg_state.last_completed.is_none(),
        "Chunky DKG should not have completed with config off"
    );
    info!("Verified: no chunky DKG session completed yet (config off).");

    // Enable chunky DKG config + ENCRYPTED_TRANSACTIONS via governance.
    info!("Enabling chunky DKG config and ENCRYPTED_TRANSACTIONS at runtime.");
    let script = r#"
script {
    use aptos_std::fixed_point64;
    use aptos_framework::aptos_governance;
    use aptos_framework::chunky_dkg_config;
    use aptos_framework::features;

    fun main(core_resources: &signer) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0x1);

        // Enable chunky DKG config (V1 with default thresholds).
        let config = chunky_dkg_config::new_v1(
            fixed_point64::create_from_rational(1, 2),
            fixed_point64::create_from_rational(2, 3)
        );
        chunky_dkg_config::set_for_next_epoch(&framework_signer, config);

        // Enable ENCRYPTED_TRANSACTIONS feature flag (108).
        features::change_feature_flags_for_next_epoch(&framework_signer, vector[108], vector[]);

        // Trigger reconfiguration.
        aptos_governance::reconfigure(&framework_signer);
    }
}
"#;

    cli.run_script(root_idx, script)
        .await
        .expect("Txn execution error.");

    // Poll and log state to diagnose the transition.
    info!("Polling DKG state after governance script...");
    let timer = tokio::time::Instant::now();
    let session = loop {
        let ledger = client
            .get_ledger_information()
            .await
            .expect("ledger info")
            .into_inner();
        let dkg_state = get_on_chain_resource::<ChunkyDKGState>(&client).await;
        let regular_dkg = get_on_chain_resource::<DKGState>(&client).await;
        let config = get_on_chain_resource::<ChunkyDKGConfigMoveStruct>(&client).await;
        info!(
            "epoch={} version={} chunky_in_progress={} chunky_completed={} regular_in_progress={} config={:?} elapsed={}s",
            ledger.epoch,
            ledger.version,
            dkg_state.in_progress.is_some(),
            dkg_state.last_completed.is_some(),
            regular_dkg.in_progress.is_some(),
            config,
            timer.elapsed().as_secs(),
        );
        if dkg_state.last_completed.is_some() {
            info!("Chunky DKG completed!");
            break dkg_state.last_complete().clone();
        }
        if timer.elapsed().as_secs() > estimated_dkg_latency_secs {
            panic!(
                "Timed out waiting for chunky DKG (epoch={}, in_progress={}, last_completed={})",
                ledger.epoch,
                dkg_state.in_progress.is_some(),
                dkg_state.last_completed.is_some(),
            );
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    };
    info!(
        "Chunky DKG completed for epoch {} after runtime enablement",
        session.target_epoch()
    );

    // Verify the transcript.
    let subtranscript = verify_chunky_dkg_transcript(&session);
    assert!(
        !subtranscript.dealers.is_empty(),
        "Transcript should have dealers"
    );

    // Verify encryption key is present.
    let enc_key = get_encryption_key_resource(&client).await;
    assert!(
        enc_key.encryption_key.is_some(),
        "Encryption key should be present after chunky DKG config is enabled"
    );
    info!(
        "Encryption key present at epoch {} ({} bytes)",
        enc_key.epoch,
        enc_key.encryption_key.as_ref().unwrap().len()
    );
}
