// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::{
    create_swarm_with_chunky_dkg, verify_chunky_dkg_transcript, wait_for_chunky_dkg_finish,
};
use aptos_forge::NodeExt;
use aptos_logger::info;

/// Test that chunky DKG completes even with one validator down (BFT: f=1 with 4 validators).
#[tokio::test]
async fn chunky_dkg_with_validator_down() {
    let epoch_duration_secs = 10;
    let estimated_dkg_latency_secs = 20;
    let time_limit_secs = epoch_duration_secs + estimated_dkg_latency_secs;

    let mut swarm = create_swarm_with_chunky_dkg(4, epoch_duration_secs).await;
    let client = swarm.validators().last().unwrap().rest_client();

    // Wait for initial DKG completion
    info!("Waiting for initial chunky DKG to complete...");
    let session_1 = wait_for_chunky_dkg_finish(&client, None, time_limit_secs).await;
    info!(
        "Initial chunky DKG completed for epoch {}",
        session_1.target_epoch()
    );

    // Stop one validator
    info!("Taking one validator down...");
    swarm.validators_mut().take(1).for_each(|v| {
        v.stop();
    });

    // Wait for next DKG session to complete with 3/4 validators
    info!(
        "Waiting for chunky DKG to complete for epoch {} with one validator down...",
        session_1.target_epoch() + 1
    );
    let session_2 =
        wait_for_chunky_dkg_finish(&client, Some(session_1.target_epoch() + 1), time_limit_secs)
            .await;
    info!(
        "Chunky DKG completed for epoch {} with 3/4 validators",
        session_2.target_epoch()
    );

    // Verify the transcript is valid even with a validator down
    let subtranscript = verify_chunky_dkg_transcript(&session_2);
    assert!(
        !subtranscript.dealers.is_empty(),
        "Transcript should have dealers even with one validator down"
    );
    info!(
        "Transcript verified successfully with {} dealers",
        subtranscript.dealers.len()
    );
}
