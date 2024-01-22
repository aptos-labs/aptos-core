// Copyright © Aptos Foundation

use crate::{
    randomness::{decrypt_key_map, get_on_chain_resource,
    },
    smoke_test_environment::SwarmBuilder,
};
use aptos_forge::{NodeExt, Swarm, SwarmExt};
use aptos_logger::info;
use digest::Digest;
use std::{sync::Arc, time::Duration};
use aptos_types::dkg::DKGState;
use crate::randomness::{get_current_version, verify_dkg_transcript};

/// Verify the correctness of DKG transcript and block-level randomness seed.
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

    //TODO: verify randomness seed.
}
