// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    smoke_test_environment::SwarmBuilder,
    storage::{db_backup, db_restore},
    utils::{
        check_create_mint_transfer_node, create_test_accounts, execute_transactions,
        execute_transactions_and_wait, swarm_utils::insert_waypoint, MAX_CATCH_UP_WAIT_SECS,
        MAX_CONNECTIVITY_WAIT_SECS, MAX_HEALTHY_WAIT_SECS,
    },
    workspace_builder,
    workspace_builder::workspace_root,
};
use anyhow::anyhow;
use velor_config::{
    config::{AdminServiceConfig, InitialSafetyRulesConfig, NodeConfig},
    network_id::NetworkId,
};
use velor_forge::{
    get_highest_synced_version, get_highest_synced_version_and_epoch,
    wait_for_all_nodes_to_catchup, LocalNode, LocalSwarm, Node, NodeExt, SwarmExt, Validator,
};
use velor_temppath::TempPath;
use velor_types::{transaction::Transaction, waypoint::Waypoint};
use move_core_types::language_storage::CORE_CODE_ADDRESS;
use regex::Regex;
use reqwest::Client;
use std::{
    fs,
    path::PathBuf,
    process::Command,
    str::FromStr,
    time::{Duration, Instant},
};

#[ignore] // TODO(joshlind): revisit the flakes once we update state sync to handle forks automatically.
#[tokio::test]
/// This test verifies:
/// 1. The behaviour of the consensus sync_only mode (to emulate a network halt).
/// 2. The flow of a genesis write-set transaction for fullnodes (after the validators have forked).
///
/// The test does the following:
/// 1. Start a 4 node validator network, including 2 VFNs.
/// 2. Use consensus `sync_only` mode to force all nodes to stop at the same version (i.e., emulate a halt).
/// 3. Use the velor CLI to generate a genesis transaction that removes the last validator from the set.
/// 4. Use the velor-debugger to manually apply the genesis transaction to all remaining validators.
/// 5. Verify that the network is able to resume consensus and that the last validator is no longer in the set.
/// 6. Use the velor-debugger to manually apply the genesis transaction to all VFNs.
/// 7. Verify that the VFNs are able to sync with the rest of the network.
async fn test_fullnode_genesis_transaction_flow() {
    println!("0. Building the Velor CLI and debugger!");
    let velor_debugger = workspace_builder::get_bin("velor-debugger");
    let velor_cli = workspace_builder::get_bin("velor");

    println!("1. Starting a 4 node validator network with 2 VFNs!");
    let num_validators = 4;
    let num_fullnodes = 2;
    let (mut swarm, cli_test_framework, _) = SwarmBuilder::new_local(num_validators)
        .with_num_fullnodes(num_fullnodes)
        .with_velor()
        .build_with_cli(0)
        .await;

    println!("2. Executing a number of test transactions");
    let validator = swarm.validators_mut().next().unwrap();
    let validator_client = validator.rest_client();
    let (mut account_0, account_1) = create_test_accounts(&mut swarm).await;
    execute_transactions_and_wait(
        &mut swarm,
        &validator_client,
        &mut account_0,
        &account_1,
        true,
    )
    .await;

    println!("3. Enabling `sync_only` mode for every validator!");
    for validator in swarm.validators_mut() {
        enable_sync_only_mode(num_validators, validator).await;
    }

    println!("4. Fetching the halt version and epoch, and stopping all validators!");
    let (halt_version, halt_epoch) =
        get_highest_synced_version_and_epoch(&swarm.get_all_nodes_clients_with_names())
            .await
            .unwrap();
    for node in swarm.validators_mut() {
        node.stop();
    }

    println!("5. Generating a genesis transaction that removes the last validator from the set!");
    let (genesis_blob_path, genesis_transaction) =
        generate_genesis_transaction(&mut swarm, velor_cli);

    println!("6. Applying the genesis transaction to the first validator!");
    let first_validator_config = swarm.validators_mut().next().unwrap().config().clone();
    let first_validator_storage_dir = first_validator_config.storage.dir();
    let output = Command::new(velor_debugger.as_path())
        .current_dir(workspace_root())
        .args(&vec![
            "velor-db",
            "bootstrap",
            first_validator_storage_dir.to_str().unwrap(),
            "--genesis-txn-file",
            genesis_blob_path.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    println!("7. Parsing the output to get the waypoint: {:?}", output);
    let output_string = std::str::from_utf8(&output.stdout).unwrap();
    let waypoint = parse_waypoint(output_string);

    println!(
        "8. Applying the genesis transaction to validators 0, 1 and 2. Waypoint: {:?}",
        waypoint
    );
    for (num_expected_peers, validator) in swarm.validators_mut().take(3).enumerate() {
        apply_genesis_to_node(
            validator,
            genesis_transaction.clone(),
            waypoint,
            num_expected_peers,
            false, // Don't test the admin service on the validators
        )
        .await;
    }

    println!("9. Verifying that we're able to resume consensus and execute transactions!");
    execute_transactions(
        &mut swarm,
        &validator_client,
        &mut account_0,
        &account_1,
        true,
    )
    .await;

    println!("10. Verifying that the last validator is no longer in the validator set!");
    let validator_set = cli_test_framework.show_validator_set().await.unwrap();
    assert_eq!(validator_set.active_validators.len(), num_validators - 1);

    println!("11. Verifying that the VFNs are stuck at the network halt! Expected epoch: {}, version: {}", halt_epoch, halt_version);
    for fullnode in swarm.fullnodes_mut() {
        // Get the current epoch and version for the fullnode
        let (current_epoch, current_version) = get_current_epoch_and_version(fullnode).await;

        // Verify that the fullnode is stuck at the network halt
        assert_eq!(current_epoch, halt_epoch);
        assert_eq!(current_version, halt_version);
    }

    println!(
        "12. Applying the genesis transaction to the VFNs. Waypoint: {:?}",
        waypoint
    );
    for (fullnode_index, fullnode) in swarm.fullnodes_mut().enumerate() {
        apply_genesis_to_node(
            fullnode,
            genesis_transaction.clone(),
            waypoint,
            1,                   // Number of expected peers
            fullnode_index == 0, // Test admin service on the first fullnode
        )
        .await;
    }

    println!("13. Verifying that the VFNs are able to sync with the validators!");
    let all_nodes = swarm.validators().take(3).chain(swarm.fullnodes());
    let all_node_clients: Vec<_> = all_nodes
        .map(|node| (node.name().to_string(), node.rest_client()))
        .collect();
    wait_for_all_nodes_to_catchup(
        &all_node_clients,
        Duration::from_secs(MAX_CATCH_UP_WAIT_SECS),
    )
    .await
    .unwrap();
}

#[tokio::test]
/// This test verifies:
/// 1. The behaviour of the consensus sync_only mode.
/// 2. The flow of a genesis write-set transaction after the chain has halted.
/// 3. That db-restore is able to restore a failed validator node.
///
/// The test does the following:
/// 1. Start a 5 node validator network.
/// 2. Enable consensus `sync_only` mode for the last validator and verify that it can sync.
/// 3. Use consensus `sync_only` mode to force all nodes to stop at the same version (i.e., emulate a halt).
/// 4. Use the velor CLI to generate a genesis transaction that removes the last validator from the set.
/// 5. Use the velor-debugger to manually apply the genesis transaction to all remaining validators.
/// 6. Verify that the network is able to resume consensus and that the last validator is no longer in the set.
/// 7. Verify that a failed validator node is able to db-restore and rejoin the network.
async fn test_validator_genesis_transaction_and_db_restore_flow() {
    println!("0. Building the Velor CLI and debugger!");
    let velor_debugger = workspace_builder::get_bin("velor-debugger");
    let velor_cli = workspace_builder::get_bin("velor");

    println!("1. Starting a 5 node validator network!");
    let num_validators = 5;
    let (mut swarm, cli_test_framework, _) = SwarmBuilder::new_local(num_validators)
        .with_velor()
        .build_with_cli(0)
        .await;

    println!("2. Enabling `sync_only` mode for the last validator and verifying that it can sync!");
    let last_validator = swarm.validators_mut().last().unwrap();
    enable_sync_only_mode(num_validators, last_validator).await;
    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_CATCH_UP_WAIT_SECS))
        .await
        .unwrap();

    println!("3. Enabling `sync_only` mode for every validator!");
    for validator in swarm.validators_mut() {
        enable_sync_only_mode(num_validators, validator).await;
    }

    println!("4. Deleting one validator's DB and verifying that it can still catch up!");
    delete_storage_and_wait_for_catchup(&mut swarm, 3).await;

    println!("5. Stopping three validator nodes!");
    for node in swarm.validators_mut().take(3) {
        node.stop();
    }

    println!("6. Generating a genesis transaction that removes the last validator from the set!");
    let (genesis_blob_path, genesis_transaction) =
        generate_genesis_transaction(&mut swarm, velor_cli);

    println!("7. Applying the genesis transaction to the first validator!");
    let first_validator_config = swarm.validators_mut().next().unwrap().config().clone();
    let first_validator_storage_dir = first_validator_config.storage.dir();
    let output = Command::new(velor_debugger.as_path())
        .current_dir(workspace_root())
        .args(&vec![
            "velor-db",
            "bootstrap",
            first_validator_storage_dir.to_str().unwrap(),
            "--genesis-txn-file",
            genesis_blob_path.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();

    println!("8. Parsing the output to get the waypoint: {:?}", output);
    let output_string = std::str::from_utf8(&output.stdout).unwrap();
    let waypoint = parse_waypoint(output_string);

    println!(
        "9. Applying the genesis transaction to validators 0, 1 and 2. Waypoint: {:?}",
        waypoint
    );
    for (num_expected_peers, validator) in swarm.validators_mut().take(3).enumerate() {
        apply_genesis_to_node(
            validator,
            genesis_transaction.clone(),
            waypoint,
            num_expected_peers,
            num_expected_peers == 0, // Test admin service on the first validator
        )
        .await;
    }

    println!("10. Verifying that we're able to resume consensus and execute transactions!");
    swarm.wait_for_startup().await.unwrap();
    check_create_mint_transfer_node(&mut swarm, 0).await;

    println!("11. Verifying that the last validator is no longer in the validator set!");
    let validator_set = cli_test_framework.show_validator_set().await.unwrap();
    assert_eq!(validator_set.active_validators.len(), num_validators - 1);

    println!("12. Deleting the DB on validator 3 and verifying that it can still catch up via db-restore!");
    delete_db_and_execute_restore(&mut swarm, 3, waypoint, num_validators).await;

    println!("13. Verifying that we're able to execute transactions on validator 3!");
    check_create_mint_transfer_node(&mut swarm, 3).await;
}

/// Applies the genesis transaction to the specified node and waits for it to become healthy
async fn apply_genesis_to_node(
    node: &mut LocalNode,
    genesis_transaction: Transaction,
    waypoint: Waypoint,
    num_expected_peers: usize,
    test_admin_service: bool,
) {
    // Insert the waypoint into the node's config
    let mut node_config = node.config().clone();
    insert_waypoint(&mut node_config, waypoint);

    // Update the genesis transaction
    node_config.execution.genesis = Some(genesis_transaction.clone());

    // If the node is a validator, reset the initial safety rules config and the sync_only flag
    if node_config.base.role.is_validator() {
        // Reset the initial safety rules config
        node_config
            .consensus
            .safety_rules
            .initial_safety_rules_config = InitialSafetyRulesConfig::None;

        // Reset the sync_only flag to false (so the validator can participate in consensus)
        node_config.consensus.sync_only = false;
    }

    // Remove the admin service override (the config optimizer should run and set the config)
    if test_admin_service {
        // TODO: is there a way we can verify the config optimizer without doing this?
        node_config.admin_service = AdminServiceConfig::default();
    }

    // Update the config and restart the node
    update_node_config_and_restart(node, node_config.clone());

    // Wait for the node to become healthy
    let network_id = if node_config.base.role.is_validator() {
        NetworkId::Validator
    } else {
        NetworkId::Vfn
    };
    wait_for_health_and_connectivity(node, network_id, num_expected_peers).await;

    // Verify that the config optimizer ran and started the admin service at the default port
    if test_admin_service {
        verify_admin_service_is_running().await;
    }
}

/// Deletes the DB on the specified validator and verifies that it can still catch up via db-restore
async fn delete_db_and_execute_restore(
    env: &mut LocalSwarm,
    validator_index: usize,
    waypoint: Waypoint,
    num_nodes: usize,
) {
    // Get the current epoch and version from the first validator
    let first_validator = env.validators_mut().next().unwrap();
    let (current_epoch, current_version) = get_current_epoch_and_version(first_validator).await;

    // Perform a DB backup on the first validator
    let first_validator_backup_port = first_validator
        .config()
        .storage
        .backup_service_address
        .port();
    let previous_epoch = current_epoch.checked_sub(1).unwrap();
    let (backup_path, _) = db_backup(
        first_validator_backup_port,
        previous_epoch,           // target epoch: most recently closed epoch
        current_version,          // target version
        current_version as usize, // txn batch size (version 0 is in its own batch)
        previous_epoch as usize,  // state snapshot interval
        &[waypoint],
    );

    // Stop the specified validator
    let validator = env.validators_mut().nth(validator_index).unwrap();
    validator.stop();

    // Disable sync_only mode on the specified validator
    let mut validator_config = validator.config().clone();
    validator_config.consensus.sync_only = false;
    validator_config
        .save_to_path(validator.config_path())
        .unwrap();

    // Delete the DB on the specified validator
    let db_dir = validator.config().storage.dir();
    fs::remove_dir_all(&db_dir).unwrap();

    // Perform a DB restore on the specified validator
    db_restore(backup_path.path(), db_dir.as_path(), &[waypoint], None);

    // Restart the validator and wait for it to become healthy
    validator.start().unwrap();
    wait_for_health_and_connectivity(validator, NetworkId::Validator, num_nodes - 2).await;

    // Wait until the validator catches up to the current version
    let client = validator.rest_client();
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

/// Deletes the DB of the specified validator and waits for it to catch up
async fn delete_storage_and_wait_for_catchup(env: &mut LocalSwarm, validator_index: usize) {
    // Stop the validator and delete the DB
    let validator = env.validators_mut().nth(validator_index).unwrap();
    validator.stop();
    validator.clear_storage().await.unwrap();

    // Restart the validator and wait for it to catch up
    validator.start().unwrap();
    env.wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_CATCH_UP_WAIT_SECS))
        .await
        .unwrap();
}

/// Enables sync_only mode for the specified validator and wait for it to become healthy
pub(crate) async fn enable_sync_only_mode(num_nodes: usize, validator_node: &mut LocalNode) {
    // Update the validator's config to enable sync_only mode
    let mut validator_config = validator_node.config().clone();
    validator_config.consensus.sync_only = true;
    update_node_config_and_restart(validator_node, validator_config.clone());

    // Wait for the validator to become healthy
    wait_for_health_and_connectivity(validator_node, NetworkId::Validator, num_nodes - 1).await;
}

/// Generates a genesis write-set transaction that removes the last validator from the set
fn generate_genesis_transaction(
    env: &mut LocalSwarm,
    velor_cli: PathBuf,
) -> (TempPath, Transaction) {
    // Get the address of the last validator
    let last_validator_address = env
        .validators()
        .last()
        .unwrap()
        .config()
        .get_peer_id()
        .unwrap();

    // Create a write-set transaction that removes the last validator from the set
    let script = format!(
        r#"
        script {{
            use velor_framework::stake;
            use velor_framework::velor_governance;
            use velor_framework::block;

            fun main(vm_signer: &signer, framework_signer: &signer) {{
                stake::remove_validators(framework_signer, &vector[@0x{}]);
                block::emit_writeset_block_event(vm_signer, @0x1);
                velor_governance::force_end_epoch(framework_signer);
            }}
    }}
    "#,
        last_validator_address.to_hex()
    );

    // Write the transaction to a temporary file
    let temp_script_path = TempPath::new();
    let mut move_script_path = temp_script_path.path().to_path_buf();
    move_script_path.set_extension("move");
    fs::write(move_script_path.as_path(), script).unwrap();

    // Determine the framework path
    let framework_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("velor-move")
        .join("framework")
        .join("velor-framework");

    // Create a temporary file to hold the genesis blob
    let genesis_blob_path = TempPath::new();
    genesis_blob_path.create_as_file().unwrap();

    // Generate the genesis write-set transaction
    Command::new(velor_cli.as_path())
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
            "--framework-local-dir",
            framework_path.as_os_str().to_str().unwrap(),
            "--assume-yes",
        ])
        .output()
        .unwrap();

    // Read the genesis transaction from the temporary file
    let genesis_transaction = {
        let buf = fs::read(genesis_blob_path.as_ref()).unwrap();
        bcs::from_bytes::<Transaction>(&buf).unwrap()
    };

    (genesis_blob_path, genesis_transaction)
}

/// Returns the current epoch and version of the specified node
/// by querying the node's REST API.
async fn get_current_epoch_and_version(node: &mut LocalNode) -> (u64, u64) {
    // Get current ledger info from the rest client
    let rest_client = node.rest_client();
    let current_ledger_info = rest_client.get_ledger_information().await.unwrap();

    // Return the current epoch and version
    let current_epoch = current_ledger_info.inner().epoch;
    let current_version = current_ledger_info.inner().version;
    (current_epoch, current_version)
}

/// Parses the waypoint from the output of the bootstrap command
fn parse_waypoint(bootstrap_command_output: &str) -> Waypoint {
    let waypoint = Regex::new(r"Got waypoint: (\d+:\w+)")
        .unwrap()
        .captures(bootstrap_command_output)
        .ok_or_else(|| {
            anyhow!("Failed to parse `velor-debugger velor-db bootstrap` waypoint output!")
        });
    Waypoint::from_str(waypoint.unwrap()[1].into()).unwrap()
}

/// Update the specified node's config and restart the node
fn update_node_config_and_restart(node: &mut LocalNode, mut config: NodeConfig) {
    // Stop the node
    node.stop();

    // Update the node's config
    let node_path = node.config_path();
    config.save_to_path(node_path).unwrap();

    // Restart the node
    node.start().unwrap();
}

/// Verifies that the admin service is running on the default port
/// and that it returns the expected response when the endpoints are disabled.
async fn verify_admin_service_is_running() {
    // Create a simple REST client
    let rest_client = Client::new();

    // Send a request to the admin service
    let default_admin_service_port = AdminServiceConfig::default().port;
    let admin_service_url = format!("http://127.0.0.1:{}", default_admin_service_port);
    let request = rest_client.get(admin_service_url.clone());

    // Verify that the admin service receives the request, and responds
    // with a message indicating that the endpoint is disabled.
    let response = request.send().await.unwrap();
    let response_string = response.text().await.unwrap();
    assert_eq!(response_string, "AdminService is not enabled.");
}

/// Wait for the specified node to become healthy and for the
/// node to connect to the specified number of peers.
async fn wait_for_health_and_connectivity(
    node: &mut LocalNode,
    network_id: NetworkId,
    num_expected_peers: usize,
) {
    // Wait for the node to become healthy
    let healthy_deadline = Instant::now()
        .checked_add(Duration::from_secs(MAX_HEALTHY_WAIT_SECS))
        .unwrap();
    node.wait_until_healthy(healthy_deadline)
        .await
        .unwrap_or_else(|err| {
            let lsof_output = Command::new("lsof").arg("-i").output().unwrap();
            panic!(
                "wait_until_healthy failed. lsof -i: {:?}: {}",
                lsof_output, err
            );
        });

    // Wait for the node to connect to the expected number of peers
    let connectivity_deadline = Instant::now()
        .checked_add(Duration::from_secs(MAX_CONNECTIVITY_WAIT_SECS))
        .unwrap();
    node.wait_for_connectivity(network_id, num_expected_peers, connectivity_deadline)
        .await
        .unwrap();
}
