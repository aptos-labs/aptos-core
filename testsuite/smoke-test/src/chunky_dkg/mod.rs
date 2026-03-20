// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{smoke_test_environment::SwarmBuilder, utils};
use aptos_forge::LocalSwarm;
use aptos_rest_client::Client;
use aptos_types::{
    decryption::PerEpochEncryptionKeyResource,
    dkg::chunky_dkg::{AggregatedSubtranscript, ChunkyDKG, ChunkyDKGSessionState, ChunkyDKGState},
    on_chain_config::{FeatureFlag, Features, OnChainChunkyDKGConfig, OnChainRandomnessConfig},
};
use move_core_types::{language_storage::CORE_CODE_ADDRESS, move_resource::MoveStructType};
use std::{sync::Arc, time::Duration};
use tokio::time::Instant;

mod correctness;
mod enable_feature;
mod with_validator_down;

/// Poll on-chain `ChunkyDKGState` until we see a completed session.
/// Returns the `ChunkyDKGSessionState` of the completed session.
#[allow(dead_code)]
async fn wait_for_chunky_dkg_finish(
    client: &Client,
    target_epoch: Option<u64>,
    time_limit_secs: u64,
) -> ChunkyDKGSessionState {
    let mut dkg_state = utils::get_on_chain_resource::<ChunkyDKGState>(client).await;
    let timer = Instant::now();
    while timer.elapsed().as_secs() < time_limit_secs
        && !(dkg_state.in_progress.is_none()
            && dkg_state.last_completed.is_some()
            && (target_epoch.is_none()
                || dkg_state
                    .last_completed
                    .as_ref()
                    .map(|session| session.target_epoch())
                    == target_epoch))
    {
        tokio::time::sleep(Duration::from_secs(1)).await;
        dkg_state = utils::get_on_chain_resource::<ChunkyDKGState>(client).await;
    }
    assert!(
        timer.elapsed().as_secs() < time_limit_secs,
        "Timed out waiting for chunky DKG to finish (target_epoch={:?})",
        target_epoch,
    );
    dkg_state.last_complete().clone()
}

/// Deserialize and verify a chunky DKG transcript from a completed session.
/// Returns the deserialized `AggregatedSubtranscript`.
#[allow(dead_code)]
fn verify_chunky_dkg_transcript(session: &ChunkyDKGSessionState) -> AggregatedSubtranscript {
    let subtranscript: AggregatedSubtranscript =
        bcs::from_bytes(&session.transcript).expect("Failed to deserialize transcript bytes");

    assert!(
        !subtranscript.dealers.is_empty(),
        "Transcript should have at least one dealer"
    );

    // Validate metadata consistency by generating a config from the session metadata.
    let _config = ChunkyDKG::generate_config(&session.metadata);

    subtranscript
}

/// Query the `PerEpochEncryptionKeyResource` from the on-chain state.
#[allow(dead_code)]
async fn get_encryption_key_resource(client: &Client) -> PerEpochEncryptionKeyResource {
    let tag = PerEpochEncryptionKeyResource::struct_tag();
    client
        .get_account_resource_bcs::<PerEpochEncryptionKeyResource>(
            CORE_CODE_ADDRESS,
            &tag.to_canonical_string(),
        )
        .await
        .expect("Failed to get PerEpochEncryptionKeyResource")
        .into_inner()
}

/// Create a local swarm with chunky DKG enabled.
/// Reuses the same genesis configuration as `decryption.rs`.
#[allow(dead_code)]
async fn create_swarm_with_chunky_dkg(
    num_validators: usize,
    epoch_duration_secs: u64,
) -> LocalSwarm {
    SwarmBuilder::new_local(num_validators)
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
            conf.consensus_config.enable_validator_txns();
            conf.randomness_config_override = Some(OnChainRandomnessConfig::default_enabled());
            conf.chunky_dkg_config_override = Some(OnChainChunkyDKGConfig::default_enabled());
            let mut features = Features::default();
            features.enable(FeatureFlag::ENCRYPTED_TRANSACTIONS);
            conf.initial_features_override = Some(features);
        }))
        .build()
        .await
}
