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
//!   3. Concurrently: randomness DKG completes → dice::roll works;
//!      chunky DKG completes → encrypted mempool e2e works.

use crate::{
    randomness::e2e_basic_consumption::publish_on_chain_dice_module,
    smoke_test_environment::SwarmBuilder,
    txn_emitter::generate_traffic,
    utils::get_on_chain_resource,
};
use aptos::{common::types::GasOptions, test::CliTestFramework};
use aptos_forge::{EmitJobMode, NodeExt, Swarm, SwarmExt, TransactionType};
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
// Helper: count encrypted and decrypted user transactions in [start, end).
// ---------------------------------------------------------------------------
async fn count_encrypted_txns(client: &Client, start_version: u64, end_version: u64) -> (u64, u64) {
    let mut count = 0u64;
    let mut decrypted_count = 0u64;
    let page_size = 100u16;
    let mut cursor = start_version;
    while cursor < end_version {
        let limit = std::cmp::min(page_size as u64, end_version - cursor) as u16;
        let txns = client
            .get_transactions_bcs(Some(cursor), Some(limit))
            .await
            .expect("failed to get transactions")
            .into_inner();
        for txn_data in &txns {
            if let Some(signed_txn) = txn_data.transaction.try_as_signed_user_txn() {
                if let Some(payload) = signed_txn.payload().as_encrypted_payload() {
                    count += 1;
                    if !payload.is_encrypted() {
                        decrypted_count += 1;
                    }
                }
            }
        }
        cursor += txns.len() as u64;
        if txns.is_empty() {
            break;
        }
    }
    (count, decrypted_count)
}

// ===========================================================================
// Main test
// ===========================================================================

/// End-to-end smoke test for the DKG migration (legacy → non-blocking mode).
///
/// NOTE (TDD): this test will fail at runtime in Phase 2 until
/// `features::get_dkg_non_blocking_feature()` is implemented in the Move
/// framework and `DKG_NON_BLOCKING = 110` is wired in Rust.
#[ignore]
#[tokio::test]
async fn dkg_migration() {
    let epoch_duration_secs: u64 = 60;

    // -----------------------------------------------------------------------
    // Setup: 4-validator local swarm with randomness + chunky DKG enabled.
    // DKG mode defaults to the legacy (end-of-epoch) mode.
    // -----------------------------------------------------------------------
    let (mut swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
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
    // Phase 3: Randomness and encrypted-mempool feature verification.
    //
    // Each task independently waits for its DKG, then exercises the feature.
    // Both tasks run concurrently — they don't gate each other.
    // -----------------------------------------------------------------------
    info!("Phase 3: verifying randomness and encrypted mempool concurrently.");
    tokio::join!(
        // --- Task A: randomness_should_work_after_dkg ---
        async {
            let dkg_session = wait_for_dkg_finish_at_epoch(&client, 4, 120).await;
            assert_eq!(
                dkg_session.metadata.dealer_epoch, 4,
                "Phase 3 (randomness): DKG dealer_epoch must be 4"
            );
            assert_eq!(
                dkg_session.metadata.dealer_validator_set,
                dkg_session.metadata.target_validator_set,
                "Phase 3 (randomness): dealer_validator_set must equal target_validator_set"
            );
            info!(
                "Phase 3 (randomness): DKG completed — dealer_epoch={}, validators={}",
                dkg_session.metadata.dealer_epoch,
                dkg_session.metadata.target_validator_set.len()
            );
            let roll_ok = try_randomness_txn(&mut cli, root_idx).await;
            assert!(roll_ok, "Phase 3 (randomness): dice::roll should succeed after DKG");
            info!("Phase 3 (randomness): randomness txn succeeded.");
        },
        // --- Task B: encrypted_mempool_should_work_after_dkg ---
        async {
            let chunky_session = wait_for_chunky_dkg_finish_at_epoch(&client, 4, 120).await;
            assert_eq!(
                chunky_session.metadata.dealer_epoch, 4,
                "Phase 3 (encrypted mempool): chunky DKG dealer_epoch must be 4"
            );
            info!(
                "Phase 3 (encrypted mempool): chunky DKG completed — dealer_epoch={}",
                chunky_session.metadata.dealer_epoch
            );

            let ledger_state = client
                .get_ledger_information()
                .await
                .expect("failed to get ledger info")
                .into_inner();
            assert!(
                ledger_state.encryption_key.is_some(),
                "Phase 3 (encrypted mempool): encryption key must be present after chunky DKG"
            );
            let version_before = ledger_state.version;

            let all_validators: Vec<_> = swarm.validators().map(|v| v.peer_id()).collect();
            let enc_stats = generate_traffic(
                &mut swarm,
                &all_validators,
                Duration::from_secs(20),
                100,
                vec![vec![(TransactionType::default(), 1)]],
                true,
                Some(EmitJobMode::MaxLoad { mempool_backlog: 20 }),
            )
            .await
            .expect("Phase 3 (encrypted mempool): traffic generation failed");
            assert!(enc_stats.committed > 0, "Phase 3 (encrypted mempool): expected committed txns");

            let version_after = client
                .get_ledger_information()
                .await
                .unwrap()
                .into_inner()
                .version;
            let (enc_count, dec_count) = count_encrypted_txns(&client, version_before, version_after).await;
            info!("Phase 3 (encrypted mempool): {enc_count} encrypted, {dec_count} decrypted.");
            assert!(enc_count > 0, "Phase 3 (encrypted mempool): expected encrypted txns in ledger");
            assert!(dec_count > 0, "Phase 3 (encrypted mempool): expected decrypted txns in ledger");
            info!("Phase 3 (encrypted mempool): e2e verified.");
        },
    );
    info!("Phase 3: both randomness and encrypted mempool verified.");
}
