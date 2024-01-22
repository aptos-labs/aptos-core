// Copyright © Aptos Foundation

use crate::{
    randomness::{decrypt_key_map, verify_dkg_transcript, wait_for_dkg_finish},
    smoke_test_environment::SwarmBuilder,
};
use aptos_forge::{NodeExt, SwarmExt};
use std::{sync::Arc, time::Duration};

#[tokio::test]
async fn dkg_basic() {
    let epoch_duration_secs = 10;
    let estimated_dkg_latency_secs = 20;
    let time_limit_secs = epoch_duration_secs + estimated_dkg_latency_secs;

    let swarm = SwarmBuilder::new_local(4)
        .with_num_fullnodes(1)
        .with_aptos()
        .with_init_genesis_config(Arc::new(|conf| {
            conf.epoch_duration_secs = 10;
        }))
        .build()
        .await;

    let client = swarm.validators().next().unwrap().rest_client();
    println!("Wait for a moment when DKG is not running.");
    let dkg_session = wait_for_dkg_finish(&client, None, time_limit_secs).await;
    println!("dkg_session={:?}", dkg_session);
    let decrypt_key_map = decrypt_key_map(&swarm);
    assert!(verify_dkg_transcript(&dkg_session, &decrypt_key_map).is_ok());
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(4, Duration::from_secs(epoch_duration_secs * 10))
        .await
        .unwrap();
}
