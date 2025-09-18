// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    randomness::{decrypt_key_map, get_current_version, verify_dkg_transcript, verify_randomness},
    smoke_test_environment::SwarmBuilder,
    utils::get_on_chain_resource,
};
use aptos_forge::{NodeExt, SwarmExt};
use aptos_logger::info;
use aptos_types::{dkg::DKGState, on_chain_config::OnChainRandomnessConfig};
use std::{sync::Arc, time::Duration};

/// Verify the correctness of DKG transcript and block-level randomness seed.
#[tokio::test]
async fn randomness_correctness() {
    let epoch_duration_secs = 20;

    let (swarm, _cli, _faucet) = SwarmBuilder::new_local(4)
        .with_num_fullnodes(1)
        .with_aptos()
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;

            // Ensure randomness is enabled.
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

    // Verify the randomness in 10 versions.
    for _ in 0..10 {
        let cur_txn_version = get_current_version(&rest_client).await;
        info!("Verifying WVUF output for version {}.", cur_txn_version);
        let wvuf_verify_result =
            verify_randomness(&decrypt_key_map, &rest_client, cur_txn_version).await;
        println!("wvuf_verify_result={:?}", wvuf_verify_result);
        assert!(wvuf_verify_result.is_ok());
    }

    info!("Wait for epoch 3.");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(3, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Epoch 3 taking too long to arrive!");

    info!("Verify DKG correctness for epoch 3.");
    let dkg_session = get_on_chain_resource::<DKGState>(&rest_client).await;
    assert!(verify_dkg_transcript(dkg_session.last_complete(), &decrypt_key_map).is_ok());

    // Again, verify the randomness in 10 versions.
    for _ in 0..10 {
        let cur_txn_version = get_current_version(&rest_client).await;
        info!("Verifying WVUF output for version {}.", cur_txn_version);
        let wvuf_verify_result =
            verify_randomness(&decrypt_key_map, &rest_client, cur_txn_version).await;
        println!("wvuf_verify_result={:?}", wvuf_verify_result);
        assert!(wvuf_verify_result.is_ok());
    }
}
