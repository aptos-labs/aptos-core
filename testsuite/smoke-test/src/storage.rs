// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::smoke_test_environment::SwarmBuilder;
use crate::{
    test_utils::{
        assert_balance, create_and_fund_account, swarm_utils::insert_waypoint,
        transfer_and_reconfig, transfer_coins,
    },
    workspace_builder,
    workspace_builder::workspace_root,
};
use anyhow::{bail, Result};
use aptos_logger::info;
use aptos_temppath::TempPath;
use aptos_types::{transaction::Version, waypoint::Waypoint};
use backup_cli::metadata::view::BackupStorageState;
use forge::{reconfig, NodeExt, Swarm, SwarmExt};
use std::{
    fs,
    path::Path,
    process::Command,
    time::{Duration, Instant},
};

const MAX_WAIT_SECS: u64 = 180;

#[tokio::test]
async fn test_db_restore() {
    // pre-build tools
    ::aptos_logger::Logger::new().init();
    info!("---------- 0. test_db_restore started.");
    workspace_builder::get_bin("db-backup");
    workspace_builder::get_bin("db-restore");
    workspace_builder::get_bin("db-backup-verify");
    info!("---------- 1. pre-building finished.");

    let mut swarm = SwarmBuilder::new_local(4).with_aptos().build().await;
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
        .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_WAIT_SECS))
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

    expected_balance_0 -= 6;
    expected_balance_1 += 6;

    transfer_and_reconfig(
        &client_1,
        &transaction_factory,
        swarm.chain_info().root_account,
        &mut account_0,
        &account_1,
        3,
    )
    .await;
    // we are at least at epoch 2
    reconfig(
        &client_1,
        &transaction_factory,
        swarm.chain_info().root_account,
    )
    .await;
    transfer_and_reconfig(
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

    info!("---------- 2. reached at least epoch 2, starting backup coordinator.");
    // make a backup from node 1
    let node1_config = swarm.validator(validator_peer_ids[1]).unwrap().config();
    let port = node1_config.storage.backup_service_address.port();
    let backup_path = db_backup(port, 2, 10, 2, 1, &[]);

    // take down node 0
    let node_to_restart = validator_peer_ids[0];
    swarm.validator_mut(node_to_restart).unwrap().stop();

    // nuke db
    let node0_config_path = swarm.validator(node_to_restart).unwrap().config_path();
    let mut node0_config = swarm.validator(node_to_restart).unwrap().config().clone();
    let genesis_waypoint = node0_config.base.waypoint.genesis_waypoint();
    insert_waypoint(&mut node0_config, genesis_waypoint);
    node0_config.save(node0_config_path).unwrap();
    let db_dir = node0_config.storage.dir();
    fs::remove_dir_all(db_dir.clone()).unwrap();

    info!("---------- 3. stopped node 0, gonna restore DB.");
    // restore db from backup
    db_restore(backup_path.path(), db_dir.as_path(), &[]);

    expected_balance_0 -= 3;
    expected_balance_1 += 3;

    transfer_and_reconfig(
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
        .wait_until_healthy(Instant::now() + Duration::from_secs(MAX_WAIT_SECS))
        .await
        .unwrap();
    info!("---------- 5. Node 0 is health, verify it's caught up.");
    // verify it's caught up
    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_WAIT_SECS))
        .await
        .unwrap();

    let client_0 = swarm.validator(node_to_restart).unwrap().rest_client();

    assert_balance(&client_0, &account_0, expected_balance_0).await;
    assert_balance(&client_0, &account_1, expected_balance_1).await;
    info!("6. Done");
}

fn db_backup_verify(backup_path: &Path, trusted_waypoints: &[Waypoint]) {
    let now = Instant::now();
    let bin_path = workspace_builder::get_bin("db-backup-verify");
    let metadata_cache_path = TempPath::new();

    metadata_cache_path.create_as_dir().unwrap();

    let mut cmd = Command::new(bin_path.as_path());

    trusted_waypoints.iter().for_each(|w| {
        cmd.arg("--trust-waypoint");
        cmd.arg(&w.to_string());
    });

    let output = cmd
        .args(&[
            "--metadata-cache-dir",
            metadata_cache_path.path().to_str().unwrap(),
            "local-fs",
            "--dir",
            backup_path.to_str().unwrap(),
        ])
        .current_dir(workspace_root())
        .output()
        .unwrap();
    if !output.status.success() {
        panic!("db-backup-verify failed, output: {:?}", output);
    }
    info!("Backup verified in {} seconds.", now.elapsed().as_secs());
}

fn wait_for_backups(
    target_epoch: u64,
    target_version: u64,
    now: Instant,
    bin_path: &Path,
    metadata_cache_path: &Path,
    backup_path: &Path,
    trusted_waypoints: &[Waypoint],
) -> Result<()> {
    for i in 0..120 {
        // the verify should always succeed.
        db_backup_verify(backup_path, trusted_waypoints);

        info!(
            "{}th wait for the backup to reach epoch {}, version {}.",
            i, target_epoch, target_version,
        );
        let output = Command::new(bin_path)
            .current_dir(workspace_root())
            .args(&[
                "one-shot",
                "query",
                "backup-storage-state",
                "--metadata-cache-dir",
                metadata_cache_path.to_str().unwrap(),
                "local-fs",
                "--dir",
                backup_path.to_str().unwrap(),
            ])
            .output()?
            .stdout;
        let state: BackupStorageState = std::str::from_utf8(&output)?.parse()?;
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
            return Ok(());
        }
        info!("Backup storage state: {}", state);
        std::thread::sleep(Duration::from_secs(1));
    }

    bail!("Failed to create backup.");
}

pub(crate) fn db_backup(
    backup_service_port: u16,
    target_epoch: u64,
    target_version: Version,
    transaction_batch_size: usize,
    state_snapshot_interval_epochs: usize,
    trusted_waypoints: &[Waypoint],
) -> TempPath {
    let now = Instant::now();
    let bin_path = workspace_builder::get_bin("db-backup");
    let metadata_cache_path1 = TempPath::new();
    let metadata_cache_path2 = TempPath::new();
    let backup_path = TempPath::new();

    metadata_cache_path1.create_as_dir().unwrap();
    metadata_cache_path2.create_as_dir().unwrap();
    backup_path.create_as_dir().unwrap();

    // spawn the backup coordinator
    let mut backup_coordinator = Command::new(bin_path.as_path())
        .current_dir(workspace_root())
        .args(&[
            "coordinator",
            "run",
            "--backup-service-address",
            &format!("http://localhost:{}", backup_service_port),
            "--transaction-batch-size",
            &transaction_batch_size.to_string(),
            "--state-snapshot-interval-epochs",
            &state_snapshot_interval_epochs.to_string(),
            "--metadata-cache-dir",
            metadata_cache_path1.path().to_str().unwrap(),
            "local-fs",
            "--dir",
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
    backup_coordinator.kill().unwrap();
    wait_res.unwrap();
    backup_path
}

pub(crate) fn db_restore(backup_path: &Path, db_path: &Path, trusted_waypoints: &[Waypoint]) {
    let now = Instant::now();
    let bin_path = workspace_builder::get_bin("db-restore");
    let metadata_cache_path = TempPath::new();

    metadata_cache_path.create_as_dir().unwrap();

    let mut cmd = Command::new(bin_path.as_path());
    trusted_waypoints.iter().for_each(|w| {
        cmd.arg("--trust-waypoint");
        cmd.arg(&w.to_string());
    });

    let output = cmd
        .args(&[
            "--target-db-dir",
            db_path.to_str().unwrap(),
            "auto",
            "--metadata-cache-dir",
            metadata_cache_path.path().to_str().unwrap(),
            "local-fs",
            "--dir",
            backup_path.to_str().unwrap(),
        ])
        .current_dir(workspace_root())
        .output()
        .unwrap();
    if !output.status.success() {
        panic!("db-restore failed, output: {:?}", output);
    }
    info!("Backup restored in {} seconds.", now.elapsed().as_secs());
}
