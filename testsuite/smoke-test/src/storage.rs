// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    smoke_test_environment::SwarmBuilder,
    utils::{
        assert_balance, create_and_fund_account, swarm_utils::insert_waypoint,
        transfer_and_maybe_reconfig, transfer_coins, MAX_CATCH_UP_WAIT_SECS, MAX_HEALTHY_WAIT_SECS,
    },
    workspace_builder,
    workspace_builder::workspace_root,
};
use anyhow::{bail, Result};
use velor_backup_cli::metadata::view::BackupStorageState;
use velor_forge::{reconfig, VelorPublicInfo, Node, NodeExt, Swarm, SwarmExt};
use velor_logger::info;
use velor_temppath::TempPath;
use velor_types::{transaction::Version, waypoint::Waypoint};
use itertools::Itertools;
use std::{
    fs,
    path::Path,
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

const LINE: &str = "----------";

#[tokio::test]
async fn test_db_restore() {
    // pre-build tools
    ::velor_logger::Logger::new().init();
    info!("---------- 0. test_db_restore started.");
    workspace_builder::get_bin("velor-debugger");
    info!("---------- 1. pre-building finished.");

    let mut swarm = SwarmBuilder::new_local(4).with_velor().build().await;
    info!("---------- 1.1 swarm built, sending some transactions.");
    let validator_peer_ids = swarm.validators().map(|v| v.peer_id()).collect::<Vec<_>>();
    let client_1 = swarm
        .validator(validator_peer_ids[1])
        .unwrap()
        .rest_client();
    let transaction_factory = swarm.chain_info().transaction_factory();

    // set up: two accounts, a lot of money
    let mut account_0 = create_and_fund_account(&mut swarm, 1000000).await;
    let account_1 = create_and_fund_account(&mut swarm, 1000000).await;

    info!("---------- 1.2 wait for nodes to catch up.");
    // we need to wait for all nodes to see it, as client_1 is different node from the
    // one creating accounts above
    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_CATCH_UP_WAIT_SECS))
        .await
        .unwrap();
    info!("---------- 1.3 caught up.");

    assert_balance(&client_1, &account_0, 1000000).await;
    assert_balance(&client_1, &account_1, 1000000).await;

    let mut expected_balance_0 = 999999;
    let mut expected_balance_1 = 1000001;

    transfer_coins(
        &client_1,
        &transaction_factory,
        &mut account_0,
        &account_1,
        1,
    )
    .await;

    assert_balance(&client_1, &account_0, expected_balance_0).await;
    assert_balance(&client_1, &account_1, expected_balance_1).await;

    expected_balance_0 -= 10;
    expected_balance_1 += 10;

    transfer_and_maybe_reconfig(
        &client_1,
        &transaction_factory,
        swarm.chain_info().root_account,
        &mut account_0,
        &account_1,
        5,
    )
    .await;
    // explicit reconfigs: we are at least at epoch 5
    for _ in 0..4 {
        reconfig(
            &client_1,
            &transaction_factory,
            swarm.chain_info().root_account,
        )
        .await;
    }
    // some more reconfigs to complicate things by putting in multiple epoch boundaries
    // in a transaction backup
    transfer_and_maybe_reconfig(
        &client_1,
        &transaction_factory,
        swarm.chain_info().root_account,
        &mut account_0,
        &account_1,
        5,
    )
    .await;
    assert_balance(&client_1, &account_0, expected_balance_0).await;
    assert_balance(&client_1, &account_1, expected_balance_1).await;

    info!("---------- 2. reached at least epoch 5, starting backup coordinator.");
    // make a backup from node 1
    let node1_config = swarm.validator(validator_peer_ids[1]).unwrap().config();
    let port = node1_config.storage.backup_service_address.port();
    let (backup_path, _) = db_backup(port, 5, 400, 200, 5, &[]);
    // take down node 0
    let node_to_restart = validator_peer_ids[0];
    swarm.validator_mut(node_to_restart).unwrap().stop();

    // nuke db
    let node0_config_path = swarm.validator(node_to_restart).unwrap().config_path();
    let mut node0_config = swarm.validator(node_to_restart).unwrap().config().clone();
    let genesis_waypoint = node0_config.base.waypoint.genesis_waypoint();
    insert_waypoint(&mut node0_config, genesis_waypoint);
    node0_config.save_to_path(node0_config_path).unwrap();
    let db_dir = node0_config.storage.dir();
    fs::remove_dir_all(db_dir.clone()).unwrap();

    info!("---------- 3. stopped node 0, gonna restore DB.");
    // restore db from backup
    db_restore(backup_path.path(), db_dir.as_path(), &[], None);

    expected_balance_0 -= 3;
    expected_balance_1 += 3;

    transfer_and_maybe_reconfig(
        &client_1,
        &transaction_factory,
        swarm.chain_info().root_account,
        &mut account_0,
        &account_1,
        3,
    )
    .await;

    assert_balance(&client_1, &account_0, expected_balance_0).await;
    assert_balance(&client_1, &account_1, expected_balance_1).await;

    info!("---------- 4. Gonna restart node 0.");
    // start node 0 on top of restored db
    swarm
        .validator_mut(node_to_restart)
        .unwrap()
        .start()
        .unwrap();
    swarm
        .validator_mut(node_to_restart)
        .unwrap()
        .wait_until_healthy(Instant::now() + Duration::from_secs(MAX_HEALTHY_WAIT_SECS))
        .await
        .unwrap();
    info!("---------- 5. Node 0 is healthy, verify it's caught up.");
    // verify it's caught up
    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_CATCH_UP_WAIT_SECS))
        .await
        .unwrap();

    let client_0 = swarm.validator(node_to_restart).unwrap().rest_client();

    assert_balance(&client_0, &account_0, expected_balance_0).await;
    assert_balance(&client_0, &account_1, expected_balance_1).await;
    info!("6. Done");
}

fn db_backup_verify(backup_path: &Path, trusted_waypoints: &[Waypoint]) {
    info!("---------- running velor-debugger velor-db backup-verify");
    let now = Instant::now();
    let bin_path = workspace_builder::get_bin("velor-debugger");
    let metadata_cache_path = TempPath::new();

    metadata_cache_path.create_as_dir().unwrap();

    let mut cmd = Command::new(bin_path.as_path());
    cmd.args(["velor-db", "backup", "verify"]);
    trusted_waypoints.iter().for_each(|w| {
        cmd.arg("--trust-waypoint");
        cmd.arg(&w.to_string());
    });

    let status = cmd
        .args([
            "--metadata-cache-dir",
            metadata_cache_path.path().to_str().unwrap(),
            "--concurrent-downloads",
            "4",
            "--local-fs-dir",
            backup_path.to_str().unwrap(),
        ])
        .current_dir(workspace_root())
        .status()
        .unwrap();
    assert!(status.success(), "{}", status);
    info!("Backup verified in {} seconds.", now.elapsed().as_secs());
}

fn replay_verify(backup_path: &Path, trusted_waypoints: &[Waypoint]) {
    info!("---------- running replay-verify");
    let now = Instant::now();
    let bin_path = workspace_builder::get_bin("velor-debugger");
    let metadata_cache_path = TempPath::new();
    let target_db_dir = TempPath::new();

    metadata_cache_path.create_as_dir().unwrap();

    let mut cmd = Command::new(bin_path.as_path());
    cmd.args(["velor-db", "replay-verify"]);
    trusted_waypoints.iter().for_each(|w| {
        cmd.arg("--trust-waypoint");
        cmd.arg(&w.to_string());
    });

    let replay = cmd
        .args([
            "--metadata-cache-dir",
            metadata_cache_path.path().to_str().unwrap(),
            "--concurrent-downloads",
            "4",
            "--target-db-dir",
            target_db_dir.path().to_str().unwrap(),
            "--local-fs-dir",
            backup_path.to_str().unwrap(),
        ])
        .current_dir(workspace_root())
        .output()
        .unwrap();
    assert!(
        replay.status.success(),
        "{}, {}",
        std::str::from_utf8(&replay.stderr).unwrap(),
        std::str::from_utf8(&replay.stdout).unwrap(),
    );

    info!(
        "Backup replay-verified in {} seconds.",
        now.elapsed().as_secs()
    );
}

fn wait_for_backups(
    target_epoch: u64,
    target_version: u64,
    now: Instant,
    bin_path: &Path,
    metadata_cache_path: &Path,
    backup_path: &Path,
    trusted_waypoints: &[Waypoint],
) -> Result<Version> {
    for i in 0..120 {
        info!(
            "{}th wait for the backup to reach epoch {}, version {}.",
            i, target_epoch, target_version,
        );
        let state = get_backup_storage_state(bin_path, metadata_cache_path, backup_path)?;
        if state.latest_epoch_ending_epoch.is_some()
            && state.latest_transaction_version.is_some()
            && state.latest_state_snapshot_epoch.is_some()
            && state.latest_state_snapshot_epoch.is_some()
            && state.latest_epoch_ending_epoch.unwrap() >= target_epoch
            && state.latest_transaction_version.unwrap() >= target_version
            && state.latest_transaction_version.unwrap()
                >= state.latest_state_snapshot_version.unwrap()
        {
            info!(
                "Backup created in {} seconds. backup storage state: {}",
                now.elapsed().as_secs(),
                state
            );
            return Ok(state.latest_state_snapshot_version.unwrap());
        }
        info!("Backup storage state: {}", state);
        if state.latest_transaction_version.is_some() {
            // the verify should always succeed unless backup storage is completely empty.
            db_backup_verify(backup_path, trusted_waypoints);
        }
        std::thread::sleep(Duration::from_secs(1));
    }

    bail!("Failed to create backup.");
}

fn get_backup_storage_state(
    bin_path: &Path,
    metadata_cache_path: &Path,
    backup_path: &Path,
) -> Result<BackupStorageState> {
    let output = Command::new(bin_path)
        .current_dir(workspace_root())
        .args([
            "velor-db",
            "backup",
            "query",
            "backup-storage-state",
            "--metadata-cache-dir",
            metadata_cache_path.to_str().unwrap(),
            "--concurrent-downloads",
            "4",
            "--local-fs-dir",
            backup_path.to_str().unwrap(),
        ])
        .output()?
        .stdout;
    std::str::from_utf8(&output)?.parse()
}

#[allow(clippy::zombie_processes)]
pub(crate) fn db_backup(
    backup_service_port: u16,
    target_epoch: u64,
    target_version: Version,
    transaction_batch_size: usize,
    state_snapshot_interval_epochs: usize,
    trusted_waypoints: &[Waypoint],
) -> (TempPath, Version) {
    info!("---------- running velor db tool backup");
    let now = Instant::now();
    let bin_path = workspace_builder::get_bin("velor-debugger");
    let metadata_cache_path1 = TempPath::new();
    let metadata_cache_path2 = TempPath::new();
    let backup_path = TempPath::new();

    metadata_cache_path1.create_as_dir().unwrap();
    metadata_cache_path2.create_as_dir().unwrap();
    backup_path.create_as_dir().unwrap();

    // Initialize backup storage, avoid race between the coordinator and wait_for_backups to create
    // the identity file.
    get_backup_storage_state(&bin_path, metadata_cache_path2.path(), backup_path.path()).unwrap();

    // spawn the backup coordinator
    let mut backup_coordinator = Command::new(bin_path.as_path())
        .current_dir(workspace_root())
        .args([
            "velor-db",
            "backup",
            "continuously",
            "--backup-service-address",
            &format!("http://localhost:{}", backup_service_port),
            "--transaction-batch-size",
            &transaction_batch_size.to_string(),
            "--state-snapshot-interval-epochs",
            &state_snapshot_interval_epochs.to_string(),
            "--metadata-cache-dir",
            metadata_cache_path1.path().to_str().unwrap(),
            "--concurrent-downloads",
            "4",
            "--local-fs-dir",
            backup_path.path().to_str().unwrap(),
        ])
        .spawn()
        .unwrap();

    // watch the backup storage, wait for it to reach target epoch and version
    let wait_res = wait_for_backups(
        target_epoch,
        target_version,
        now,
        bin_path.as_path(),
        metadata_cache_path2.path(),
        backup_path.path(),
        trusted_waypoints,
    );

    // start the backup compaction
    let compaction = Command::new(bin_path.as_path())
        .current_dir(workspace_root())
        .args([
            "velor-db",
            "backup-maintenance",
            "compact",
            "--epoch-ending-file-compact-factor",
            "2",
            "--state-snapshot-file-compact-factor",
            "2",
            "--transaction-file-compact-factor",
            "2",
            "--metadata-cache-dir",
            metadata_cache_path1.path().to_str().unwrap(),
            "--concurrent-downloads",
            "4",
            "--local-fs-dir",
            backup_path.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        compaction.status.success(),
        "{}",
        std::str::from_utf8(&compaction.stderr).unwrap()
    );
    backup_coordinator.kill().unwrap();
    let snapshot_ver = wait_res.unwrap();
    replay_verify(backup_path.path(), trusted_waypoints);
    (backup_path, snapshot_ver)
}

pub(crate) fn db_restore(
    backup_path: &Path,
    db_path: &Path,
    trusted_waypoints: &[Waypoint],
    target_verion: Option<Version>, /* target version should be same as epoch ending version to start a node */
) {
    let now = Instant::now();
    let bin_path = workspace_builder::get_bin("velor-debugger");
    let metadata_cache_path = TempPath::new();

    metadata_cache_path.create_as_dir().unwrap();

    let mut cmd = Command::new(bin_path.as_path());
    cmd.args(["velor-db", "restore", "bootstrap-db"]);
    trusted_waypoints.iter().for_each(|w| {
        cmd.arg("--trust-waypoint");
        cmd.arg(&w.to_string());
    });

    cmd.arg("--enable-storage-sharding");
    cmd.arg("--enable-state-indices");
    if let Some(version) = target_verion {
        cmd.arg("--target-version");
        cmd.arg(&version.to_string());
    }

    let status = cmd
        .args([
            "--target-db-dir",
            db_path.to_str().unwrap(),
            "--concurrent-downloads",
            "4",
            "--metadata-cache-dir",
            metadata_cache_path.path().to_str().unwrap(),
            "--local-fs-dir",
            backup_path.to_str().unwrap(),
        ])
        .current_dir(workspace_root())
        .status()
        .unwrap();
    assert!(status.success(), "{}", status);
    info!("Backup restored in {} seconds.", now.elapsed().as_secs());
}

async fn do_transfer_or_reconfig(info: &mut VelorPublicInfo) -> Result<()> {
    const LOTS_MONEY: u64 = 100_000_000;
    let r = rand::random::<u64>() % 10;
    if r < 3 {
        info!(
            "{LINE} background task: triggering reconfig. Root account seq_num: {}. Ledger info: {:?}",
            info.root_account().sequence_number(),
            info.client().get_ledger_information().await.unwrap(),
        );
        info.reconfig().await;
        info!(
            "{LINE} background task: Reconfig done. Root account seq_num: {}",
            info.root_account().sequence_number(),
        );
    } else {
        let mut sender = info.create_and_fund_user_account(LOTS_MONEY).await?;
        let receiver = info.create_and_fund_user_account(LOTS_MONEY).await?;
        let num_txns = rand::random::<usize>() % 100;
        for _ in 0..num_txns {
            info.transfer_non_blocking(&mut sender, &receiver, 1)
                .await?;
        }
    }

    Ok(())
}

async fn do_transfers_and_reconfigs(mut info: VelorPublicInfo, quit_flag: Arc<AtomicBool>) {
    // loop until aborted
    while !quit_flag.load(Ordering::Acquire) {
        do_transfer_or_reconfig(&mut info).await.unwrap();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
async fn test_db_restart() {
    ::velor_logger::Logger::new().init();

    info!("{LINE} Test started.");
    let mut swarm = SwarmBuilder::new_local(4).with_velor().build().await;
    swarm.wait_all_alive(Duration::from_secs(60)).await.unwrap();
    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_CATCH_UP_WAIT_SECS))
        .await
        .unwrap();
    info!("{LINE} Created receiver account and caught up.");

    let mut restarting_validator_ids = swarm.validators().map(|v| v.peer_id()).collect_vec();
    let non_restarting_validator_id = restarting_validator_ids.pop().unwrap();
    let non_restarting_validator = swarm.validator(non_restarting_validator_id).unwrap();
    let chain_info = swarm.chain_info();
    let mut pub_chain_info = VelorPublicInfo::new(
        chain_info.chain_id(),
        non_restarting_validator
            .inspection_service_endpoint()
            .to_string(),
        non_restarting_validator.rest_api_endpoint().to_string(),
        chain_info.root_account(),
    );
    let client = non_restarting_validator.rest_client();

    info!("{LINE} Gonna start continuous coin transfer and reconfigs in the background.");
    let quit_flag = Arc::new(AtomicBool::new(false));
    let background_traffic = tokio::task::spawn(do_transfers_and_reconfigs(
        pub_chain_info.clone(),
        quit_flag.clone(),
    ));

    for round in 0..10 {
        info!("{LINE} Restart round {round}");
        for (v, vid) in restarting_validator_ids.iter().enumerate() {
            let validator = swarm.validator_mut(*vid).unwrap();
            // sometimes trigger reconfig right before the restart, to expose edge cases around
            // epoch change
            if rand::random::<usize>() % 3 == 0 {
                info!(
                    "{LINE} Triggering reconfig right before restarting. Root account seq_num: {}. Ledger info: {:?}",
                    pub_chain_info.root_account().sequence_number(),
                    client.get_ledger_information().await.unwrap(),
                );
                reconfig(
                    &client,
                    &pub_chain_info.transaction_factory(),
                    pub_chain_info.root_account(),
                )
                .await;
                info!(
                    "{LINE} Reconfig done. Root account seq_num: {}",
                    pub_chain_info.root_account().sequence_number(),
                )
            }
            info!(
                "{LINE} Round {round}: Restarting validator {v}. ledger info: {:?}",
                client.get_ledger_information().await.unwrap(),
            );
            validator.restart().await.unwrap();
            swarm
                .wait_for_all_nodes_to_catchup(Duration::from_secs(60))
                .await
                .unwrap();
            info!("{LINE} Round {round}: Validator {v} restarted and caught up.");
            tokio::time::sleep(Duration::from_secs(10)).await;
        }
    }

    info!("{LINE} Stopping background traffic, and make sure background task didn't panic.");
    quit_flag.store(true, Ordering::Release);
    // Make sure background thread didn't panic.
    background_traffic.await.unwrap();

    info!("{LINE} Check again that all validators are alive.");
    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(60))
        .await
        .unwrap();

    info!("{LINE} All validators survived.");
    info!("{LINE} Test succeeded.");
}
