// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Regression test: a validator must resume committing after a restart with a commit backlog.
//!
//! A validator restarts with more rounds certified in its ConsensusDB than committed in its
//! ledger. On startup, `BlockStore::try_send_for_execution` replays one ordered batch per
//! recovered QC. The buffer manager only ingests ordered batches while
//! `latest_round <= highest_committed_round + MAX_BACKLOG (20)`. The rest of the network has
//! already committed these rounds, so it never sends commit votes for them again. The only
//! commit proof that arrives is for a round beyond the backlog window, so the buffer manager
//! parks it and never applies it, and the validator does not commit again until an epoch change.

use crate::smoke_test_environment::SwarmBuilder;
use aptos_forge::{NodeExt, SwarmExt};
use aptos_rest_client::Client;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

const COMMIT_STALL_FAILPOINT: &str = "consensus::commit_ledger::stall";
/// How far (in blocks) the stalled validator's ledger must fall behind before the restart.
/// It must be well above the buffer manager's MAX_BACKLOG (20) and well below the gap at which
/// pre-commit pauses (200), which would otherwise trigger state sync while the node runs.
const MIN_COMMIT_GAP_BLOCKS: u64 = 50;
const MAX_WAIT_SECS: u64 = 90;
const RECOVERY_WAIT_SECS: u64 = 120;

/// Returns (block_height, version) of the node's committed ledger, if its API responds.
async fn ledger(client: &Client) -> Option<(u64, u64)> {
    client.get_ledger_information().await.ok().map(|response| {
        let state = response.into_inner();
        (state.block_height, state.version)
    })
}

#[tokio::test]
async fn test_restart_with_commit_backlog_over_max_backlog() {
    let swarm = SwarmBuilder::new_local(4)
        .with_aptos()
        .with_init_config(Arc::new(|_, conf, _| {
            conf.api.failpoints_enabled = true;
        }))
        .with_init_genesis_config(Arc::new(|genesis_config| {
            // An epoch change resets the pipeline and would hide the bug.
            genesis_config.epoch_duration_secs = 10_000;
        }))
        .build()
        .await;
    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_WAIT_SECS))
        .await
        .unwrap();

    // A and B stay healthy, C is taken down to remove fault tolerance, V is the node under test.
    let peers: Vec<_> = swarm.validators().map(|v| v.peer_id()).collect();
    let (peer_a, peer_b, peer_c, peer_v) = (peers[0], peers[1], peers[2], peers[3]);
    let client_a = swarm.validator(peer_a).unwrap().rest_client();
    let client_v = swarm.validator(peer_v).unwrap().rest_client();

    // Step 1: stall V's ledger commit. V keeps ordering and pre-committing blocks (and storing
    // their QCs in ConsensusDB), but its committed ledger stops. A, B and C keep committing.
    client_v
        .set_failpoint(COMMIT_STALL_FAILPOINT.to_string(), "return".to_string())
        .await
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(MAX_WAIT_SECS);
    let (stalled_height, stalled_version) = loop {
        let (height_a, _) = ledger(&client_a).await.expect("A's API must respond");
        let (height_v, version_v) = ledger(&client_v).await.expect("V's API must respond");
        if height_a >= height_v + MIN_COMMIT_GAP_BLOCKS {
            break (height_v, version_v);
        }
        assert!(
            Instant::now() < deadline,
            "V's ledger did not fall {} blocks behind (A: {}, V: {})",
            MIN_COMMIT_GAP_BLOCKS,
            height_a,
            height_v
        );
        tokio::time::sleep(Duration::from_millis(500)).await;
    };
    println!("V's ledger is stalled at height {stalled_height} (version {stalled_version})");

    // Step 2: stop C. V withholds votes (its commit root is far behind), so A and B alone
    // cannot form a quorum and the chain halts at a tip that V has already seen.
    swarm.validator(peer_c).unwrap().stop();
    // Give A and B time to finish persisting everything they committed.
    tokio::time::sleep(Duration::from_secs(5)).await;
    let (tip_height, tip_version) = ledger(&client_a).await.expect("A's API must respond");
    println!("Chain halted at tip height {tip_height} (version {tip_version})");

    // Step 3: stop A, B and V. Restarting V while it has no peers means its state sync
    // bootstrapper has nothing to sync to, so consensus recovery starts from V's stale ledger.
    for peer in [peer_a, peer_b, peer_v] {
        swarm.validator(peer).unwrap().stop();
    }

    // Step 4: restart V alone. The failpoint is in-process only, so it is now off.
    let node_v = swarm.validator(peer_v).unwrap();
    node_v.start().unwrap();
    node_v
        .wait_until_healthy(Instant::now() + Duration::from_secs(MAX_WAIT_SECS))
        .await
        .unwrap();
    // Let state sync bootstrap and consensus recovery replay the recovered QCs.
    tokio::time::sleep(Duration::from_secs(10)).await;

    // Precondition: V restarted far enough behind to exceed the buffer manager backlog.
    let (restart_height, _) = ledger(&client_v).await.expect("V's API must respond");
    println!("V restarted at height {restart_height}; tip is {tip_height}");
    assert!(
        tip_height >= restart_height + MIN_COMMIT_GAP_BLOCKS / 2,
        "Precondition not met: V restarted at height {restart_height}, tip is {tip_height}"
    );

    // Step 5: bring A and B back. C stays down, so the chain needs V to make progress.
    for peer in [peer_a, peer_b] {
        let node = swarm.validator(peer).unwrap();
        node.start().unwrap();
        node.wait_until_healthy(Instant::now() + Duration::from_secs(MAX_WAIT_SECS))
            .await
            .unwrap();
    }

    // Step 6: the chain must recover. V must commit up to the old tip, and the chain must
    // grow past it. With the bug, V never commits and the chain stays halted.
    let deadline = Instant::now() + Duration::from_secs(RECOVERY_WAIT_SECS);
    let mut last_report = Instant::now();
    loop {
        let height_a = ledger(&client_a).await.map_or(0, |(h, _)| h);
        let (height_v, version_v) = ledger(&client_v).await.unwrap_or((0, 0));
        if version_v >= tip_version && height_a > tip_height + 10 {
            println!("Recovered: V at height {height_v}, A at height {height_a}");
            break;
        }
        if last_report.elapsed() >= Duration::from_secs(10) {
            println!(
                "Waiting: V at height {height_v}, A at height {height_a}, old tip {tip_height}"
            );
            last_report = Instant::now();
        }
        assert!(
            Instant::now() < deadline,
            "Chain did not recover within {RECOVERY_WAIT_SECS}s: V stuck at height {height_v} \
             (restarted at {restart_height}), A at height {height_a}, old tip {tip_height}"
        );
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
