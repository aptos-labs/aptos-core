// Copyright © Aptos Foundation

use crate::{
    randomness::{
        decrypt_key_map, get_current_version, get_on_chain_resource, verify_dkg_transcript,
        verify_randomness,
    },
    smoke_test_environment::SwarmBuilder,
};
use aptos_forge::{NodeExt, Swarm, SwarmExt};
use aptos_logger::info;
use aptos_types::on_chain_config::DKGState;
use digest::Digest;
use std::{sync::Arc, time::Duration};

/// Verify the correctness of DKG transcript and WVUF output (i.e., block-level randomness seed).
///
/// Let `secret_agg` be the direct aggregation of all the input secrets sampled by the dealing validators.
/// Let `secret_recon` be the secret reconstructed from enough secret shares of the new validators.
/// DKG correctness: `secret_0 == secret_1`.
///
/// Let `output_eval` be the direct WVUF evaluation of the input using  `secret_1`.
/// Let `output_agg` be the aggregation of output shares from enough new validators. (This is what's on chain.)
/// WVUF correctness: `output_0 == output_1`.
#[tokio::test]
async fn randomness_correctness() {
    let epoch_duration_secs = 20;

    let (mut swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
        .with_num_fullnodes(1)
        .with_aptos()
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;
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

    /// Verify the randomness in 5 versions.
    for _ in 0..5 {
        let cur_txn_version = get_current_version(&rest_client).await;
        info!("Verifying WVUF output for version {}.", cur_txn_version);
        let wvuf_verify_result =
            verify_randomness(&decrypt_key_map, &rest_client, cur_txn_version).await;
        assert!(wvuf_verify_result.is_ok());
    }

    info!("Wait for epoch 3.");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(3, Duration::from_secs(epoch_duration_secs * 2))
        .await
        .expect("Epoch 2 taking too long to arrive!");

    info!("Verify DKG correctness for epoch 3.");
    let dkg_session = get_on_chain_resource::<DKGState>(&rest_client).await;
    assert!(verify_dkg_transcript(dkg_session.last_complete(), &decrypt_key_map).is_ok());

    /// Again, verify the randomness in 5 versions.
    for _ in 0..5 {
        let cur_txn_version = get_current_version(&rest_client).await;
        info!("Verifying WVUF output for version {}.", cur_txn_version);
        let wvuf_verify_result =
            verify_randomness(&decrypt_key_map, &rest_client, cur_txn_version).await;
        assert!(wvuf_verify_result.is_ok());
    }
}
