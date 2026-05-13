// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{smoke_test_environment::SwarmBuilder, utils::get_on_chain_resource};
use aptos::common::types::GasOptions;
use aptos_forge::{NodeExt, Swarm, SwarmExt};
use aptos_logger::{debug, info};
use aptos_rest_client::Client;
use aptos_types::{
    dkg::{chunky_dkg::ChunkyDKGState, DKGState},
    on_chain_config::{FeatureFlag, Features, OnChainChunkyDKGConfig, OnChainRandomnessConfig},
};
use futures::future::join_all;
use std::{
    ops::Add,
    sync::Arc,
    time::{Duration, Instant},
};

/// Recovery from a ChunkyDKG-output stall using only a governance txn.
///
/// In the common chunky-stuck failure mode, consensus is still alive (blocks
/// keep being produced; only the epoch transition is wedged). A governance
/// proposal calling `aptos_governance::force_end_epoch` clears the lingering
/// chunky session and advances the epoch atomically, with no validator
/// restart, no local seqnum override, and no execution divergence. This test
/// exercises that recovery path.
///
///   1. Wait for epoch 2 so a normal DKG/chunky cycle has completed.
///   2. Arm `chunky_dkg::process_dkg_start_event` failpoint on every
///      validator so the chunky manager returns early and never produces a
///      transcript.
///   3. Wait until the next V2 prologue fires and the on-chain state shows
///      DKG cleared but chunky still in_progress (the wedge state).
///   4. Verify consensus is still alive by running a liveness check.
///   5. Clear the failpoint so chunky can run normally after recovery.
///   6. Submit a governance script calling `aptos_governance::force_end_epoch`.
///   7. Verify the epoch advances past the wedged epoch.
///   8. Verify ChunkyDKG completes a fresh session in the next epoch
///      transition.
#[tokio::test]
async fn chunky_dkg_stall_governance_recovery() {
    let epoch_duration_secs = 20;
    let estimated_dkg_latency_secs = 60;

    let (swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
        .with_num_fullnodes(0)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| {
            config.api.allow_encrypted_txns_submission = true;
            config.api.failpoints_enabled = true;
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
            conf.consensus_config.enable_validator_txns();
            conf.randomness_config_override = Some(OnChainRandomnessConfig::default_enabled());
            conf.chunky_dkg_config_override = Some(OnChainChunkyDKGConfig::default_enabled());
            let mut features = Features::default();
            features.enable(FeatureFlag::ENCRYPTED_TRANSACTIONS);
            conf.initial_features_override = Some(features);
        }))
        .build_with_cli(0)
        .await;

    let root_addr = swarm.chain_info().root_account().address();
    let root_idx = cli.add_account_with_address_to_cli(swarm.root_key(), root_addr);

    let validator_clients: Vec<Client> =
        swarm.validators().map(|node| node.rest_client()).collect();
    let rest_client = validator_clients[0].clone();

    info!("Wait for epoch 2.");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(
            2,
            Duration::from_secs(epoch_duration_secs + estimated_dkg_latency_secs),
        )
        .await
        .expect("Epoch 2 taking too long to arrive!");

    info!("Arm chunky DKG failpoint on every validator so chunky manager returns early.");
    let tasks = validator_clients.iter().map(|client| {
        client.set_failpoint(
            "chunky_dkg::process_dkg_start_event".to_string(),
            "return".to_string(),
        )
    });
    let results = join_all(tasks).await;
    debug!("set_failpoint results={:?}", results);
    for r in results {
        r.expect("set_failpoint failed");
    }

    info!("Wait for the wedge: DKG cleared, chunky session lingering on-chain.");
    let stall_deadline = Instant::now().add(Duration::from_secs(
        epoch_duration_secs + estimated_dkg_latency_secs,
    ));
    loop {
        if Instant::now() >= stall_deadline {
            panic!("Timed out waiting for the chunky stall to develop.");
        }
        let dkg_state = get_on_chain_resource::<DKGState>(&rest_client).await;
        let chunky_state = get_on_chain_resource::<ChunkyDKGState>(&rest_client).await;
        let dkg_cleared = dkg_state.in_progress.is_none();
        let chunky_pending = chunky_state.in_progress.is_some();
        debug!(
            "stall poll: dkg_in_progress={} chunky_in_progress={}",
            dkg_state.in_progress.is_some(),
            chunky_pending
        );
        if dkg_cleared && chunky_pending {
            info!("Wedge reached: dkg cleared, chunky session lingering.");
            break;
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    let stalled_epoch = rest_client
        .get_ledger_information()
        .await
        .unwrap()
        .into_inner()
        .epoch;
    info!("Reconfig wedged in epoch {}.", stalled_epoch);

    info!("Verify consensus is still alive while reconfig is wedged.");
    let liveness_check_result = swarm
        .liveness_check(Instant::now().add(Duration::from_secs(20)))
        .await;
    assert!(
        liveness_check_result.is_ok(),
        "Chain should still be producing blocks during a chunky-only stall \
         (only the epoch transition is wedged); liveness check failed: {:?}",
        liveness_check_result
    );

    info!("Clear the failpoint so chunky DKG can run normally after recovery.");
    let tasks = validator_clients.iter().map(|client| {
        client.set_failpoint(
            "chunky_dkg::process_dkg_start_event".to_string(),
            "off".to_string(),
        )
    });
    let results = join_all(tasks).await;
    debug!("clear failpoint results={:?}", results);
    for r in results {
        r.expect("clear failpoint failed");
    }

    info!("Submit governance force_end_epoch to clear the wedge.");
    let script = r#"
script {
    use aptos_framework::aptos_governance;

    fun main(core_resources: &signer) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0x1);
        aptos_governance::force_end_epoch(&framework_signer);
    }
}
    "#;
    let gas_options = GasOptions {
        gas_unit_price: Some(1),
        max_gas: Some(2000000),
        expiration_secs: 60,
    };
    let txn_summary = cli
        .run_script_with_gas_options(root_idx, script, Some(gas_options))
        .await
        .expect("Governance txn execution failed.");
    debug!("force_end_epoch txn_summary={:?}", txn_summary);

    info!(
        "Verify the chain advances past the wedged epoch {}.",
        stalled_epoch
    );
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(
            stalled_epoch + 1,
            Duration::from_secs(epoch_duration_secs + estimated_dkg_latency_secs),
        )
        .await
        .unwrap_or_else(|_| {
            panic!(
                "Epoch {} not reached after governance force_end_epoch.",
                stalled_epoch + 1
            )
        });

    let recovered_epoch = rest_client
        .get_ledger_information()
        .await
        .unwrap()
        .into_inner()
        .epoch;
    info!(
        "Recovered to epoch {}. Wait for the next normal reconfig with chunky.",
        recovered_epoch
    );

    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(
            recovered_epoch + 1,
            Duration::from_secs(epoch_duration_secs + estimated_dkg_latency_secs),
        )
        .await
        .unwrap_or_else(|_| {
            panic!(
                "Epoch {} not reached after recovery (next normal reconfig).",
                recovered_epoch + 1
            )
        });

    info!("Verify ChunkyDKG completed a session in the post-recovery epoch transition.");
    let chunky_state = get_on_chain_resource::<ChunkyDKGState>(&rest_client).await;
    let last_completed = chunky_state
        .last_completed
        .as_ref()
        .expect("ChunkyDKG should have a completed session after recovery");
    let target_epoch = last_completed.target_epoch();
    assert!(
        target_epoch > stalled_epoch,
        "Last completed ChunkyDKG should target a post-recovery epoch (>{}); got {}",
        stalled_epoch,
        target_epoch
    );
}
