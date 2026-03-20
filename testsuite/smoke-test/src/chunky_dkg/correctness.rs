// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::{
    create_swarm_with_chunky_dkg, get_encryption_key_resource, verify_chunky_dkg_transcript,
    wait_for_chunky_dkg_finish,
};
use aptos_forge::NodeExt;
use aptos_logger::info;

/// Consolidated correctness test covering:
/// 1. Basic chunky DKG completes successfully
/// 2. Transcript deserializes and has sufficient dealers
/// 3. Encryption key is derived from the DKG
/// 4. Multi-epoch key rotation works correctly
#[tokio::test]
async fn chunky_dkg_correctness() {
    let epoch_duration_secs = 10;
    let estimated_dkg_latency_secs = 20;
    let time_limit_secs = epoch_duration_secs + estimated_dkg_latency_secs;

    let swarm = create_swarm_with_chunky_dkg(4, epoch_duration_secs).await;
    let client = swarm.validators().last().unwrap().rest_client();

    // ---- Epoch 2: Initial DKG completion ----
    info!("Waiting for chunky DKG to finish for epoch 2...");
    let session_1 = wait_for_chunky_dkg_finish(&client, Some(2), time_limit_secs).await;
    info!(
        "Chunky DKG completed for epoch {}",
        session_1.target_epoch()
    );

    // Verify transcript
    let subtranscript_1 = verify_chunky_dkg_transcript(&session_1);
    assert!(
        subtranscript_1.dealers.len() >= 3,
        "Expected >= 3 dealers (BFT threshold), got {}",
        subtranscript_1.dealers.len()
    );

    // Verify encryption key
    let enc_key_1 = get_encryption_key_resource(&client).await;
    assert_eq!(enc_key_1.epoch, 2, "Encryption key epoch should be 2");
    assert!(
        enc_key_1.encryption_key.is_some(),
        "Encryption key should be present after DKG"
    );
    let key_bytes_1 = enc_key_1.encryption_key.unwrap();
    info!("Encryption key at epoch 2: {} bytes", key_bytes_1.len());

    // ---- Epoch 3: Key rotation ----
    info!("Waiting for chunky DKG to finish for epoch 3...");
    let session_2 = wait_for_chunky_dkg_finish(&client, Some(3), time_limit_secs).await;
    info!(
        "Chunky DKG completed for epoch {}",
        session_2.target_epoch()
    );

    // Verify second transcript
    let subtranscript_2 = verify_chunky_dkg_transcript(&session_2);
    assert!(
        subtranscript_2.dealers.len() >= 3,
        "Expected >= 3 dealers for second session, got {}",
        subtranscript_2.dealers.len()
    );

    // Verify consecutive dealer epochs
    assert_eq!(
        session_2.metadata.dealer_epoch,
        session_1.metadata.dealer_epoch + 1,
        "Sessions should be for consecutive dealer epochs"
    );

    // Verify transcript bytes differ between epochs
    assert_ne!(
        session_1.transcript, session_2.transcript,
        "Transcript bytes should differ between epochs"
    );

    // Verify encryption key changed
    let enc_key_2 = get_encryption_key_resource(&client).await;
    assert_eq!(enc_key_2.epoch, 3, "Encryption key epoch should be 3");
    assert!(
        enc_key_2.encryption_key.is_some(),
        "Encryption key should be present at epoch 3"
    );
    let key_bytes_2 = enc_key_2.encryption_key.unwrap();
    assert_ne!(
        key_bytes_1, key_bytes_2,
        "Encryption key must change between epoch 2 and epoch 3"
    );
    info!("Encryption key rotated successfully between epochs");
}
