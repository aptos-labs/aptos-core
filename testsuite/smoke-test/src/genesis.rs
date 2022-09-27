// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    smoke_test_environment::SwarmBuilder,
    storage::{db_backup, db_restore},
    test_utils::{check_create_mint_transfer_node, swarm_utils::insert_waypoint},
    workspace_builder,
    workspace_builder::workspace_root,
};
use anyhow::anyhow;
use aptos_config::config::NodeConfig;
use aptos_temppath::TempPath;
use aptos_types::{transaction::Transaction, waypoint::Waypoint};
use forge::{get_highest_synced_version, LocalNode, Node, NodeExt, SwarmExt};
use move_deps::move_core_types::language_storage::CORE_CODE_ADDRESS;
use regex::Regex;
use std::{fs, process::Command, str::FromStr, time::Duration};

fn update_node_config_restart(validator: &mut LocalNode, mut config: NodeConfig) {
    validator.stop();
    let node_path = validator.config_path();
    config.save(node_path).unwrap();
    validator.start().unwrap();
}

#[tokio::test]
/// This test verifies the flow of a genesis transaction after the chain starts.
/// 1. Test the consensus sync_only mode, every node should stop at the same version.
/// 2. Test the db-bootstrapper applying a manual genesis transaction (remove validator 0) on diemdb directly
/// 3. Test the nodes and clients resume working after updating waypoint
/// 4. Test a node lagging behind can sync to the waypoint
async fn test_genesis_transaction_flow() {
    let db_bootstrapper = workspace_builder::get_bin("db-bootstrapper");
    let aptos_cli = workspace_builder::get_bin("aptos");

    // prebuild tools.
    workspace_builder::get_bin("db-backup");
    workspace_builder::get_bin("db-restore");
    workspace_builder::get_bin("db-backup-verify");

    let num_nodes = 5;
    let (mut env, cli, _) = SwarmBuilder::new_local(num_nodes)
        .with_aptos()
        .build_with_cli(0)
        .await;

    println!("1. Set sync_only = true for the last node and check it can sync to others");
    let node = env.validators_mut().nth(4).unwrap();
    let mut new_config = node.config().clone();
    new_config.consensus.sync_only = true;
    update_node_config_restart(node, new_config.clone());
    // wait for some versions
    env.wait_for_all_nodes_to_catchup_to_version(10, Duration::from_secs(10))
        .await
        .unwrap();

    println!("2. Set sync_only = true for all nodes and restart");
    for node in env.validators_mut() {
        let mut node_config = node.config().clone();
        node_config.consensus.sync_only = true;
        update_node_config_restart(node, node_config)
    }

    println!("3. delete one node's db and test they can still sync when sync_only is true for every nodes");
    let node = env.validators_mut().nth(3).unwrap();
    node.stop();
    node.clear_storage().await.unwrap();
    node.start().unwrap();

    println!("4. verify all nodes are at the same round and no progress being made");
    env.wait_for_all_nodes_to_catchup(Duration::from_secs(30))
        .await
        .unwrap();

    println!("5. kill nodes and prepare a genesis txn to remove the last validator");
    for node in env.validators_mut().take(3) {
        node.stop();
    }

    let first_validator_address = env.validators().nth(4).unwrap().config().peer_id().unwrap();

    let script = format!(
        r#"
        script {{
            use aptos_framework::stake;
            use aptos_framework::aptos_governance;
            use aptos_framework::block;

            fun main(vm_signer: &signer, framework_signer: &signer) {{
                stake::remove_validators(framework_signer, &vector[@0x{:?}]);
                block::emit_writeset_block_event(vm_signer, @0x1);
                aptos_governance::reconfigure(framework_signer);
            }}
    }}
    "#,
        first_validator_address
    );

    let temp_script_path = TempPath::new();
    let mut move_script_path = temp_script_path.path().to_path_buf();
    move_script_path.set_extension("move");

    fs::write(move_script_path.as_path(), script).unwrap();

    let genesis_blob_path = TempPath::new();
    genesis_blob_path.create_as_file().unwrap();

    Command::new(aptos_cli.as_path())
        .current_dir(workspace_root())
        .args(&vec![
            "genesis",
            "generate-admin-write-set",
            "--output-file",
            genesis_blob_path.path().to_str().unwrap(),
            "--execute-as",
            CORE_CODE_ADDRESS.clone().to_hex().as_str(),
            "--script-path",
            move_script_path.as_path().to_str().unwrap(),
            "--framework-git-rev",
            "HEAD",
            "--assume-yes",
        ])
        .output()
        .unwrap();

    let genesis_transaction = {
        let buf = fs::read(genesis_blob_path.as_ref()).unwrap();
        bcs::from_bytes::<Transaction>(&buf).unwrap()
    };

    println!("6. prepare the waypoint with the transaction");
    let waypoint_command = Command::new(db_bootstrapper.as_path())
        .current_dir(workspace_root())
        .args(&vec![
            env.validators()
                .next()
                .unwrap()
                .config()
                .storage
                .dir()
                .to_str()
                .unwrap(),
            "--genesis-txn-file",
            genesis_blob_path.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    println!("Db bootstrapper output: {:?}", waypoint_command);
    let output = std::str::from_utf8(&waypoint_command.stdout).unwrap();
    let waypoint = parse_waypoint(output);

    println!("7. apply genesis transaction for nodes 0, 1, 2");
    for node in env.validators_mut().take(3) {
        let mut node_config = node.config().clone();
        insert_waypoint(&mut node_config, waypoint);
        node_config.execution.genesis = Some(genesis_transaction.clone());
        // reset the sync_only flag to false
        node_config.consensus.sync_only = false;
        update_node_config_restart(node, node_config);
    }

    println!("8. verify it's able to mint after the waypoint");
    env.wait_for_startup().await.unwrap();
    check_create_mint_transfer_node(&mut env, 0).await;

    let (epoch, version) = {
        let response = env
            .validators()
            .next()
            .unwrap()
            .rest_client()
            .get_ledger_information()
            .await
            .unwrap();
        (response.inner().epoch, response.inner().version)
    };

    let backup_path = db_backup(
        env.validators()
            .next()
            .unwrap()
            .config()
            .storage
            .backup_service_address
            .port(),
        epoch.checked_sub(1).unwrap(), // target epoch: most recently closed epoch
        version,                       // target version
        version as usize,              // txn batch size (version 0 is in its own batch)
        epoch.checked_sub(1).unwrap() as usize, // state snapshot interval
        &[waypoint],
    );

    println!("9. verify node 4 is out from the validator set");
    assert_eq!(
        cli.show_validator_set()
            .await
            .unwrap()
            .active_validators
            .len(),
        4
    );

    println!("10. nuke DB on node 3, and run db-restore, test if it rejoins the network okay.");
    let node = env.validators_mut().nth(3).unwrap();
    node.stop();
    let mut node_config = node.config().clone();
    node_config.consensus.sync_only = false;
    node_config.save(node.config_path()).unwrap();

    let db_dir = node.config().storage.dir();
    fs::remove_dir_all(&db_dir).unwrap();
    db_restore(backup_path.path(), db_dir.as_path(), &[waypoint]);

    node.start().unwrap();
    let client = node.rest_client();
    // wait for it to catch up
    {
        let version = get_highest_synced_version(&env.get_all_nodes_clients_with_names())
            .await
            .unwrap();
        loop {
            if let Ok(resp) = client.get_ledger_information().await {
                if resp.into_inner().version > version {
                    println!("Node 3 catches up on {}", version);
                    break;
                }
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    check_create_mint_transfer_node(&mut env, 3).await;
}

fn parse_waypoint(db_bootstrapper_output: &str) -> Waypoint {
    let waypoint = Regex::new(r"Got waypoint: (\d+:\w+)")
        .unwrap()
        .captures(db_bootstrapper_output)
        .ok_or_else(|| anyhow!("Failed to parse db-bootstrapper output."));
    Waypoint::from_str(waypoint.unwrap()[1].into()).unwrap()
}
