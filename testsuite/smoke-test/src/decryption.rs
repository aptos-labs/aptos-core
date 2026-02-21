// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{smoke_test_environment::SwarmBuilder, txn_emitter::generate_traffic};
use aptos_forge::{EmitJobMode, LocalSwarm, NodeExt, TransactionType};
use aptos_logger::info;
use aptos_rest_client::Client;
use aptos_types::on_chain_config::{
    FeatureFlag, Features, OnChainChunkyDKGConfig, OnChainRandomnessConfig,
};
use std::{sync::Arc, time::Duration};

/// Wait until the ledger reaches the given epoch, returning the encryption key bytes if present.
async fn wait_for_epoch(client: &Client, target_epoch: u64, timeout_secs: u64) -> Option<Vec<u8>> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        let state = client
            .get_ledger_information()
            .await
            .expect("failed to get ledger info")
            .into_inner();
        if state.epoch >= target_epoch {
            return state.encryption_key;
        }
        if tokio::time::Instant::now() > deadline {
            panic!(
                "timed out waiting for epoch {}, current epoch is {}",
                target_epoch, state.epoch
            );
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

/// Count the number of encrypted user transactions in the range [start_version, end_version).
async fn count_encrypted_txns(client: &Client, start_version: u64, end_version: u64) -> (u64, u64) {
    let mut count = 0u64;
    let mut decrypted_count = 0u64;
    let page_size = 100u16;
    let mut cursor = start_version;
    while cursor < end_version {
        let limit = std::cmp::min(page_size as u64, end_version - cursor) as u16;
        let txns = client
            .get_transactions_bcs(Some(cursor), Some(limit))
            .await
            .expect("failed to get transactions")
            .into_inner();
        for txn_data in &txns {
            if let Some(signed_txn) = txn_data.transaction.try_as_signed_user_txn() {
                if let Some(payload) = signed_txn.payload().as_encrypted_payload() {
                    count += 1;
                    if !payload.is_encrypted() {
                        decrypted_count += 1;
                    }
                }
            }
        }
        cursor += txns.len() as u64;
        if txns.is_empty() {
            break;
        }
    }
    (count, decrypted_count)
}

async fn create_swarm_with_encryption(num_validators: usize) -> LocalSwarm {
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
        .with_init_genesis_config(Arc::new(|conf| {
            conf.epoch_duration_secs = 10;
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

/// Smoke test that verifies:
/// 1. An encryption key exists after epoch 2.
/// 2. The encryption key changes between epochs.
/// 3. Encrypted transactions are committed (via the emitter).
#[tokio::test]
async fn test_encryption_key_rotation_and_encrypted_txns() {
    let num_validators = 4;
    let mut swarm = create_swarm_with_encryption(num_validators).await;

    let client = swarm.validators().last().unwrap().rest_client();

    // ---- Wait for epoch 2 and record the encryption key ----
    info!("Waiting for epoch 2...");
    let key_at_epoch2 = wait_for_epoch(&client, 2, 90).await;
    let epoch2 = client
        .get_ledger_information()
        .await
        .unwrap()
        .into_inner()
        .epoch;
    info!(
        "Reached epoch {} with encryption key present: {}",
        epoch2,
        key_at_epoch2.is_some()
    );
    assert!(
        key_at_epoch2.is_some(),
        "Encryption key should exist after epoch 2, but was None"
    );

    // Record the ledger version so we can scan transactions later.
    let version_before_traffic = client
        .get_ledger_information()
        .await
        .unwrap()
        .into_inner()
        .version;

    // ---- Use the emitter to generate encrypted traffic ----
    info!("Emitting encrypted traffic...");
    let all_validators: Vec<_> = swarm.validators().map(|v| v.peer_id()).collect();
    let stats = generate_traffic(
        &mut swarm,
        &all_validators,
        Duration::from_secs(20),
        100,
        vec![vec![(TransactionType::default(), 1)]],
        true,
        Some(EmitJobMode::MaxLoad {
            mempool_backlog: 20,
        }),
    )
    .await
    .unwrap();
    info!(
        "Emitter stats: submitted={}, committed={}",
        stats.submitted, stats.committed
    );
    assert!(
        stats.committed > 0,
        "Expected some committed transactions from the emitter, got 0"
    );

    // ---- Wait for the next epoch and check the key changed ----
    info!("Waiting for epoch {}...", epoch2 + 1);
    let key_at_next_epoch = wait_for_epoch(&client, epoch2 + 1, 60).await;
    let next_epoch = client
        .get_ledger_information()
        .await
        .unwrap()
        .into_inner()
        .epoch;
    info!(
        "Reached epoch {} with encryption key present: {}",
        next_epoch,
        key_at_next_epoch.is_some()
    );

    assert!(
        key_at_next_epoch.is_some(),
        "Encryption key should exist at epoch {}, but was None",
        next_epoch
    );
    assert_ne!(
        key_at_epoch2.unwrap(),
        key_at_next_epoch.unwrap(),
        "Encryption key must change between epoch {} and epoch {}",
        epoch2,
        next_epoch,
    );

    // ---- Count encrypted transactions in the committed history ----
    let final_version = client
        .get_ledger_information()
        .await
        .unwrap()
        .into_inner()
        .version;

    let (encrypted_count, decrypted_count) =
        count_encrypted_txns(&client, version_before_traffic, final_version).await;
    info!(
        "Found {} encrypted transactions ({} decrypted) between version {} and {}",
        encrypted_count, decrypted_count, version_before_traffic, final_version
    );
    assert!(
        encrypted_count > 0,
        "Expected encrypted transactions to be committed, but found 0 in versions [{}, {})",
        version_before_traffic,
        final_version
    );
    assert!(
        decrypted_count > 0,
        "Expected decrypted encrypted transactions to be committed, but found 0 in versions [{}, {})",
        version_before_traffic,
        final_version
    );
}
