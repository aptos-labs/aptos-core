// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{smoke_test_environment::SwarmBuilder, txn_emitter::generate_traffic};
use aptos_forge::{EmitJobMode, NodeExt, SwarmExt, TransactionType};
use aptos_logger::info;
use aptos_types::on_chain_config::{
    FeatureFlag, Features, OnChainChunkyDKGConfig, OnChainRandomnessConfig,
};
use std::{sync::Arc, time::Duration};

/// Verify that SecretShareMsg works via the RPC path (not just direct send).
///
/// This test disables the direct-send broadcast of secret shares via a failpoint,
/// forcing the system to rely on the RPC path. If the RPC handler for SecretShareMsg
/// is missing, validators will not be able to exchange secret shares and the chain
/// will stall.
#[tokio::test]
async fn secret_share_rpc_path() {
    let epoch_duration_secs = 20;

    let mut swarm = SwarmBuilder::new_local(4)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| {
            config.api.failpoints_enabled = true;
            config.api.allow_encrypted_txns_submission = true;
            config.consensus.quorum_store.enable_batch_v2_tx = true;
            config.consensus.quorum_store.enable_batch_v2_rx = true;
            config.consensus.quorum_store.enable_opt_qs_v2_payload_tx = true;
            config.consensus.quorum_store.enable_opt_qs_v2_payload_rx = true;
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
        .await;

    let validator_clients: Vec<_> = swarm.validators().map(|v| v.rest_client()).collect();

    // Wait for epoch 2 so DKG + secret sharing are active.
    info!("Waiting for epoch 2...");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(epoch_duration_secs * 5))
        .await
        .expect("Timed out waiting for epoch 2");

    // Disable secret share direct-send on all validators via failpoint.
    // This forces the system to use only the RPC path for secret share exchange.
    info!("Injecting failpoint to disable secret share direct-send on all validators...");
    for client in &validator_clients {
        client
            .set_failpoint(
                "consensus::send::broadcast_secret_share".to_string(),
                "return".to_string(),
            )
            .await
            .expect("Failed to set failpoint");
    }

    // Generate encrypted traffic to exercise the decryption / secret share path.
    info!("Emitting encrypted traffic with direct-send disabled...");
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
    .expect("Failed to generate encrypted traffic");
    info!(
        "Emitter stats: submitted={}, committed={}",
        stats.submitted, stats.committed
    );
    assert!(
        stats.committed > 0,
        "Expected committed encrypted transactions, got 0"
    );

    // Wait for the chain to advance to epoch 3. This requires a new DKG round and
    // secret share exchange, which can now only succeed via the RPC path.
    info!("Waiting for epoch 3 with secret share direct-send disabled...");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(3, Duration::from_secs(epoch_duration_secs * 5))
        .await
        .expect("Timed out waiting for epoch 3 — SecretShareMsg RPC path may be broken");

    info!("All validators reached epoch 3 using only the RPC path for secret shares.");
}
