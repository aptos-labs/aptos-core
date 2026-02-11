// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    randomness::{
        decrypt_key_map, get_current_version, script_to_enable_main_logic, verify_dkg_transcript,
        verify_randomness,
    },
    smoke_test_environment::SwarmBuilder,
    utils::get_on_chain_resource,
};
use aptos_forge::{Node, Swarm, SwarmExt};
use aptos_logger::{debug, info};
use aptos_types::{dkg::DKGState, on_chain_config::OnChainRandomnessConfig};
use std::{sync::Arc, time::Duration};

/// Start with randomness config V2 (fast path enabled), switch to V1 (fast path disabled)
/// via governance. The chain and randomness feature should continue to work.
#[tokio::test]
async fn v2_to_v1_config_switch() {
    let epoch_duration_secs = 20;
    let estimated_dkg_latency_secs = 40;

    let (swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
        .with_num_fullnodes(1)
        .with_aptos()
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;
            conf.allow_new_validators = true;

            // Start with V2 (fast path enabled).
            conf.consensus_config.enable_validator_txns();
            conf.randomness_config_override = Some(OnChainRandomnessConfig::default_enabled());
        }))
        .build_with_cli(0)
        .await;

    let root_addr = swarm.chain_info().root_account().address();
    let root_idx = cli.add_account_with_address_to_cli(swarm.root_key(), root_addr);

    let decrypt_key_map = decrypt_key_map(&swarm);

    let client_endpoint = swarm.validators().nth(1).unwrap().rest_api_endpoint();
    let client = aptos_rest_client::Client::new(client_endpoint.clone());

    // First DKG runs at end of epoch 2; randomness is first available in epoch 3.
    info!("Wait for epoch 3 (first epoch with randomness; V2 config, fast path enabled).");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(
            3,
            Duration::from_secs(epoch_duration_secs * 2 + estimated_dkg_latency_secs),
        )
        .await
        .expect("Epoch 3 taking too long to arrive.");

    info!("Verify DKG and randomness under V2.");
    let dkg_session = get_on_chain_resource::<DKGState>(&client).await;
    assert!(verify_dkg_transcript(dkg_session.last_complete(), &decrypt_key_map).is_ok());
    for _ in 0..5 {
        let cur_txn_version = get_current_version(&client).await;
        let wvuf_verify_result =
            verify_randomness(&decrypt_key_map, &client, cur_txn_version).await;
        assert!(wvuf_verify_result.is_ok(), "{:?}", wvuf_verify_result);
    }

    info!("Switch randomness config from V2 to V1 (disable fast path) for next epoch.");
    let script = script_to_enable_main_logic();
    let txn_summary = cli
        .run_script(root_idx, script.as_str())
        .await
        .expect("Txn execution error.");
    debug!("txn_summary={:?}", txn_summary);

    info!("Wait for epoch 4 (config change takes effect; V1, fast path disabled).");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(4, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Epoch 4 taking too long to arrive.");

    info!("Verify chain and randomness still work under V1.");
    let dkg_session = get_on_chain_resource::<DKGState>(&client).await;
    assert!(verify_dkg_transcript(dkg_session.last_complete(), &decrypt_key_map).is_ok());
    for _ in 0..5 {
        let cur_txn_version = get_current_version(&client).await;
        let wvuf_verify_result =
            verify_randomness(&decrypt_key_map, &client, cur_txn_version).await;
        assert!(wvuf_verify_result.is_ok(), "{:?}", wvuf_verify_result);
    }

    info!("Wait for epoch 5 and verify DKG and randomness again.");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(
            5,
            Duration::from_secs(epoch_duration_secs + estimated_dkg_latency_secs),
        )
        .await
        .expect("Epoch 5 taking too long to arrive.");

    let dkg_session = get_on_chain_resource::<DKGState>(&client)
        .await
        .last_completed
        .expect("dkg result for epoch 5 should be present");
    assert_eq!(5, dkg_session.target_epoch());
    assert!(verify_dkg_transcript(&dkg_session, &decrypt_key_map).is_ok());
    for _ in 0..5 {
        let cur_txn_version = get_current_version(&client).await;
        let wvuf_verify_result =
            verify_randomness(&decrypt_key_map, &client, cur_txn_version).await;
        assert!(wvuf_verify_result.is_ok(), "{:?}", wvuf_verify_result);
    }
}
