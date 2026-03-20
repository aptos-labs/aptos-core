// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    randomness::{decrypt_key_map, get_current_version, verify_dkg_transcript, verify_randomness},
    smoke_test_environment::SwarmBuilder,
    utils::get_on_chain_resource,
};
use aptos_forge::{NodeExt, SwarmExt};
use aptos_logger::info;
use aptos_types::{dkg::DKGState, on_chain_config::OnChainRandomnessConfig};
use std::{sync::Arc, time::Duration};

/// Combined test for optimistic share verification covering:
/// 1. Happy path: all shares valid, randomness produced normally
/// 2. Corrupt share with sufficient valid shares: pre_aggregate_verify detects
///    and removes the bad share, 3 remaining valid shares meet threshold
/// 3. Stall: corrupt share + stopped validator = only 2 valid shares < threshold,
///    chain stalls because consensus waits for randomness
/// 4. Recovery: restart stopped validator, 3 valid shares meet threshold again,
///    chain resumes
#[tokio::test]
async fn optimistic_verification() {
    let epoch_duration_secs = 20;

    let (mut swarm, _cli, _faucet) = SwarmBuilder::new_local(4)
        .with_num_fullnodes(1)
        .with_aptos()
        .with_init_config(Arc::new(|_, conf, _| {
            conf.api.failpoints_enabled = true;
        }))
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;
            conf.consensus_config.enable_validator_txns();
            conf.consensus_config.disable_rand_check();
            conf.randomness_config_override = Some(OnChainRandomnessConfig::new_v1(50, 66));
        }))
        .build_with_cli(0)
        .await;

    let decrypt_key_map = decrypt_key_map(&swarm);
    let validator_clients: Vec<_> = swarm.validators().map(|v| v.rest_client()).collect();
    let rest_client = &validator_clients[1]; // validator 1 stays up throughout

    // --- Phase 1: Happy path ---
    info!("Wait for epoch 2. Epoch 1 does not have randomness.");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Epoch 2 taking too long to arrive!");

    info!("Verify DKG correctness for epoch 2.");
    let dkg_session = get_on_chain_resource::<DKGState>(rest_client).await;
    assert!(verify_dkg_transcript(dkg_session.last_complete(), &decrypt_key_map).is_ok());

    info!("Verify randomness correctness for 10 versions.");
    for _ in 0..10 {
        let v = get_current_version(rest_client).await;
        assert!(verify_randomness(&decrypt_key_map, rest_client, v)
            .await
            .is_ok());
    }

    // --- Phase 2: Corrupt share, sufficient valid shares ---
    info!("Inject corrupt share failpoint on validator 0.");
    validator_clients[0]
        .set_failpoint(
            "consensus::rand::corrupt_share".to_string(),
            "return".to_string(),
        )
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_secs(5)).await;

    info!("Verify randomness still works with 3 valid shares.");
    for _ in 0..10 {
        let v = get_current_version(rest_client).await;
        assert!(
            verify_randomness(&decrypt_key_map, rest_client, v)
                .await
                .is_ok(),
            "Randomness should survive corrupt shares via fallback"
        );
    }

    // --- Phase 3: Stall — stop validator 3, only 2 valid shares remain ---
    info!("Stop validator 3.");
    swarm.validators_mut().nth(3).unwrap().stop();

    // Wait for pipeline to drain, then verify chain has stalled.
    tokio::time::sleep(Duration::from_secs(10)).await;
    let version_before = get_current_version(rest_client).await;
    tokio::time::sleep(Duration::from_secs(10)).await;
    let version_after = get_current_version(rest_client).await;
    info!(
        "Chain stall check: version_before={}, version_after={}",
        version_before, version_after
    );
    assert_eq!(
        version_before, version_after,
        "Chain should stall with only 2 valid shares (below threshold 3)"
    );

    // --- Phase 4: Recovery — restart validator 3 ---
    info!("Start validator 3.");
    swarm.validators_mut().nth(3).unwrap().start().unwrap();

    tokio::time::sleep(Duration::from_secs(15)).await;

    info!("Verify chain resumed and randomness works via unhappy path.");
    let version_after_recovery = get_current_version(rest_client).await;
    assert!(
        version_after_recovery > version_after,
        "Chain should resume after starting validator 3"
    );
    for _ in 0..10 {
        let v = get_current_version(rest_client).await;
        assert!(
            verify_randomness(&decrypt_key_map, rest_client, v)
                .await
                .is_ok(),
            "Randomness should work via unhappy path with 3 valid shares"
        );
    }
}
