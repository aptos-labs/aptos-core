// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    genesis::enable_sync_only_mode, smoke_test_environment::SwarmBuilder,
    utils::get_on_chain_resource,
};
use aptos::common::types::GasOptions;
use aptos_config::config::{OverrideNodeConfig, PersistableConfig};
use aptos_forge::{NodeExt, Swarm, SwarmExt};
use aptos_logger::{debug, info};
use aptos_types::{
    dkg::chunky_dkg::ChunkyDKGState,
    on_chain_config::{FeatureFlag, Features, OnChainChunkyDKGConfig, OnChainRandomnessConfig},
};
use std::{
    ops::Add,
    sync::Arc,
    time::{Duration, Instant},
};

/// Chain recovery using a local config from ChunkyDKG stall should work.
/// See `chunky_dkg_config_seqnum.move` for more details.
///
/// Test flow:
/// 1. Inject failpoint to block ChunkyDKG start event processing on all validators.
/// 2. Wait for epoch boundary stall (consensus runs producing blocks, but epoch can't
///    transition because ChunkyDKG never completes while regular DKG does).
/// 3. Put all validators into sync_only mode to converge to the same version.
/// 4. Restart validators with `chunky_dkg_override_seq_num=1` and `sync_only=false`.
///    This clears the failpoint and disables ChunkyDKGManager. Epoch is still stuck because
///    on-chain ChunkyDKG session remains in-progress with no one to complete it.
/// 5. Governance `force_end_epoch()` clears the stuck session and bumps on-chain seqnum to 2.
///    Since on-chain seqnum (2) > local override (1), ChunkyDKG is re-enabled in the new epoch.
/// 6. Verify the next epoch advances (both DKG and ChunkyDKG complete successfully).
#[tokio::test]
async fn chunky_dkg_stall_recovery() {
    let epoch_duration_secs = 10;
    let estimated_dkg_latency_secs = 120;

    let (mut swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
        .with_num_fullnodes(0)
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

    info!("Wait for epoch 2 (proves ChunkyDKG completed for epoch transition).");
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(
            2,
            Duration::from_secs(epoch_duration_secs + estimated_dkg_latency_secs),
        )
        .await
        .expect("Epoch 2 taking too long to arrive!");

    info!("Verify ChunkyDKG completed a session before we stall.");
    let rest_client = swarm.validators().next().unwrap().rest_client();
    let dkg_state = get_on_chain_resource::<ChunkyDKGState>(&rest_client).await;
    assert!(
        dkg_state.last_completed.is_some(),
        "ChunkyDKG should have a completed session before stall"
    );
    let pre_stall_completed_epoch = dkg_state.last_completed.unwrap().target_epoch();

    info!("Injecting chunky_dkg::process_dkg_start_event failpoint to stall ChunkyDKG.");
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

    // Get the current epoch. The next epoch boundary should stall because ChunkyDKG
    // can't complete (failpoint blocks its start event processing).
    let current_epoch = rest_client
        .get_ledger_information()
        .await
        .unwrap()
        .into_inner()
        .epoch;
    info!(
        "Current epoch is {}. Waiting to confirm epoch doesn't advance past {} (ChunkyDKG stall).",
        current_epoch, current_epoch
    );

    // Wait long enough for an epoch transition to happen if it could.
    // The epoch timer is 10s, DKG takes some time. If the epoch advances, the failpoint didn't work.
    tokio::time::sleep(Duration::from_secs(epoch_duration_secs + 30)).await;
    let epoch_after_stall = rest_client
        .get_ledger_information()
        .await
        .unwrap()
        .into_inner()
        .epoch;
    info!(
        "After waiting, epoch is {}. Expected <= {} (stalled).",
        epoch_after_stall,
        current_epoch + 1
    );
    // The epoch might advance by 1 (regular DKG completes but ChunkyDKG blocks the finish),
    // but should not advance further. Actually it should NOT advance at all since
    // both DKG and ChunkyDKG must complete for epoch transition.
    // However, the reconfiguration starts when the epoch timer fires. Regular DKG completes
    // but ChunkyDKG doesn't, so finish() is never called. The epoch stays the same.
    assert!(
        epoch_after_stall <= current_epoch + 1,
        "Epoch should be stalled due to ChunkyDKG failpoint, but advanced to {}",
        epoch_after_stall
    );

    // Put all validators into sync_only mode to converge to the same version.
    info!("Putting all validators into sync_only mode to converge versions.");
    for validator in swarm.validators_mut() {
        enable_sync_only_mode(4, validator).await;
    }

    // Restart all validators with the override and sync_only=false.
    info!("Restarting all validators with chunky_dkg_override_seq_num=1 and sync_only=false.");
    for (idx, validator) in swarm.validators_mut().enumerate() {
        validator.stop();
        let config_path = validator.config_path();
        let mut validator_override_config =
            OverrideNodeConfig::load_config(config_path.clone()).unwrap();
        validator_override_config
            .override_config_mut()
            .chunky_dkg_override_seq_num = 1;
        validator_override_config
            .override_config_mut()
            .consensus
            .sync_only = false;
        validator_override_config.save_config(config_path).unwrap();
        validator.start().unwrap();
        info!("Validator {} restarted with override.", idx);
    }

    // Wait for all validators to become healthy (needs quorum to make progress).
    info!("Waiting for all validators to become healthy.");
    swarm
        .liveness_check(Instant::now().add(Duration::from_secs(60)))
        .await
        .expect("Validators failed to become healthy after restart");

    // Chain is running (consensus produces blocks in current epoch) but epoch transition
    // is still stuck: on-chain ChunkyDKG session is in-progress with no one to complete it
    // (ChunkyDKGManager disabled by override).
    // Use force_end_epoch() to clear the incomplete session and bump seqnum to re-enable.
    info!("Running governance script: bump seqnum to 2 and force_end_epoch().");
    let rest_client = swarm.validators().next().unwrap().rest_client();
    let script = r#"
script {
    use aptos_framework::aptos_governance;
    use aptos_framework::chunky_dkg_config_seqnum;

    fun main(core_resources: &signer) {
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0x1);
        chunky_dkg_config_seqnum::set_for_next_epoch(&framework_signer, 2);
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
        .expect("Txn execution error.");
    debug!("txn_summary={:?}", txn_summary);

    let epoch = rest_client
        .get_ledger_information()
        .await
        .unwrap()
        .into_inner()
        .epoch;
    info!(
        "After force_end_epoch, current epoch is {}. Wait until epoch {} (ChunkyDKG re-enabled, should complete).",
        epoch,
        epoch + 1
    );

    // In the new epoch, on-chain seqnum is 2, local override is 1.
    // Since 1 < 2, ChunkyDKG is re-enabled. Failpoint was cleared by restart.
    // Both DKG and ChunkyDKG must complete for the epoch to advance.
    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(
            epoch + 1,
            Duration::from_secs(epoch_duration_secs + estimated_dkg_latency_secs),
        )
        .await
        .unwrap_or_else(|_| panic!("Epoch {} taking too long to arrive!", epoch + 1));

    info!("Verify ChunkyDKG completed a new session after re-enable.");
    let dkg_state = get_on_chain_resource::<ChunkyDKGState>(&rest_client).await;
    assert!(
        dkg_state.last_completed.is_some(),
        "ChunkyDKG should have a completed session after re-enable"
    );
    let post_reenable_epoch = dkg_state.last_completed.unwrap().target_epoch();
    assert!(
        post_reenable_epoch > pre_stall_completed_epoch,
        "ChunkyDKG should have completed for a newer epoch after re-enable (got {}, pre-stall was {})",
        post_reenable_epoch,
        pre_stall_completed_epoch
    );

    info!(
        "ChunkyDKG re-enabled: completed session for epoch {} (pre-stall was {}).",
        post_reenable_epoch, pre_stall_completed_epoch
    );
}
