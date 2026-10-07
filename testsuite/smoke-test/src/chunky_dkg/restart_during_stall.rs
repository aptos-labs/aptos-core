// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::shadow_mode::create_swarm_with_dkg_only;
use crate::utils::get_on_chain_resource;
use aptos_forge::{NodeExt, SwarmExt};
use aptos_logger::info;
use aptos_types::dkg::{chunky_dkg::ChunkyDKGState, DKGState};
use std::time::Duration;

/// Enable chunky DKG in V1 mode, with no shadow grace period and no epoch watchdog.
async fn enable_chunky_v1(cli: &aptos::test::CliTestFramework, root_idx: usize) {
    let script = r#"
script {
    use aptos_std::fixed_point64;
    use aptos_framework::aptos_governance;
    use aptos_framework::chunky_dkg_config;
    use aptos_framework::features;

    fun main(core_resources: &signer) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0x1);
        let chunky_cfg = chunky_dkg_config::new_v1(
            fixed_point64::create_from_rational(1, 2),
            fixed_point64::create_from_rational(2, 3),
        );
        chunky_dkg_config::set_for_next_epoch(&framework_signer, chunky_cfg);
        // ENCRYPTED_TRANSACTIONS feature flag (108), required for the chunky DKG path.
        features::change_feature_flags_for_next_epoch(&framework_signer, vector[108], vector[]);
        aptos_governance::reconfigure(&framework_signer);
    }
}
"#;
    cli.run_script(root_idx, script)
        .await
        .expect("Failed to enable chunky V1 via governance");
}

/// A validator restarts after the next epoch's randomness DKG completed, but before the epoch
/// changes, because the reconfiguration still waits for chunky DKG. The validator must load the
/// randomness keys of the current epoch. Before the fix, it started with randomness disabled,
/// executed blocks without randomness, diverged from the quorum, and crashed.
#[tokio::test]
async fn validator_restart_while_reconfig_waits_for_chunky_dkg() {
    let epoch_duration_secs = 20;
    let (mut swarm, cli, root_idx) = create_swarm_with_dkg_only(4, epoch_duration_secs).await;
    // Validator 0 restarts later, so read on-chain state through validator 1.
    let client = swarm.validators().nth(1).unwrap().rest_client();

    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(epoch_duration_secs * 3))
        .await
        .expect("Waited too long for epoch 2.");

    info!("Stalling chunky DKG on all validators.");
    for validator in swarm.validators() {
        validator
            .rest_client()
            .set_failpoint(
                "chunky_dkg::process_dkg_start_event".to_string(),
                "return".to_string(),
            )
            .await
            .expect("Failed to set failpoint");
    }
    enable_chunky_v1(&cli, root_idx).await;

    info!("Waiting until the next epoch's randomness DKG completes while chunky DKG is stalled.");
    let timer = tokio::time::Instant::now();
    let stuck_epoch = loop {
        assert!(
            timer.elapsed() < Duration::from_secs(epoch_duration_secs * 6),
            "The reconfiguration never waited on chunky DKG."
        );
        let epoch = client
            .get_ledger_information()
            .await
            .expect("ledger info")
            .into_inner()
            .epoch;
        let dkg_state = get_on_chain_resource::<DKGState>(&client).await;
        let chunky_dkg_state = get_on_chain_resource::<ChunkyDKGState>(&client).await;
        let next_epoch_dkg_completed = dkg_state
            .last_completed
            .as_ref()
            .is_some_and(|session| session.metadata.dealer_epoch == epoch);
        if next_epoch_dkg_completed && chunky_dkg_state.in_progress.is_some() {
            break epoch;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    };
    info!(
        "Epoch {} waits for chunky DKG. The next epoch's randomness DKG completed.",
        stuck_epoch
    );

    info!("Restarting validator 0 inside the window.");
    let node = swarm.validators_mut().next().unwrap();
    node.restart().await.expect("Failed to restart validator 0");

    // Without the fix, validator 0 executes blocks without randomness, diverges, and crashes.
    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(60))
        .await
        .expect("Validators did not catch up after the restart.");
    tokio::time::sleep(Duration::from_secs(30)).await;
    for validator in swarm.validators() {
        validator
            .health_check()
            .await
            .expect("A validator is unhealthy after the restart.");
    }
    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(60))
        .await
        .expect("Validators fell behind after the restart.");

    // The check above is only meaningful while the reconfiguration is still in progress.
    let epoch = client
        .get_ledger_information()
        .await
        .expect("ledger info")
        .into_inner()
        .epoch;
    assert_eq!(epoch, stuck_epoch, "The epoch changed during the test.");
}
