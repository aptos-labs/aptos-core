// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    randomness::{decrypt_key_map, script_to_update_consensus_config, verify_dkg_transcript},
    smoke_test_environment::SwarmBuilder,
    utils::{get_current_consensus_config, get_on_chain_resource},
};
use velor_forge::{Node, Swarm, SwarmExt};
use velor_logger::{debug, info};
use velor_types::{
    dkg::DKGState, on_chain_config::OnChainRandomnessConfig, randomness::PerBlockRandomness,
};
use std::{sync::Arc, time::Duration};

/// Disable on-chain randomness by only disabling validator transactions.
#[tokio::test]
async fn disable_feature_1() {
    let epoch_duration_secs = 20;

    let (swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
        .with_num_fullnodes(1)
        .with_velor()
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;
            conf.allow_new_validators = true;

            // Ensure randomness is enabled.
            conf.consensus_config.enable_validator_txns();
            conf.randomness_config_override = Some(OnChainRandomnessConfig::default_enabled());
        }))
        .build_with_cli(0)
        .await;

    let root_addr = swarm.chain_info().root_account().address();
    let root_idx = cli.add_account_with_address_to_cli(swarm.root_key(), root_addr);

    let decrypt_key_map = decrypt_key_map(&swarm);

    let client_endpoint = swarm.validators().nth(1).unwrap().rest_api_endpoint();
    let client = velor_rest_client::Client::new(client_endpoint.clone());

    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(3, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Waited too long for epoch 3.");

    info!("Now in epoch 3. Disabling validator transactions.");
    let mut config = get_current_consensus_config(&client).await;
    assert!(config.is_vtxn_enabled());
    config.disable_validator_txns();
    let disable_vtxn_script = script_to_update_consensus_config(&config);
    debug!("disable_vtxn_script={}", disable_vtxn_script);
    let txn_summary = cli
        .run_script(root_idx, disable_vtxn_script.as_str())
        .await
        .expect("Txn execution error.");
    debug!("disabling_vtxn_summary={:?}", txn_summary);

    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(4, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Waited too long for epoch 4.");

    info!("Now in epoch 4. DKG transcript should still be available. Randomness seed should be unavailable.");
    let dkg_session = get_on_chain_resource::<DKGState>(&client)
        .await
        .last_completed
        .expect("dkg result for epoch 4 should be present");
    assert_eq!(4, dkg_session.target_epoch());
    assert!(verify_dkg_transcript(&dkg_session, &decrypt_key_map).is_ok());

    let randomness_seed = get_on_chain_resource::<PerBlockRandomness>(&client).await;
    assert!(randomness_seed.seed.is_none());

    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(5, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Waited too long for epoch 5.");

    info!("Now in epoch 5. DKG transcript should be unavailable. Randomness seed should be unavailable.");
    let maybe_last_complete = get_on_chain_resource::<DKGState>(&client)
        .await
        .last_completed;
    assert!(
        maybe_last_complete.is_none() || maybe_last_complete.as_ref().unwrap().target_epoch() != 5
    );

    let randomness_seed = get_on_chain_resource::<PerBlockRandomness>(&client).await;
    assert!(randomness_seed.seed.is_none());
}
