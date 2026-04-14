// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::{
    get_encryption_key_resource, verify_chunky_dkg_transcript, wait_for_chunky_dkg_finish,
};
use crate::{
    smoke_test_environment::SwarmBuilder, txn_emitter::generate_traffic,
    utils::get_on_chain_resource,
};
use aptos_forge::{EmitJobMode, Node, NodeExt, Swarm, SwarmExt, TransactionType};
use aptos_logger::info;
use aptos_types::{
    dkg::{chunky_dkg::ChunkyDKGState, DKGState},
    on_chain_config::{ChunkyDKGConfigMoveStruct, OnChainRandomnessConfig},
};
use std::{sync::Arc, time::Duration};

/// Create a swarm with randomness/DKG enabled but chunky DKG OFF.
/// Returns (swarm, cli, root_idx) for governance script execution.
async fn create_swarm_with_dkg_only(
    num_validators: usize,
    epoch_duration_secs: u64,
) -> (
    aptos_forge::LocalSwarm,
    aptos::test::CliTestFramework,
    usize,
) {
    let (swarm, mut cli, _faucet) = SwarmBuilder::new_local(num_validators)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| {
            config.api.failpoints_enabled = true;
            config.api.allow_encrypted_txns_submission = true;
            config.consensus.quorum_store.enable_batch_v2_tx = true;
            config.consensus.quorum_store.enable_batch_v2_rx = true;
            config.consensus.quorum_store.enable_opt_qs_v2_payload_tx = true;
            config.consensus.quorum_store.enable_opt_qs_v2_payload_rx = true;
            config
                .state_sync
                .state_sync_driver
                .enable_auto_bootstrapping = true;
            config
                .state_sync
                .state_sync_driver
                .max_connection_deadline_secs = 3;
        }))
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;
            conf.allow_new_validators = true;
            conf.consensus_config.enable_validator_txns();
            conf.randomness_config_override = Some(OnChainRandomnessConfig::default_enabled());
            // Chunky DKG config defaults to Off. ENCRYPTED_TRANSACTIONS not set.
        }))
        .build_with_cli(0)
        .await;

    let root_addr = swarm.chain_info().root_account().address();
    let root_idx = cli.add_account_with_address_to_cli(swarm.root_key(), root_addr);
    (swarm, cli, root_idx)
}

/// Enable shadow mode (ConfigShadowV1) via governance script.
async fn enable_shadow_mode(
    cli: &aptos::test::CliTestFramework,
    root_idx: usize,
    grace_period_secs: u64,
) {
    let script = format!(
        r#"
script {{
    use aptos_std::fixed_point64;
    use aptos_framework::aptos_governance;
    use aptos_framework::chunky_dkg_config;
    use aptos_framework::features;

    fun main(core_resources: &signer) {{
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0x1);

        let config = chunky_dkg_config::new_shadow_v1(
            fixed_point64::create_from_rational(1, 2),
            fixed_point64::create_from_rational(2, 3),
            {}
        );
        chunky_dkg_config::set_for_next_epoch(&framework_signer, config);

        features::change_feature_flags_for_next_epoch(&framework_signer, vector[108], vector[]);

        aptos_governance::reconfigure(&framework_signer);
    }}
}}
"#,
        grace_period_secs
    );
    cli.run_script(root_idx, &script)
        .await
        .expect("Failed to enable shadow mode via governance");
}

/// Upgrade from shadow mode to full ConfigV1 via governance script.
async fn upgrade_to_v1(cli: &aptos::test::CliTestFramework, root_idx: usize) {
    let script = r#"
script {
    use aptos_std::fixed_point64;
    use aptos_framework::aptos_governance;
    use aptos_framework::chunky_dkg_config;

    fun main(core_resources: &signer) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0x1);

        let config = chunky_dkg_config::new_v1(
            fixed_point64::create_from_rational(1, 2),
            fixed_point64::create_from_rational(2, 3)
        );
        chunky_dkg_config::set_for_next_epoch(&framework_signer, config);

        aptos_governance::reconfigure(&framework_signer);
    }
}
"#;
    cli.run_script(root_idx, script)
        .await
        .expect("Failed to upgrade to V1 via governance");
}

/// Test the full shadow mode lifecycle:
/// 1. Start with DKG only (chunky DKG OFF)
/// 2. Enable shadow mode via governance → chunky DKG runs, result stored on-chain
/// 3. Verify epochs advance (not blocked)
#[tokio::test]
async fn chunky_dkg_shadow_mode() {
    let epoch_duration_secs = 20;
    let estimated_dkg_latency_secs = 120;
    let grace_period_secs = 60;

    let (swarm, cli, root_idx) = create_swarm_with_dkg_only(4, epoch_duration_secs).await;
    let client_endpoint = swarm.validators().nth(1).unwrap().rest_api_endpoint();
    let client = aptos_rest_client::Client::new(client_endpoint);

    // Wait for epoch 2 so the network is stable.
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(epoch_duration_secs * 3))
        .await
        .expect("Waited too long for epoch 2.");

    // Verify chunky DKG has NOT completed (config is off).
    let chunky_dkg_state = get_on_chain_resource::<ChunkyDKGState>(&client).await;
    assert!(
        chunky_dkg_state.last_completed.is_none(),
        "Chunky DKG should not have completed with config off"
    );
    info!("Verified: no chunky DKG session with config off.");

    // Enable shadow mode via governance.
    info!("Enabling shadow mode (ConfigShadowV1) via governance...");
    enable_shadow_mode(&cli, root_idx, grace_period_secs).await;

    // Poll until chunky DKG completes.
    info!("Polling for chunky DKG completion in shadow mode...");
    let timer = tokio::time::Instant::now();
    let session = loop {
        let ledger = client
            .get_ledger_information()
            .await
            .expect("ledger info")
            .into_inner();
        let dkg_state = get_on_chain_resource::<ChunkyDKGState>(&client).await;
        let regular_dkg = get_on_chain_resource::<DKGState>(&client).await;
        let config = get_on_chain_resource::<ChunkyDKGConfigMoveStruct>(&client).await;
        info!(
            "epoch={} version={} chunky_in_progress={} chunky_completed={} regular_in_progress={} config={:?} elapsed={}s",
            ledger.epoch,
            ledger.version,
            dkg_state.in_progress.is_some(),
            dkg_state.last_completed.is_some(),
            regular_dkg.in_progress.is_some(),
            config,
            timer.elapsed().as_secs(),
        );
        if dkg_state.last_completed.is_some() {
            info!("Shadow chunky DKG completed!");
            break dkg_state.last_complete().clone();
        }
        if timer.elapsed().as_secs() > estimated_dkg_latency_secs {
            panic!(
                "Timed out waiting for shadow chunky DKG (epoch={}, in_progress={})",
                ledger.epoch,
                dkg_state.in_progress.is_some(),
            );
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    };

    // Verify transcript.
    let subtranscript = verify_chunky_dkg_transcript(&session);
    assert!(
        !subtranscript.dealers.is_empty(),
        "Shadow DKG should produce a transcript with dealers"
    );
    info!(
        "Shadow chunky DKG completed for epoch {} with {} dealers",
        session.target_epoch(),
        subtranscript.dealers.len()
    );
}

/// Test transitioning from shadow mode to full V1:
/// 1. Start with DKG only (chunky DKG OFF)
/// 2. Enable shadow mode → chunky DKG runs
/// 3. Upgrade to V1 → encryption key should now be present
#[tokio::test]
async fn chunky_dkg_shadow_to_v1() {
    let epoch_duration_secs = 20;
    let estimated_dkg_latency_secs = 120;
    let grace_period_secs = 60;

    let (mut swarm, cli, root_idx) = create_swarm_with_dkg_only(4, epoch_duration_secs).await;
    let client_endpoint = swarm.validators().nth(1).unwrap().rest_api_endpoint();
    let client = aptos_rest_client::Client::new(client_endpoint);

    // Wait for epoch 2 so the network is stable.
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(epoch_duration_secs * 3))
        .await
        .expect("Waited too long for epoch 2.");

    // ---- Phase 1: Enable shadow mode ----
    info!("Phase 1: Enabling shadow mode...");
    enable_shadow_mode(&cli, root_idx, grace_period_secs).await;

    // Wait for shadow chunky DKG to complete.
    let shadow_session =
        wait_for_chunky_dkg_finish(&client, None, estimated_dkg_latency_secs).await;
    let shadow_epoch = shadow_session.target_epoch();
    info!("Shadow chunky DKG completed for epoch {}", shadow_epoch);

    // Verify transcript is valid.
    verify_chunky_dkg_transcript(&shadow_session);

    // ---- Phase 2: Upgrade to V1 ----
    info!("Phase 2: Upgrading from shadow to V1...");
    upgrade_to_v1(&cli, root_idx).await;

    // Wait for the next chunky DKG to complete under V1.
    info!("Waiting for chunky DKG under V1...");
    let timer = tokio::time::Instant::now();
    let v1_session = loop {
        let dkg_state = get_on_chain_resource::<ChunkyDKGState>(&client).await;
        if let Some(ref completed) = dkg_state.last_completed {
            if completed.target_epoch() > shadow_epoch {
                info!(
                    "V1 chunky DKG completed for epoch {}",
                    completed.target_epoch()
                );
                break completed.clone();
            }
        }
        if timer.elapsed().as_secs() > estimated_dkg_latency_secs {
            panic!("Timed out waiting for V1 chunky DKG");
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    };

    // Verify transcript under V1.
    verify_chunky_dkg_transcript(&v1_session);

    // Verify encryption key is now present (V1 enables real decryption).
    let enc_key = get_encryption_key_resource(&client).await;
    assert!(
        enc_key.encryption_key.is_some(),
        "Encryption key should be present after upgrading to V1"
    );
    info!(
        "V1 encryption key present at epoch {} ({} bytes)",
        enc_key.epoch,
        enc_key.encryption_key.as_ref().unwrap().len()
    );

    // ---- Phase 3: Verify encrypted transactions work under V1 ----
    info!("Phase 3: Emitting encrypted traffic under V1...");
    let all_validators: Vec<_> = swarm.validators().map(|v| v.peer_id()).collect();
    let stats = generate_traffic(
        &mut swarm,
        &all_validators,
        Duration::from_secs(20),
        100,
        vec![vec![(TransactionType::default(), 1)]],
        true,
        Some(EmitJobMode::MaxLoad {
            mempool_backlog: 20,
        }),
    )
    .await
    .unwrap();
    info!(
        "Encrypted traffic stats: submitted={}, committed={}",
        stats.submitted, stats.committed
    );
    assert!(
        stats.committed > 0,
        "Expected committed encrypted transactions after V1 upgrade"
    );
    info!("Shadow → V1 transition complete: encrypted transactions working");
}

/// Test that the grace period safety net fires when chunky DKG fails.
/// Uses a failpoint to prevent chunky DKG from processing the start event,
/// simulating a stuck chunky DKG. Regular DKG completes normally.
/// The grace period should force the epoch change.
#[tokio::test]
async fn chunky_dkg_shadow_mode_grace_period_failpoint() {
    let epoch_duration_secs = 20;
    let grace_period_secs = 30;

    let (swarm, cli, root_idx) = create_swarm_with_dkg_only(4, epoch_duration_secs).await;
    let client_endpoint = swarm.validators().nth(1).unwrap().rest_api_endpoint();
    let client = aptos_rest_client::Client::new(client_endpoint);

    // Wait for epoch 2 so the network is stable.
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(epoch_duration_secs * 3))
        .await
        .expect("Waited too long for epoch 2.");

    // Activate failpoint on all validators to block chunky DKG from processing start events.
    info!("Activating chunky_dkg::process_dkg_start_event failpoint on all validators...");
    let validator_clients: Vec<_> = swarm.validators().map(|v| v.rest_client()).collect();
    for client in &validator_clients {
        client
            .set_failpoint(
                "chunky_dkg::process_dkg_start_event".to_string(),
                "return".to_string(),
            )
            .await
            .expect("Failed to set failpoint");
    }

    // Enable shadow mode with the grace period.
    info!(
        "Enabling shadow mode (grace_period={}s)...",
        grace_period_secs
    );
    enable_shadow_mode(&cli, root_idx, grace_period_secs).await;

    // Wait for the epoch to advance. Regular DKG will complete, but chunky DKG is stuck.
    // The grace period safety net should force the epoch change.
    let time_limit_secs = epoch_duration_secs + grace_period_secs + 60; // epoch + grace + buffer
    let timer = tokio::time::Instant::now();
    let mut epoch_after_shadow = None;

    info!("Waiting for epoch to advance despite stuck chunky DKG...");
    while timer.elapsed().as_secs() < time_limit_secs {
        let ledger = client
            .get_ledger_information()
            .await
            .expect("ledger info")
            .into_inner();
        let dkg_state = get_on_chain_resource::<ChunkyDKGState>(&client).await;
        info!(
            "epoch={} chunky_in_progress={} chunky_completed={} elapsed={}s",
            ledger.epoch,
            dkg_state.in_progress.is_some(),
            dkg_state.last_completed.is_some(),
            timer.elapsed().as_secs(),
        );

        // The governance script triggers an immediate reconfig.
        // After that epoch, the next epoch boundary should trigger shadow mode.
        // We need at least one more epoch change where the grace period fires.
        if ledger.epoch >= 4 {
            epoch_after_shadow = Some(ledger.epoch);
            break;
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }

    assert!(
        epoch_after_shadow.is_some(),
        "Epoch should have advanced despite chunky DKG being stuck (grace period should fire)"
    );
    info!(
        "Grace period safety net worked: epoch advanced to {} with chunky DKG stuck",
        epoch_after_shadow.unwrap()
    );

    // Verify chunky DKG did NOT complete (failpoint prevented it).
    let dkg_state = get_on_chain_resource::<ChunkyDKGState>(&client).await;
    assert!(
        dkg_state.in_progress.is_none(),
        "Chunky DKG in_progress should have been cleared by grace period"
    );
    info!("Confirmed: chunky DKG session was cleared by the grace period safety net");
}
