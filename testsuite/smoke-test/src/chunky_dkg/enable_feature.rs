// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::{get_encryption_key_resource, verify_chunky_dkg_transcript};
use crate::{smoke_test_environment::SwarmBuilder, utils::get_on_chain_resource};
use aptos_forge::{Node, Swarm, SwarmExt};
use aptos_logger::{debug, info};
use aptos_types::{dkg::chunky_dkg::ChunkyDKGState, on_chain_config::OnChainRandomnessConfig};
use std::{sync::Arc, time::Duration};

/// Enable chunky DKG config and the ENCRYPTED_TRANSACTIONS feature flag via
/// a governance Move script at runtime, with randomness and validator txns
/// already enabled at genesis.
#[tokio::test]
async fn chunky_dkg_enable_feature() {
    let epoch_duration_secs = 20;
    let estimated_dkg_latency_secs = 40;

    // Start with randomness and validator txns enabled at genesis,
    // but chunky DKG and encrypted transactions disabled.
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
            // chunky DKG and ENCRYPTED_TRANSACTIONS are NOT enabled at genesis.
        }))
        .build_with_cli(0)
        .await;

    let root_addr = swarm.chain_info().root_account().address();
    let root_idx = cli.add_account_with_address_to_cli(swarm.root_key(), root_addr);

    let client_endpoint = swarm.validators().nth(1).unwrap().rest_api_endpoint();
    let client = aptos_rest_client::Client::new(client_endpoint.clone());

    // Wait for epoch 3 so the network is stable.
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(3, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Waited too long for epoch 3.");

    // Enable chunky DKG config and ENCRYPTED_TRANSACTIONS feature flag via governance.
    info!("Now in epoch 3. Enabling chunky DKG config and ENCRYPTED_TRANSACTIONS feature.");
    let script = r#"
script {
    use aptos_framework::aptos_governance;
    use aptos_framework::chunky_dkg_config;
    use aptos_std::fixed_point64;

    fun main(core_resources: &signer) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0x1);

        // Enable chunky DKG.
        let chunky_dkg_config = chunky_dkg_config::new_v1(
            fixed_point64::create_from_rational(1, 2),
            fixed_point64::create_from_rational(2, 3)
        );
        chunky_dkg_config::set_for_next_epoch(&framework_signer, chunky_dkg_config);

        // Enable ENCRYPTED_TRANSACTIONS feature flag (108).
        aptos_governance::toggle_features(
            &framework_signer,
            vector[108],
            vector[]
        );
    }
}
"#;

    debug!("script={}", script);
    let txn_summary = cli
        .run_script(root_idx, script)
        .await
        .expect("Txn execution error.");
    debug!("txn_summary={:?}", txn_summary);

    // Epoch 4: configs are now active, but chunky DKG hasn't completed yet.
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(4, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Waited too long for epoch 4.");

    info!(
        "Now in epoch 4. Chunky DKG should not have completed yet (no DKG ran at end of epoch 3)."
    );
    let chunky_dkg_state = get_on_chain_resource::<ChunkyDKGState>(&client).await;
    let no_chunky_dkg_yet = chunky_dkg_state.last_completed.is_none()
        || chunky_dkg_state
            .last_completed
            .as_ref()
            .map(|s| s.target_epoch())
            != Some(4);
    assert!(
        no_chunky_dkg_yet,
        "Chunky DKG should not have completed for epoch 4 yet"
    );

    // Epoch 5: DKG should have run during epoch 4 and completed for epoch 5.
    info!("Waiting for epoch 5 (chunky DKG should complete)...");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(
            5,
            Duration::from_secs(epoch_duration_secs + estimated_dkg_latency_secs),
        )
        .await
        .expect("Waited too long for epoch 5.");

    let chunky_dkg_state = get_on_chain_resource::<ChunkyDKGState>(&client).await;
    let session = chunky_dkg_state
        .last_completed
        .expect("Chunky DKG should have completed for epoch 5");
    assert_eq!(5, session.target_epoch());

    // Verify the transcript is valid.
    let subtranscript = verify_chunky_dkg_transcript(&session);
    assert!(
        !subtranscript.dealers.is_empty(),
        "Transcript should have dealers"
    );
    info!(
        "Chunky DKG completed for epoch 5 with {} dealers after runtime enablement",
        subtranscript.dealers.len()
    );

    // Verify encryption key was derived.
    let enc_key = get_encryption_key_resource(&client).await;
    assert!(
        enc_key.encryption_key.is_some(),
        "Encryption key should be present after chunky DKG"
    );
    info!(
        "Encryption key present at epoch {} ({} bytes)",
        enc_key.epoch,
        enc_key.encryption_key.unwrap().len()
    );
}
