// Copyright © Aptos Foundation

use crate::{
    dkg::{decrypt_key_map, verify_dkg_transcript, wait_for_epoch_fully_entered},
    smoke_test_environment::SwarmBuilder,
};
use aptos_forge::NodeExt;
use std::sync::Arc;

#[tokio::test]
async fn dkg_basic() {
    let epoch_duration_secs = 10;
    let estimated_dkg_latency_secs = 20;
    let time_limit_secs = epoch_duration_secs + estimated_dkg_latency_secs;

    let swarm = SwarmBuilder::new_local(4)
        .with_aptos()
        .with_init_genesis_config(Arc::new(|conf| {
            conf.epoch_duration_secs = 10;
        }))
        .build()
        .await;

    let client = swarm.validators().next().unwrap().rest_client();
    println!("Wait for a moment when DKG is not running.");
    let dkg_state_1 = wait_for_epoch_fully_entered(&client, None, time_limit_secs).await;

    println!("Current epoch is {}.", dkg_state_1.target_epoch);
    println!(
        "Waiting until we fully entered epoch {}.",
        dkg_state_1.target_epoch + 1
    );

    let dkg_state_2 =
        wait_for_epoch_fully_entered(&client, Some(dkg_state_1.target_epoch + 1), time_limit_secs)
            .await;
    println!(
        "Verifying the transcript generated for epoch {} by epoch {}.",
        dkg_state_2.target_epoch, dkg_state_1.target_epoch
    );

    let decrypt_key_map = decrypt_key_map(&swarm);
    assert!(verify_dkg_transcript(
        &dkg_state_1,
        &dkg_state_2,
        &decrypt_key_map
    ));
}
