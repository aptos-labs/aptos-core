// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{smoke_test_environment::SwarmBuilder, utils::get_on_chain_resource};
use aptos_forge::{LocalNode, LocalSwarm, NodeExt, SwarmExt};
use aptos_logger::info;
use aptos_rest_client::Client;
use aptos_types::{
    dkg::{chunky_dkg::ChunkyDKGState, DKGState},
    on_chain_config::{
        FeatureFlag, Features, OnChainChunkyDKGConfig, OnChainRandomnessConfig, ValidatorSet,
    },
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

const HOLD_CHUNKY_DKG: &str = "chunky_dkg::process_dkg_start_event";
const DEAL_FINISHED: &str = "[ChunkyDKG] Deal transcript finished.";
/// If set, validator logs are copied into this directory before the test ends.
const LOG_DIR_ENV: &str = "CHUNKY_SMOKE_LOG_DIR";

fn save_logs(swarm: &LocalSwarm) {
    let Ok(dir) = std::env::var(LOG_DIR_ENV) else {
        return;
    };
    let _ = std::fs::create_dir_all(&dir);
    for node in swarm.validators() {
        let _ = std::fs::copy(node.log_path(), format!("{}/{}.log", dir, node.name()));
    }
}

fn fail(swarm: &LocalSwarm, message: String) -> ! {
    save_logs(swarm);
    panic!("{}", message);
}

/// Returns the length of the node's log, so later checks only scan new lines.
fn log_len(node: &LocalNode) -> usize {
    node.get_log_contents().map(|s| s.len()).unwrap_or(0)
}

/// Returns true if a line after byte `from` contains `needle` and the epoch field.
fn has_log_line(node: &LocalNode, from: usize, needle: &str, epoch: u64) -> bool {
    let epoch_field = format!("\"epoch\":{}", epoch);
    let contents = node.get_log_contents().unwrap_or_default();
    contents.get(from..).is_some_and(|new_lines| {
        new_lines
            .lines()
            .any(|line| line.contains(needle) && line.contains(&epoch_field))
    })
}

async fn wait_for_log_line(
    swarm: &LocalSwarm,
    index: usize,
    from: usize,
    needle: &str,
    epoch: u64,
) {
    let deadline = Instant::now() + Duration::from_secs(300);
    let node = swarm.validators().nth(index).unwrap();
    while !has_log_line(node, from, needle, epoch) {
        if Instant::now() >= deadline {
            fail(
                swarm,
                format!(
                    "Timed out waiting for {:?} in epoch {} on {}",
                    needle,
                    epoch,
                    node.name()
                ),
            );
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

/// Fails unless the epoch is still `epoch` and its Chunky DKG session is still pending.
async fn assert_still_pending(swarm: &LocalSwarm, observer: &Client, epoch: u64, step: &str) {
    let ledger = observer
        .get_ledger_information()
        .await
        .unwrap()
        .into_inner();
    let chunky = get_on_chain_resource::<ChunkyDKGState>(observer).await;
    let pending = chunky
        .in_progress
        .as_ref()
        .is_some_and(|s| s.metadata.dealer_epoch == epoch);
    if ledger.epoch != epoch || !pending {
        fail(
            swarm,
            format!(
                "Chunky DKG for epoch {} stopped being pending {} (now epoch {}). The hold or the test setup is wrong.",
                epoch, step, ledger.epoch
            ),
        );
    }
}

/// Restarts a validator without the hold failpoint, so it resumes the pending
/// Chunky DKG session and deals a new transcript.
async fn restart(swarm: &mut LocalSwarm, index: usize) {
    let node = swarm.validators_mut().nth(index).unwrap();
    node.stop();
    node.start().unwrap();
    node.wait_until_healthy(Instant::now() + Duration::from_secs(120))
        .await
        .unwrap();
}

/// A dealer that restarts during Chunky DKG deals a second valid transcript.
/// Validators that already accepted its first transcript must still sign an
/// aggregate that uses the second one.
///
/// Four validators with equal stake, so aggregation and certification each
/// need 3 validators. V3 never deals or signs, so they need V0, V1, and V2:
/// 1. V0 deals transcript A. V1 deals and accepts V0:A.
/// 2. V0 restarts and deals transcript B.
/// 3. V2 deals and accepts V0:B. V0 and V2 aggregate with V0:B, and V1
///    aggregates with V0:A.
///
/// Certification completes only if V1 signs the V0:B aggregate, or V0 and V2
/// sign the V0:A aggregate.
#[tokio::test]
async fn chunky_dkg_certifies_after_dealer_redeals() {
    let mut swarm = SwarmBuilder::new_local(4)
        .with_aptos()
        .with_init_config(Arc::new(|_, config, _| {
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
        .with_init_genesis_config(Arc::new(|config| {
            config.epoch_duration_secs = 30;
            config.consensus_config.enable_validator_txns();
            config.randomness_config_override = Some(OnChainRandomnessConfig::default_enabled());
            // V1 blocks the epoch change until Chunky DKG completes. There is
            // no grace period and no epoch watchdog.
            config.chunky_dkg_config_override = Some(OnChainChunkyDKGConfig::default_enabled());
            let mut features = Features::default();
            features.enable(FeatureFlag::ENCRYPTED_TRANSACTIONS);
            config.initial_features_override = Some(features);
        }))
        .build()
        .await;

    swarm
        // The first Chunky DKG includes expensive key generation on debug builds.
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(600))
        .await
        .unwrap();

    // V3 never restarts, so use it for on-chain reads.
    let observer = swarm.validators().nth(3).unwrap().rest_client();
    for node in swarm.validators() {
        node.rest_client()
            .set_failpoint(HOLD_CHUNKY_DKG.into(), "return".into())
            .await
            .unwrap();
    }

    // Wait for the next session: ordinary DKG done, Chunky DKG pending, nobody dealt.
    let deadline = Instant::now() + Duration::from_secs(180);
    let stalled_epoch = loop {
        if Instant::now() >= deadline {
            fail(
                &swarm,
                "Timed out waiting for a pending Chunky DKG session".into(),
            );
        }
        let ledger = observer
            .get_ledger_information()
            .await
            .unwrap()
            .into_inner();
        let ordinary = get_on_chain_resource::<DKGState>(&observer).await;
        let chunky = get_on_chain_resource::<ChunkyDKGState>(&observer).await;
        if ordinary.in_progress.is_none()
            && ordinary
                .last_completed
                .as_ref()
                .is_some_and(|s| s.metadata.dealer_epoch == ledger.epoch)
            && chunky
                .in_progress
                .as_ref()
                .is_some_and(|s| s.metadata.dealer_epoch == ledger.epoch)
        {
            break ledger.epoch;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    };
    info!("Chunky DKG for epoch {} is pending.", stalled_epoch);
    // Nobody may deal while the hold is active.
    tokio::time::sleep(Duration::from_secs(20)).await;
    assert_still_pending(
        &swarm,
        &observer,
        stalled_epoch,
        "while all validators hold",
    )
    .await;

    // The aggregator log identifies a dealer by its index in the validator set.
    let v0_address = swarm.validators().next().unwrap().peer_id();
    let v0_index = get_on_chain_resource::<ValidatorSet>(&observer)
        .await
        .active_validators
        .iter()
        .position(|v| v.account_address == v0_address)
        .expect("V0 is in the validator set");
    let v1_accepted_v0 = format!(
        "[ChunkyDKG] added chunky transcript from validator {}, ",
        v0_index
    );

    // 1. V0 deals transcript A. V1 deals and accepts V0:A.
    let from = log_len(swarm.validators().next().unwrap());
    restart(&mut swarm, 0).await;
    wait_for_log_line(&swarm, 0, from, DEAL_FINISHED, stalled_epoch).await;
    let from = log_len(swarm.validators().nth(1).unwrap());
    restart(&mut swarm, 1).await;
    wait_for_log_line(&swarm, 1, from, &v1_accepted_v0, stalled_epoch).await;
    info!("V1 accepted V0's first transcript.");
    assert_still_pending(&swarm, &observer, stalled_epoch, "after V0 and V1 dealt").await;

    // 2. V0 restarts and deals transcript B.
    let from = log_len(swarm.validators().next().unwrap());
    restart(&mut swarm, 0).await;
    wait_for_log_line(&swarm, 0, from, DEAL_FINISHED, stalled_epoch).await;
    info!("V0 dealt its second transcript.");
    assert_still_pending(&swarm, &observer, stalled_epoch, "after V0 re-dealt").await;

    // 3. V2 deals and accepts V0:B. Aggregation now reaches quorum on V0, V1, and V2.
    restart(&mut swarm, 2).await;

    let deadline = Instant::now() + Duration::from_secs(300);
    loop {
        let ledger = observer
            .get_ledger_information()
            .await
            .unwrap()
            .into_inner();
        if ledger.epoch > stalled_epoch {
            break;
        }
        if Instant::now() >= deadline {
            fail(
                &swarm,
                format!(
                    "Chunky DKG certification did not complete after a dealer re-dealt. Epoch {} is still stalled.",
                    stalled_epoch
                ),
            );
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    let chunky = get_on_chain_resource::<ChunkyDKGState>(&observer).await;
    assert_eq!(
        chunky.last_completed.unwrap().metadata.dealer_epoch,
        stalled_epoch
    );
    save_logs(&swarm);
}
