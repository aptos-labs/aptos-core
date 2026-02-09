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

/// Verify randomness works end-to-end with optimistic share verification enabled.
#[tokio::test]
async fn optimistic_verification_happy_path() {
    let epoch_duration_secs = 20;

    let (swarm, _cli, _faucet) = SwarmBuilder::new_local(4)
        .with_num_fullnodes(1)
        .with_aptos()
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;
            conf.consensus_config.enable_validator_txns();
            conf.consensus_config.disable_rand_check();
            conf.randomness_config_override = Some(OnChainRandomnessConfig::default_enabled());
        }))
        .build_with_cli(0)
        .await;

    let decrypt_key_map = decrypt_key_map(&swarm);
    let rest_client = swarm.validators().next().unwrap().rest_client();

    info!("Wait for epoch 2. Epoch 1 does not have randomness.");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Epoch 2 taking too long to arrive!");

    info!("Verify DKG correctness for epoch 2.");
    let dkg_session = get_on_chain_resource::<DKGState>(&rest_client).await;
    assert!(verify_dkg_transcript(dkg_session.last_complete(), &decrypt_key_map).is_ok());

    info!("Verify randomness correctness for 10 versions with optimistic verification.");
    for _ in 0..10 {
        let cur_txn_version = get_current_version(&rest_client).await;
        info!("Verifying WVUF output for version {}.", cur_txn_version);
        let wvuf_verify_result =
            verify_randomness(&decrypt_key_map, &rest_client, cur_txn_version).await;
        assert!(wvuf_verify_result.is_ok());
    }
}

/// Verify that randomness still works when one validator sends corrupted shares.
/// The corrupted share passes optimistic_verify (structural checks only) but fails
/// batch verification in aggregate(), triggering the fallback path which discards
/// the bad share and re-aggregates with remaining valid shares.
#[tokio::test]
async fn optimistic_verification_with_corrupt_share() {
    let epoch_duration_secs = 20;

    let (swarm, _cli, _faucet) = SwarmBuilder::new_local(4)
        .with_num_fullnodes(1)
        .with_aptos()
        .with_init_config(Arc::new(|_, conf, _| {
            conf.api.failpoints_enabled = true;
        }))
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;
            conf.consensus_config.enable_validator_txns();
            conf.consensus_config.disable_rand_check();
            conf.randomness_config_override = Some(OnChainRandomnessConfig::default_enabled());
        }))
        .build_with_cli(0)
        .await;

    let decrypt_key_map = decrypt_key_map(&swarm);
    let validator_clients: Vec<_> = swarm.validators().map(|v| v.rest_client()).collect();
    let rest_client = &validator_clients[0];

    info!("Wait for epoch 2.");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Epoch 2 taking too long to arrive!");

    info!("Verify randomness works before injecting fault.");
    for _ in 0..5 {
        let cur_txn_version = get_current_version(rest_client).await;
        let result = verify_randomness(&decrypt_key_map, rest_client, cur_txn_version).await;
        assert!(result.is_ok());
    }

    info!("Inject corrupt share failpoint on validator 0.");
    validator_clients[0]
        .set_failpoint(
            "consensus::rand::corrupt_share".to_string(),
            "return".to_string(),
        )
        .await
        .unwrap();

    info!("Wait for randomness to continue working despite corrupt shares.");
    tokio::time::sleep(Duration::from_secs(5)).await;

    info!("Verify randomness still works with one validator sending bad shares.");
    for _ in 0..10 {
        let cur_txn_version = get_current_version(rest_client).await;
        let result = verify_randomness(&decrypt_key_map, rest_client, cur_txn_version).await;
        assert!(
            result.is_ok(),
            "Randomness should survive corrupt shares via fallback"
        );
    }

    info!("Disable corrupt share failpoint.");
    validator_clients[0]
        .set_failpoint(
            "consensus::rand::corrupt_share".to_string(),
            "off".to_string(),
        )
        .await
        .unwrap();

    info!("Verify randomness continues working after failpoint disabled.");
    tokio::time::sleep(Duration::from_secs(5)).await;
    for _ in 0..5 {
        let cur_txn_version = get_current_version(rest_client).await;
        let result = verify_randomness(&decrypt_key_map, rest_client, cur_txn_version).await;
        assert!(result.is_ok());
    }
}
