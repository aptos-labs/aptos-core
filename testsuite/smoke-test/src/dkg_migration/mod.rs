// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Smoke test for the DKG migration feature.
//!
//! # Security context
//! In the legacy (end-of-epoch) DKG model, the dealer set for epoch X+1's DKG
//! is the *predicted* validator set assembled at the end of epoch X. Because
//! staking rewards and fees are generated during the DKG window, the actual
//! epoch X+1 validator set can diverge from the predicted set, causing incorrect
//! secret-share indices.
//!
//! The fix moves DKG to the *beginning* of epoch X+1 (non-blocking mode): after
//! reconfiguration has committed the finalized validator set, a single-txn
//! epoch transition occurs immediately, and the non-blocking DKG runs during epoch
//! X+1 using the correct, final validator set.  While the DKG is in progress,
//! randomness-requiring transactions abort and the encrypted mempool is
//! unavailable.  Both DKGs (randomness + Chunky) are independent; each feature
//! activates as soon as its own DKG completes.
//!
//! # Test structure (TDD)
//! This test is written *before* the underlying implementation exists.  It will
//! compile but fail at runtime until:
//!   - `features::get_dkg_non_blocking_feature()` is added to the Move framework, and
//!   - the corresponding `DKG_NON_BLOCKING = 110` feature flag is wired in Rust.
//!
//! Phases:
//!   1. Verify legacy (end-of-epoch) DKG still works.
//!   2. Governance script: switch to non-blocking mode via feature flag.
//!   3. Verify features are unavailable while DKG is in progress.
//!   4. Randomness DKG completes independently.
//!   5. Chunky DKG completes independently.
//!   6. Epoch 4→5 transition without end-of-epoch DKG.
//!   7. Short epochs: DKG abandoned mid-epoch without chain stall.

use crate::{
    randomness::e2e_basic_consumption::publish_on_chain_dice_module,
    smoke_test_environment::SwarmBuilder,
    utils::get_on_chain_resource,
};
use aptos::{common::types::GasOptions, test::CliTestFramework};
use aptos_forge::{NodeExt, Swarm, SwarmExt};
use aptos_logger::info;
use aptos_move_cli::MemberId;
use aptos_rest_client::Client;
use aptos_types::{
    dkg::{
        chunky_dkg::{ChunkyDKGSessionState, ChunkyDKGState},
        DKGSessionState, DKGState,
    },
    on_chain_config::{
        FeatureFlag, Features, OnChainChunkyDKGConfig, OnChainRandomnessConfig,
    },
};
use std::{str::FromStr, sync::Arc, time::Duration};
use tokio::time::Instant;

// ---------------------------------------------------------------------------
// Helper: wait until the randomness DKG for `dealer_epoch` finishes
// (non-blocking mode semantics: `last_completed.dealer_epoch == dealer_epoch`).
// ---------------------------------------------------------------------------
#[allow(dead_code)]
async fn wait_for_dkg_finish_at_epoch(
    client: &Client,
    dealer_epoch: u64,
    time_limit_secs: u64,
) -> DKGSessionState {
    let timer = Instant::now();
    loop {
        assert!(
            timer.elapsed().as_secs() < time_limit_secs,
            "Timed out waiting for non-blocking randomness DKG to finish (dealer_epoch={dealer_epoch})",
        );
        let dkg_state = get_on_chain_resource::<DKGState>(client).await;
        let done = dkg_state.in_progress.is_none()
            && dkg_state
                .last_completed
                .as_ref()
                .map(|s| s.metadata.dealer_epoch == dealer_epoch)
                .unwrap_or(false);
        if done {
            return dkg_state.last_complete().clone();
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

// ---------------------------------------------------------------------------
// Helper: wait until the chunky DKG for `dealer_epoch` finishes.
// ---------------------------------------------------------------------------
#[allow(dead_code)]
async fn wait_for_chunky_dkg_finish_at_epoch(
    client: &Client,
    dealer_epoch: u64,
    time_limit_secs: u64,
) -> ChunkyDKGSessionState {
    let timer = Instant::now();
    loop {
        assert!(
            timer.elapsed().as_secs() < time_limit_secs,
            "Timed out waiting for non-blocking chunky DKG to finish (dealer_epoch={dealer_epoch})",
        );
        let state = get_on_chain_resource::<ChunkyDKGState>(client).await;
        // In non-blocking mode the session's `dealer_epoch` equals the current epoch.
        let done = state.in_progress.is_none()
            && state
                .last_completed
                .as_ref()
                .map(|s| s.metadata.dealer_epoch == dealer_epoch)
                .unwrap_or(false);
        if done {
            return state.last_complete().clone();
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

// ---------------------------------------------------------------------------
// Helper: assert that a non-blocking DKG is in progress for `expected_epoch`.
// In non-blocking mode, `in_progress.dealer_epoch == current_epoch` (not current-1).
// ---------------------------------------------------------------------------
#[allow(dead_code)]
async fn assert_dkg_started_at_epoch(client: &Client, expected_epoch: u64) {
    let dkg_state = get_on_chain_resource::<DKGState>(client).await;
    assert!(
        dkg_state.in_progress.is_some(),
        "Expected a non-blocking DKG to be in progress for epoch {expected_epoch}, but none found"
    );
    let in_progress = dkg_state.in_progress.as_ref().unwrap();
    assert_eq!(
        in_progress.metadata.dealer_epoch, expected_epoch,
        "Non-blocking DKG in_progress.dealer_epoch should equal the current epoch {expected_epoch}"
    );
}

// ---------------------------------------------------------------------------
// Helper: submit a randomness-consuming transaction and return whether it
// succeeded.  Uses the `dice::roll` entry function from `on_chain_dice`.
// ---------------------------------------------------------------------------
#[allow(dead_code)]
async fn try_randomness_txn(cli: &mut CliTestFramework, account_idx: usize) -> bool {
    let account = cli.account_id(account_idx).to_hex_literal();
    let roll_func_id = MemberId::from_str(&format!("{}::dice::roll", account)).unwrap();
    let gas_options = GasOptions {
        gas_unit_price: Some(100),
        max_gas: Some(10_000),
        expiration_secs: 60,
    };
    cli.run_function(account_idx, Some(gas_options), roll_func_id, vec![], vec![])
        .await
        .is_ok()
}

// ---------------------------------------------------------------------------
// Helper: return the current ledger version.
// ---------------------------------------------------------------------------
async fn get_current_version(client: &Client) -> u64 {
    client
        .get_ledger_information()
        .await
        .unwrap()
        .inner()
        .version
}

// ===========================================================================
// Main test
// ===========================================================================

/// End-to-end smoke test for the DKG migration (legacy → non-blocking mode).
///
/// NOTE (TDD): this test will fail at runtime in Phase 2 until
/// `features::get_dkg_non_blocking_feature()` is implemented in the Move
/// framework and `DKG_NON_BLOCKING = 110` is wired in Rust.
#[tokio::test]
async fn dkg_migration() {
    let epoch_duration_secs: u64 = 60;

    // -----------------------------------------------------------------------
    // Setup: 4-validator local swarm with randomness + chunky DKG enabled.
    // DKG mode defaults to the legacy (end-of-epoch) mode.
    // -----------------------------------------------------------------------
    let (swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
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
            conf.chunky_dkg_config_override = Some(OnChainChunkyDKGConfig::default_enabled());
            // ENCRYPTED_TRANSACTIONS feature flag for chunky DKG.
            let mut features = Features::default();
            features.enable(FeatureFlag::ENCRYPTED_TRANSACTIONS);
            conf.initial_features_override = Some(features);
            // DKG mode defaults to legacy (end-of-epoch DKG).
        }))
        .build_with_cli(0)
        .await;

    let root_addr = swarm.chain_info().root_account().address();
    let root_idx = cli.add_account_with_address_to_cli(swarm.root_key(), root_addr);

    let client = swarm.validators().nth(1).unwrap().rest_client();

    // -----------------------------------------------------------------------
    // Phase 1: Verify legacy (end-of-epoch) DKG works.
    //
    // The first DKG runs at the end of epoch 2 to provide keys for epoch 3.
    // Epochs 1 and 2 have no randomness available.
    // -----------------------------------------------------------------------
    info!("Phase 1: waiting for epoch 2 (network stable).");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(epoch_duration_secs * 3))
        .await
        .expect("epoch 2 taking too long");

    // Publish the on-chain-dice module so we can test randomness consumption.
    info!("Phase 1: publishing on-chain-dice module.");
    publish_on_chain_dice_module(&mut cli, root_idx).await;

    info!("Phase 1: waiting for epoch 3 (legacy DKG ran at end of epoch 2).");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(3, Duration::from_secs(epoch_duration_secs + 120))
        .await
        .expect("epoch 3 taking too long — legacy DKG may have stalled");

    // Legacy mode assertion: dealer_epoch == previous epoch (2), target_epoch == 3.
    let dkg_state = get_on_chain_resource::<DKGState>(&client).await;
    assert!(
        dkg_state.last_completed.is_some(),
        "Phase 1: DKGState.last_completed should be present in epoch 3"
    );
    let legacy_session = dkg_state.last_complete();
    assert_eq!(
        legacy_session.metadata.dealer_epoch, 2,
        "Phase 1 (legacy): dealer_epoch should be 2 (previous epoch)"
    );
    assert_eq!(
        legacy_session.target_epoch(),
        3,
        "Phase 1 (legacy): target_epoch should be 3"
    );
    info!(
        "Phase 1: legacy DKG verified — dealer_epoch={}, target_epoch={}",
        legacy_session.metadata.dealer_epoch,
        legacy_session.target_epoch()
    );

    // Chunky DKG should have completed for epoch 3 as well.
    let chunky_state = get_on_chain_resource::<ChunkyDKGState>(&client).await;
    assert!(
        chunky_state.last_completed.is_some(),
        "Phase 1: ChunkyDKGState.last_completed should be present in epoch 3"
    );
    assert_eq!(
        chunky_state.last_complete().target_epoch(),
        3,
        "Phase 1 (legacy chunky): target_epoch should be 3"
    );

    // Submit a randomness-required transaction — should succeed in epoch 3.
    let roll_succeeded = try_randomness_txn(&mut cli, root_idx).await;
    assert!(
        roll_succeeded,
        "Phase 1: randomness txn (dice::roll) should succeed in epoch 3"
    );
    info!("Phase 1: randomness txn succeeded in epoch 3. Legacy mode verified.");

    // -----------------------------------------------------------------------
    // Phase 2: Governance proposal — switch to non-blocking DKG mode via
    // feature flag.
    //
    // NOTE (TDD): this script will fail at runtime until
    // `features::get_dkg_non_blocking_feature()` is implemented in the Move
    // framework.
    // -----------------------------------------------------------------------
    info!("Phase 2: submitting governance script to enable non-blocking DKG mode.");
    let non_blocking_mode_script = r#"
script {
    use aptos_framework::aptos_governance;
    use std::features;

    fun main(core_resources: &signer) {
        let fw = aptos_governance::get_signer_testnet_only(
            core_resources, @0x1);
        features::change_feature_flags_for_next_epoch(
            &fw,
            vector[features::get_dkg_non_blocking_feature()],
            vector[]
        );
        aptos_governance::force_end_epoch_test_only(&fw);
    }
}
"#;
    cli.run_script(root_idx, non_blocking_mode_script)
        .await
        .expect("governance script to enable non-blocking DKG mode failed — get_dkg_non_blocking_feature() may not be implemented yet");

    // Wait for epoch 4 (first epoch in non-blocking mode).
    info!("Phase 2: waiting for epoch 4 (first non-blocking-mode epoch).");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(4, Duration::from_secs(30))
        .await
        .expect("epoch 4 taking too long");

    // Epoch 4 should have started with an in-progress non-blocking DKG.
    // In non-blocking mode: dealer_epoch == current epoch (4), not 3.
    let dkg_state_ep4 = get_on_chain_resource::<DKGState>(&client).await;
    if let Some(ref in_progress) = dkg_state_ep4.in_progress {
        assert_eq!(
            in_progress.metadata.dealer_epoch, 4,
            "Phase 2: non-blocking DKG in_progress.dealer_epoch must equal current epoch 4"
        );
        info!(
            "Phase 2: non-blocking DKG started — in_progress.dealer_epoch={}",
            in_progress.metadata.dealer_epoch
        );
    } else {
        // The DKG may have already completed in fast environments.
        // Check that it completed with dealer_epoch == 4.
        if let Some(ref completed) = dkg_state_ep4.last_completed {
            assert_eq!(
                completed.metadata.dealer_epoch, 4,
                "Phase 2: non-blocking DKG last_completed.dealer_epoch must be 4"
            );
        }
        info!("Phase 2: non-blocking DKG already completed by the time we checked.");
    }

    // The last *completed* session should still be from legacy epoch 2
    // (the non-blocking DKG for epoch 4 is only in_progress, not done yet).
    if dkg_state_ep4.in_progress.is_some() {
        let last_completed = dkg_state_ep4
            .last_completed
            .as_ref()
            .expect("Phase 2: last_completed should still be present from legacy mode");
        assert_eq!(
            last_completed.metadata.dealer_epoch, 2,
            "Phase 2: last_completed.dealer_epoch should still be 2 (legacy) while non-blocking DKG is in progress"
        );
    }

    // -----------------------------------------------------------------------
    // Phase 3: Verify features are unavailable while DKG is in progress.
    // -----------------------------------------------------------------------
    let dkg_state_phase3 = get_on_chain_resource::<DKGState>(&client).await;
    if dkg_state_phase3.in_progress.is_some() {
        info!("Phase 3: DKG still in progress — verifying feature unavailability.");

        // Randomness-requiring txn should abort while DKG is in progress.
        let roll_failed = !try_randomness_txn(&mut cli, root_idx).await;
        assert!(
            roll_failed,
            "Phase 3: randomness txn (dice::roll) should fail/abort while DKG is in progress"
        );
        info!("Phase 3: randomness txn correctly aborted during DKG period.");

        // Chain must still make progress (blocks committed) despite no randomness.
        let v_before = get_current_version(&client).await;
        tokio::time::sleep(Duration::from_secs(5)).await;
        let v_after = get_current_version(&client).await;
        assert!(
            v_after > v_before,
            "Phase 3: chain must progress without randomness — before={v_before}, after={v_after}"
        );
        info!("Phase 3: chain progressed from v{v_before} to v{v_after} without randomness.");
    } else {
        info!("Phase 3: DKG completed before phase 3 checks — skipping unavailability assertions.");
    }

    // -----------------------------------------------------------------------
    // Phase 4: Randomness DKG completes.
    //
    // Poll until last_completed.dealer_epoch == 4.  In non-blocking mode the
    // dealer set equals the target set (same epoch).
    // -----------------------------------------------------------------------
    info!("Phase 4: waiting for non-blocking randomness DKG to finish (dealer_epoch=4).");
    let dkg_session = wait_for_dkg_finish_at_epoch(&client, 4, 120).await;

    assert_eq!(
        dkg_session.metadata.dealer_epoch, 4,
        "Phase 4: DKG session dealer_epoch must be 4 (current epoch) in non-blocking mode"
    );
    assert_eq!(
        dkg_session.metadata.dealer_validator_set,
        dkg_session.metadata.target_validator_set,
        "Phase 4: in non-blocking mode dealer_validator_set must equal target_validator_set"
    );
    info!(
        "Phase 4: randomness DKG completed — dealer_epoch={}, validators={}",
        dkg_session.metadata.dealer_epoch,
        dkg_session.metadata.target_validator_set.len()
    );

    // Randomness-requiring txn should succeed now.
    let roll_ok = try_randomness_txn(&mut cli, root_idx).await;
    assert!(
        roll_ok,
        "Phase 4: randomness txn (dice::roll) should succeed after DKG completes"
    );
    info!("Phase 4: randomness txn succeeded after DKG completion.");

    // -----------------------------------------------------------------------
    // Phase 5: Chunky DKG completes.
    // -----------------------------------------------------------------------
    info!("Phase 5: waiting for non-blocking chunky DKG to finish (dealer_epoch=4).");
    let chunky_session = wait_for_chunky_dkg_finish_at_epoch(&client, 4, 120).await;

    assert_eq!(
        chunky_session.metadata.dealer_epoch, 4,
        "Phase 5: chunky DKG session dealer_epoch must be 4 in non-blocking mode"
    );
    info!(
        "Phase 5: chunky DKG completed — dealer_epoch={}",
        chunky_session.metadata.dealer_epoch
    );

    // -----------------------------------------------------------------------
    // Phase 6: Epoch 4 → 5 transition without end-of-epoch DKG.
    //
    // In non-blocking mode there is no DKG gating the epoch boundary, so the
    // transition should be nearly instant (< 5 seconds after epoch_interval).
    // -----------------------------------------------------------------------
    info!("Phase 6: waiting for epoch 5 (non-blocking mode — no end-of-epoch DKG).");
    let transition_start = Instant::now();
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(5, Duration::from_secs(90))
        .await
        .expect("epoch 5 taking too long");
    let transition_elapsed = transition_start.elapsed().as_secs();
    info!("Phase 6: epoch 4→5 transition took {transition_elapsed}s");

    // The transition should have been fast — no DKG gating.
    // We allow a generous 30-second window on top of the epoch boundary to
    // account for slow CI environments, but it must be much less than a full
    // additional epoch (60s).
    assert!(
        transition_elapsed < 30,
        "Phase 6: epoch 4→5 transition took {transition_elapsed}s — expected < 30s for non-blocking mode (no end-of-epoch DKG)"
    );

    // Immediately after epoch 5 starts, a new non-blocking DKG should be in flight.
    let dkg_ep5 = get_on_chain_resource::<DKGState>(&client).await;
    if let Some(ref in_prog) = dkg_ep5.in_progress {
        assert_eq!(
            in_prog.metadata.dealer_epoch, 5,
            "Phase 6: new non-blocking DKG in_progress.dealer_epoch must be 5"
        );
        info!(
            "Phase 6: new non-blocking DKG started for epoch 5 — dealer_epoch={}",
            in_prog.metadata.dealer_epoch
        );
    }

    // -----------------------------------------------------------------------
    // Phase 7: Short epochs — DKG abandoned mid-epoch without chain stall.
    //
    // Set epoch_interval = 5 seconds (shorter than DKG completion time).
    // The chain should advance through multiple epochs cleanly, abandoning
    // each in-progress DKG at each epoch boundary.
    // -----------------------------------------------------------------------
    info!("Phase 7: setting epoch_interval to 5 seconds for fast-abandon test.");
    let short_epoch_script = r#"
script {
    use aptos_framework::aptos_governance;
    use aptos_framework::block;

    fun main(core_resources: &signer) {
        let fw = aptos_governance::get_signer_testnet_only(core_resources, @0x1);
        // 5-second epochs — shorter than DKG completion time.
        block::update_epoch_interval_microsecs(&fw, 5_000_000);
        aptos_governance::force_end_epoch_test_only(&fw);
    }
}
"#;
    cli.run_script(root_idx, short_epoch_script)
        .await
        .expect("short epoch governance script failed");

    // Epochs 6, 7, 8 must all transition quickly without waiting for DKG.
    for target_epoch in [6u64, 7, 8] {
        info!("Phase 7: waiting for epoch {target_epoch} (short epoch, DKG abandoned).");
        swarm
            .wait_for_all_nodes_to_catchup_to_epoch(target_epoch, Duration::from_secs(30))
            .await
            .unwrap_or_else(|_| {
                panic!("Phase 7: epoch {target_epoch} taking too long — chain stalled on abandoned DKG")
            });
        info!("Phase 7: reached epoch {target_epoch}.");
    }
    info!("Phase 7: chain advanced through epochs 6, 7, 8 without stalling. Non-blocking DKG abandonment works correctly.");
}
