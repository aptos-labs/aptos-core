// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    smoke_test_environment::new_local_swarm_with_aptos,
    test_utils::{
        swarm_utils::{create_root_storage, load_validators_backend_storage},
        write_key_to_file_bcs_format, write_key_to_file_hex_format,
    },
};
use anyhow::{bail, Result};
use aptos::{
    common::utils::append_file_extension,
    op::key::{GenerateKey, PUBLIC_KEY_EXTENSION},
};
use aptos_config::{
    config::{PeerRole, SecureBackend},
    network_id::NetworkId,
};
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    x25519, HashValue, PrivateKey, Uniform, ValidCryptoMaterialStringExt,
};
use aptos_global_constants::{
    CONSENSUS_KEY, FULLNODE_NETWORK_KEY, GENESIS_WAYPOINT, OPERATOR_ACCOUNT, OPERATOR_KEY,
    OWNER_ACCOUNT, OWNER_KEY, VALIDATOR_NETWORK_KEY, WAYPOINT,
};
use aptos_management::storage::to_x25519;
use aptos_operational_tool::{
    keys::{EncodingType, KeyType},
    test_helper::OperationalTool,
};
use aptos_rest_client::Client as RestClient;
use aptos_sdk::move_types::move_resource::MoveResource;
use aptos_secure_storage::{CryptoStorage, KVStorage, Storage};
use aptos_temppath::TempPath;
use aptos_types::{
    account_address::{from_identity_public_key, AccountAddress},
    block_info::BlockInfo,
    ledger_info::LedgerInfo,
    network_address::NetworkAddress,
    transaction::authenticator::AuthenticationKey,
    validator_config::ValidatorOperatorConfigResource,
    waypoint::Waypoint,
};
use forge::{LocalNode, LocalSwarm, Node, NodeExt, SwarmExt};
use rand::rngs::OsRng;
use std::{
    collections::HashSet,
    convert::{TryFrom, TryInto},
    fs,
    path::{Path, PathBuf},
    str::FromStr,
    time::{Duration, Instant},
};

async fn test_account_resource(
    _swarm: &LocalSwarm,
    op_tool: &OperationalTool,
    _backend: &SecureBackend,
    storage: &mut Storage,
) {
    // Fetch the owner account resource
    let owner_account = storage.get::<AccountAddress>(OWNER_ACCOUNT).unwrap().value;
    let account_resource = op_tool.account_resource(owner_account).await.unwrap();
    assert_eq!(owner_account, account_resource.account);
    assert_eq!(0, account_resource.sequence_number);

    // Fetch the operator account resource
    let operator_account = storage
        .get::<AccountAddress>(OPERATOR_ACCOUNT)
        .unwrap()
        .value;
    let account_resource = op_tool.account_resource(operator_account).await.unwrap();
    assert_eq!(operator_account, account_resource.account);
    assert_eq!(0, account_resource.sequence_number);

    // Verify operator key
    let on_chain_operator_key = hex::decode(account_resource.authentication_key).unwrap();
    let operator_key = storage.get_public_key(OPERATOR_KEY).unwrap().public_key;
    assert_eq!(
        AuthenticationKey::ed25519(&operator_key),
        AuthenticationKey::try_from(on_chain_operator_key).unwrap()
    );
}

// TODO(https://github.com/aptos-labs/aptos-core/issues/317)
#[ignore]
#[tokio::test]
async fn test_auto_validate_options() {
    let (swarm, op_tool, backend, _storage) = launch_swarm_with_op_tool_and_backend(1).await;

    // Rotate the operator key with a really low timeout to prevent validation
    let (txn_ctx, _) = op_tool
        .rotate_operator_key_with_custom_validation(&backend, false, Some(1), Some(0))
        .await
        .unwrap();
    assert!(txn_ctx.execution_result.is_none());

    // Now wait for transaction execution
    let client = swarm.validators().next().unwrap().rest_client();
    wait_for_account_sequence_number(&client, txn_ctx.address, txn_ctx.sequence_number)
        .await
        .unwrap();

    // Verify that the transaction was executed correctly
    let txn_ctx = op_tool
        .validate_transaction(txn_ctx.address, txn_ctx.sequence_number)
        .await
        .unwrap();
    assert!(txn_ctx.execution_result.unwrap().success);

    // Rotate the operator key with a custom timeout of 1 minute and a a custom sleep interval
    let (txn_ctx, _) = op_tool
        .rotate_operator_key_with_custom_validation(&backend, false, Some(2), Some(60))
        .await
        .unwrap();
    assert!(txn_ctx.execution_result.unwrap().success);
}

// TODO(https://github.com/aptos-labs/aptos-core/issues/317)
#[ignore]
#[tokio::test]
async fn test_consensus_key_rotation() {
    let (_swarm, op_tool, backend, mut storage) = launch_swarm_with_op_tool_and_backend(1).await;

    // Rotate the consensus key
    let (txn_ctx, new_consensus_key) = op_tool.rotate_consensus_key(&backend, false).await.unwrap();
    assert!(txn_ctx.execution_result.unwrap().success);

    // Verify that the config has been updated correctly with the new consensus key
    let validator_account = storage.get::<AccountAddress>(OWNER_ACCOUNT).unwrap().value;
    let config_consensus_key = op_tool
        .validator_config(validator_account, Some(&backend))
        .await
        .unwrap()
        .consensus_public_key;
    assert_eq!(new_consensus_key, config_consensus_key);

    // Verify that the validator set info contains the new consensus key
    let info_consensus_key = op_tool
        .validator_set(Some(validator_account), Some(&backend))
        .await
        .unwrap()[0]
        .consensus_public_key
        .clone();
    assert_eq!(new_consensus_key, info_consensus_key);

    // Rotate the consensus key in storage manually and perform another rotation using the op_tool.
    // Here, we expected the op_tool to see that the consensus key in storage doesn't match the one
    // on-chain, and thus it should simply forward a transaction to the blockchain.
    let rotated_consensus_key = storage.rotate_key(CONSENSUS_KEY).unwrap();
    let (txn_ctx, new_consensus_key) = op_tool.rotate_consensus_key(&backend, true).await.unwrap();
    assert!(txn_ctx.execution_result.is_none());
    assert_eq!(rotated_consensus_key, new_consensus_key);
}

async fn test_create_operator_hex_file(
    swarm: &mut LocalSwarm,
    op_tool: &OperationalTool,
    backend: &SecureBackend,
    storage: &mut Storage,
) {
    create_operator_with_file_writer(
        write_key_to_file_hex_format,
        swarm,
        op_tool,
        backend,
        storage,
    )
    .await;
}

async fn test_create_operator_bcs_file(
    swarm: &mut LocalSwarm,
    op_tool: &OperationalTool,
    backend: &SecureBackend,
    storage: &mut Storage,
) {
    create_operator_with_file_writer(
        write_key_to_file_bcs_format,
        swarm,
        op_tool,
        backend,
        storage,
    )
    .await;
}

async fn test_create_validator_hex_file(
    swarm: &mut LocalSwarm,
    op_tool: &OperationalTool,
    backend: &SecureBackend,
    storage: &mut Storage,
) {
    create_validator_with_file_writer(
        write_key_to_file_hex_format,
        swarm,
        op_tool,
        backend,
        storage,
    )
    .await;
}

async fn test_create_validator_bcs_file(
    swarm: &mut LocalSwarm,
    op_tool: &OperationalTool,
    backend: &SecureBackend,
    storage: &mut Storage,
) {
    create_validator_with_file_writer(
        write_key_to_file_bcs_format,
        swarm,
        op_tool,
        backend,
        storage,
    )
    .await;
}

// TODO(https://github.com/aptos-labs/aptos-core/issues/317)
#[ignore]
#[tokio::test]
async fn test_disable_address_validation() {
    let (_swarm, op_tool, backend, _storage) = launch_swarm_with_op_tool_and_backend(1).await;

    // Try to set the validator config with a bad address and verify failure
    let bad_network_address = NetworkAddress::from_str("/dns4/127.0.0.1/tcp/1234").unwrap();
    op_tool
        .set_validator_config(
            Some(bad_network_address.clone()),
            None,
            &backend,
            false,
            false,
        )
        .await
        .unwrap_err();

    // Now disable address verification to set the validator config with a bad network address
    let txn_ctx = op_tool
        .set_validator_config(Some(bad_network_address), None, &backend, false, true)
        .await
        .unwrap();
    assert!(txn_ctx.execution_result.unwrap().success);

    // Rotate the consensus key and verify that it isn't blocked by a bad network address
    let _ = op_tool.rotate_consensus_key(&backend, false).await.unwrap();

    // Rotate the validator network key and verify that it isn't blocked by a bad network address
    let _ = op_tool
        .rotate_validator_network_key(&backend, false)
        .await
        .unwrap();

    // Rotate the fullnode network key and verify that it isn't blocked by a bad network address
    let _ = op_tool
        .rotate_fullnode_network_key(&backend, false)
        .await
        .unwrap();

    // Rotate the operator key and verify that it isn't blocked by a bad network address
    let _ = op_tool.rotate_operator_key(&backend, false).await.unwrap();

    // Update the validator network address with a valid address
    let new_network_address = NetworkAddress::from_str("/ip4/10.0.0.16/tcp/80").unwrap();
    let _ = op_tool
        .set_validator_config(Some(new_network_address), None, &backend, false, false)
        .await
        .unwrap();
}

// TODO(https://github.com/aptos-labs/aptos-core/issues/317)
#[ignore]
#[tokio::test]
async fn test_set_operator_and_add_new_validator() {
    let num_nodes = 3;
    let (mut swarm, op_tool, _, _) = launch_swarm_with_op_tool_and_backend(num_nodes).await;

    // Create new validator and validator operator keys and accounts
    let (validator_key, validator_account) = create_new_test_account();
    let (operator_key, operator_account) = create_new_test_account();

    // Write the validator key to a file and create the validator account
    let validator_key_path = write_key_to_file(
        &validator_key.public_key(),
        swarm.validators().next().unwrap(),
        write_key_to_file_hex_format,
    );
    let root_backend = create_root_storage(&mut swarm);
    let val_human_name = "new_validator";
    let (txn_ctx, _) = op_tool
        .create_validator(
            val_human_name,
            validator_key_path.to_str().unwrap(),
            &root_backend,
            false,
        )
        .await
        .unwrap();
    assert!(txn_ctx.execution_result.unwrap().success);

    let validator = swarm.validators().next().unwrap();
    // Write the operator key to a file and create the operator account
    let operator_key_path = write_key_to_file(
        &operator_key.public_key(),
        validator,
        write_key_to_file_bcs_format,
    );
    let op_human_name = "new_operator";
    let (txn_ctx, _) = op_tool
        .create_validator_operator(
            op_human_name,
            operator_key_path.to_str().unwrap(),
            &root_backend,
            true,
        )
        .await
        .unwrap();

    // Wait for transaction execution
    let client = validator.rest_client();
    wait_for_account_sequence_number(&client, txn_ctx.address, txn_ctx.sequence_number)
        .await
        .unwrap();

    // Verify that the transaction was executed
    let txn_ctx = op_tool
        .validate_transaction(txn_ctx.address, txn_ctx.sequence_number)
        .await
        .unwrap();
    assert!(txn_ctx.execution_result.unwrap().success);

    // Overwrite the keys in storage to execute the command from the new validator's perspective
    let backend = load_validators_backend_storage(validator);
    let mut storage: Storage = (&backend).try_into().unwrap();
    storage.set(OWNER_ACCOUNT, validator_account).unwrap();
    storage
        .import_private_key(OWNER_KEY, validator_key)
        .unwrap();

    // TODO: Add check to Verify no validator operator when this test is enabled

    // Set the validator operator
    let txn_ctx = op_tool
        .set_validator_operator(op_human_name, operator_account, &backend, true)
        .await
        .unwrap();
    assert!(txn_ctx.execution_result.is_none());

    // Wait for transaction execution
    wait_for_account_sequence_number(&client, txn_ctx.address, txn_ctx.sequence_number)
        .await
        .unwrap();

    // Overwrite the keys in storage to execute the command from the new operator's perspective
    storage.set(OPERATOR_ACCOUNT, operator_account).unwrap();
    storage
        .import_private_key(OPERATOR_KEY, operator_key)
        .unwrap();

    // Set the validator config
    let network_address = Some(NetworkAddress::from_str("/ip4/10.0.0.16/tcp/80").unwrap());
    let txn_ctx = op_tool
        .set_validator_config(
            network_address.clone(),
            network_address,
            &backend,
            true,
            false,
        )
        .await
        .unwrap();
    assert!(txn_ctx.execution_result.is_none());

    // Wait for transaction execution
    wait_for_account_sequence_number(&client, txn_ctx.address, txn_ctx.sequence_number)
        .await
        .unwrap();

    // TODO: Add check to verify the operator has been set correctly when this test is enabled

    // Check the validator set size
    let validator_set_infos = op_tool.validator_set(None, Some(&backend)).await.unwrap();
    assert_eq!(num_nodes, validator_set_infos.len());
    assert!(!validator_set_infos
        .iter()
        .any(|info| info.account_address == validator_account));

    // Add the validator to the validator set
    let txn_ctx = op_tool
        .add_validator(validator_account, &root_backend, true)
        .await
        .unwrap();

    // Wait for transaction execution
    wait_for_account_sequence_number(&client, txn_ctx.address, txn_ctx.sequence_number)
        .await
        .unwrap();
    // Verify that the transaction wasn't executed
    let txn_ctx = op_tool
        .validate_transaction(txn_ctx.address, txn_ctx.sequence_number)
        .await
        .unwrap();
    assert!(txn_ctx.execution_result.unwrap().success);

    // Check the new validator has been added to the set
    let validator_set_infos = op_tool.validator_set(None, Some(&backend)).await.unwrap();
    assert_eq!(num_nodes + 1, validator_set_infos.len());
    let validator_info = validator_set_infos
        .iter()
        .find(|info| info.account_address == validator_account)
        .unwrap();
    assert_eq!(validator_account, validator_info.account_address);
    assert_eq!(val_human_name, validator_info.name);

    // Try and add the same validator again and watch it fail
    let txn_ctx = op_tool
        .add_validator(validator_account, &root_backend, false)
        .await
        .unwrap();
    assert!(!txn_ctx.execution_result.unwrap().success);
}

// TODO(https://github.com/aptos-labs/aptos-core/issues/317)
#[ignore]
#[tokio::test]
// Because each test takes non-neglible time to start, streamlining them into a single test
async fn test_single_node_operations() {
    let (mut swarm, op_tool, backend, mut storage) = launch_swarm_with_op_tool_and_backend(1).await;

    test_account_resource(&swarm, &op_tool, &backend, &mut storage).await;
    test_create_operator_bcs_file(&mut swarm, &op_tool, &backend, &mut storage).await;
    test_create_operator_hex_file(&mut swarm, &op_tool, &backend, &mut storage).await;
    test_create_validator_bcs_file(&mut swarm, &op_tool, &backend, &mut storage).await;
    test_create_validator_hex_file(&mut swarm, &op_tool, &backend, &mut storage).await;
    test_extract_private_key(&swarm, &op_tool, &backend, &mut storage).await;
    test_extract_public_key(&swarm, &op_tool, &backend, &mut storage).await;
    test_extract_peer_from_storage(&swarm, &op_tool, &backend, &storage).await;
    test_insert_waypoint(&swarm, &op_tool, &backend, &mut storage).await;
    test_print_account(&swarm, &op_tool, &backend, &mut storage).await;
    test_print_key(&swarm, &op_tool, &backend, &mut storage).await;
    test_print_waypoints(&swarm, &op_tool, &backend, &mut storage).await;
    test_verify_validator_state(&swarm, &op_tool, &backend, &mut storage).await;
}

async fn test_extract_private_key(
    swarm: &LocalSwarm,
    op_tool: &OperationalTool,
    backend: &SecureBackend,
    storage: &mut Storage,
) {
    // Extract the operator private key to file
    let node_config_path = swarm.validators().next().unwrap().config_path();
    let key_file_path = node_config_path.with_file_name(OPERATOR_KEY);
    let _ = op_tool
        .extract_private_key(
            OPERATOR_KEY,
            key_file_path.to_str().unwrap(),
            KeyType::Ed25519,
            EncodingType::BCS,
            backend,
        )
        .await
        .unwrap();

    // Verify the operator private key has been written correctly
    let file_contents = fs::read(key_file_path).unwrap();
    let key_from_file = bcs::from_bytes(&file_contents).unwrap();
    let key_from_storage = storage.export_private_key(OPERATOR_KEY).unwrap();
    assert_eq!(key_from_storage, key_from_file);
}

async fn test_extract_public_key(
    swarm: &LocalSwarm,
    op_tool: &OperationalTool,
    backend: &SecureBackend,
    storage: &mut Storage,
) {
    // Extract the operator public key to file
    let node_config_path = swarm.validators().next().unwrap().config_path();
    let key_file_path = node_config_path.with_file_name(OPERATOR_KEY);
    let _ = op_tool
        .extract_public_key(
            OPERATOR_KEY,
            key_file_path.to_str().unwrap(),
            KeyType::Ed25519,
            EncodingType::BCS,
            backend,
        )
        .await
        .unwrap();

    // Verify the operator key has been written correctly
    let file_contents = fs::read(key_file_path).unwrap();
    let key_from_file = bcs::from_bytes(&file_contents).unwrap();
    let key_from_storage = storage.get_public_key(OPERATOR_KEY).unwrap().public_key;
    assert_eq!(key_from_storage, key_from_file);
}

async fn test_extract_peer_from_storage(
    swarm: &LocalSwarm,
    op_tool: &OperationalTool,
    backend: &SecureBackend,
    _storage: &Storage,
) {
    // Check Validator Network Key
    let config = swarm.validators().next().unwrap().config().clone();
    let map = op_tool
        .extract_peer_from_storage(VALIDATOR_NETWORK_KEY, backend)
        .await
        .unwrap();
    let network_config = config.validator_network.unwrap();
    let expected_peer_id = network_config.peer_id();
    let expected_public_key = network_config.identity_key().public_key();
    let (peer_id, peer) = map.iter().next().unwrap();
    assert_eq!(expected_public_key, *peer.keys.iter().next().unwrap());
    assert_eq!(expected_peer_id, *peer_id);

    // Check FullNode Network Key
    let map = op_tool
        .extract_peer_from_storage(FULLNODE_NETWORK_KEY, backend)
        .await
        .unwrap();
    let network_config = config
        .full_node_networks
        .iter()
        .find(|network| network.network_id == NetworkId::Public)
        .unwrap();
    let expected_peer_id = network_config.peer_id();
    let expected_public_key = network_config.identity_key().public_key();
    let (peer_id, peer) = map.iter().next().unwrap();
    assert_eq!(expected_public_key, *peer.keys.iter().next().unwrap());
    assert_eq!(expected_peer_id, *peer_id);
}

// TODO(https://github.com/aptos-labs/aptos-core/issues/317)
#[ignore]
#[tokio::test]
async fn test_extract_peer_from_file() {
    let op_tool = OperationalTool::test();
    let path = TempPath::new();
    path.create_as_file().unwrap();
    let key = generate_x25519_key(path.as_ref()).await;

    let peer = op_tool
        .extract_peer_from_file(path.as_ref(), EncodingType::Hex)
        .await
        .unwrap();
    assert_eq!(1, peer.len());
    let (peer_id, peer) = peer.iter().next().unwrap();
    let public_key = key.public_key();
    assert_eq!(public_key, *peer.keys.iter().next().unwrap());
    assert_eq!(from_identity_public_key(public_key), *peer_id);
}

// TODO(https://github.com/aptos-labs/aptos-core/issues/317)
#[ignore]
#[tokio::test]
async fn test_extract_peers_from_keys() {
    let op_tool = OperationalTool::test();
    let output_path = TempPath::new();
    output_path.create_as_file().unwrap();

    let mut keys = HashSet::new();
    for _ in 1..10 {
        let key_path = TempPath::new();
        key_path.create_as_file().unwrap();
        let key = generate_x25519_key(key_path.as_ref()).await;
        keys.insert(key.public_key());
    }
    let peers = op_tool
        .extract_peers_from_keys(keys.clone(), output_path.as_ref())
        .await
        .unwrap();
    assert_eq!(keys.len(), peers.len());
    for key in keys {
        let address = from_identity_public_key(key);
        let peer = peers.get(&address).unwrap();
        let keys = &peer.keys;

        assert_eq!(1, keys.len());
        assert!(keys.contains(&key));
        assert_eq!(PeerRole::Upstream, peer.role);
        assert!(peer.addresses.is_empty());
    }
}

// TODO(https://github.com/aptos-labs/aptos-core/issues/317)
#[ignore]
#[tokio::test]
async fn test_generate_key() {
    let path = TempPath::new();
    path.create_as_file().unwrap();
    let pub_path = append_file_extension(path.as_ref(), PUBLIC_KEY_EXTENSION).unwrap();

    // Base64
    let (priv_key, pub_key) =
        GenerateKey::generate_x25519(aptos::common::types::EncodingType::Base64, path.as_ref())
            .await
            .unwrap();
    let read_priv_key = x25519::PrivateKey::try_from(
        base64::decode(fs::read(path.as_ref()).unwrap())
            .unwrap()
            .as_slice(),
    )
    .unwrap();
    let read_pub_key = x25519::PublicKey::try_from(
        base64::decode(fs::read(&pub_path).unwrap())
            .unwrap()
            .as_slice(),
    )
    .unwrap();
    assert_eq!(priv_key, read_priv_key);
    assert_eq!(pub_key, read_priv_key.public_key());
    assert_eq!(pub_key, read_pub_key);

    // Hex
    let (priv_key, pub_key) =
        GenerateKey::generate_x25519(aptos::common::types::EncodingType::Hex, path.as_ref())
            .await
            .unwrap();
    let read_priv_key = x25519::PrivateKey::from_encoded_string(
        &String::from_utf8(fs::read(path.as_ref()).unwrap()).unwrap(),
    )
    .unwrap();
    let read_pub_key = x25519::PublicKey::from_encoded_string(
        &String::from_utf8(fs::read(&pub_path).unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(priv_key, read_priv_key);
    assert_eq!(pub_key, read_priv_key.public_key());
    assert_eq!(pub_key, read_pub_key);

    // BCS
    let (priv_key, pub_key) =
        GenerateKey::generate_x25519(aptos::common::types::EncodingType::BCS, path.as_ref())
            .await
            .unwrap();
    let read_priv_key = bcs::from_bytes(&fs::read(path.as_ref()).unwrap()).unwrap();
    let read_pub_key = bcs::from_bytes(&fs::read(&pub_path).unwrap()).unwrap();
    assert_eq!(priv_key, read_priv_key);
    assert_eq!(pub_key, read_priv_key.public_key());
    assert_eq!(pub_key, read_pub_key);

    // BCS ed25519
    let (priv_key, pub_key) =
        GenerateKey::generate_ed25519(aptos::common::types::EncodingType::BCS, path.as_ref())
            .await
            .unwrap();
    let read_priv_key = bcs::from_bytes(&fs::read(path.as_ref()).unwrap()).unwrap();
    let read_pub_key = bcs::from_bytes(&fs::read(&pub_path).unwrap()).unwrap();
    assert_eq!(priv_key, read_priv_key);
    assert_eq!(pub_key, read_priv_key.public_key());
    assert_eq!(pub_key, read_pub_key);
}

async fn test_insert_waypoint(
    _swarm: &LocalSwarm,
    op_tool: &OperationalTool,
    backend: &SecureBackend,
    storage: &mut Storage,
) {
    // Get the current waypoint from storage
    let current_waypoint: Waypoint = storage.get(WAYPOINT).unwrap().value;

    // Insert a new waypoint and genesis waypoint into storage
    let inserted_waypoint =
        Waypoint::new_any(&LedgerInfo::new(BlockInfo::empty(), HashValue::zero()));
    assert_ne!(current_waypoint, inserted_waypoint);
    op_tool
        .insert_waypoint(inserted_waypoint, backend, true)
        .await
        .unwrap();

    // Verify the waypoint has changed in storage and that genesis waypoint is now set
    assert_eq!(inserted_waypoint, storage.get(WAYPOINT).unwrap().value);
    assert_eq!(
        inserted_waypoint,
        storage.get(GENESIS_WAYPOINT).unwrap().value
    );

    // Insert the old waypoint into storage, but skip the genesis waypoint
    op_tool
        .insert_waypoint(current_waypoint, backend, false)
        .await
        .unwrap();
    assert_eq!(current_waypoint, storage.get(WAYPOINT).unwrap().value);
    assert_eq!(
        inserted_waypoint,
        storage.get(GENESIS_WAYPOINT).unwrap().value
    );
}

// TODO(https://github.com/aptos-labs/aptos-core/issues/317)
#[ignore]
#[tokio::test]
async fn test_fullnode_network_key_rotation() {
    let (swarm, op_tool, backend, storage) = launch_swarm_with_op_tool_and_backend(1).await;

    // Rotate the full node network key
    let (txn_ctx, new_network_key) = op_tool
        .rotate_fullnode_network_key(&backend, true)
        .await
        .unwrap();
    assert!(txn_ctx.execution_result.is_none());

    // Wait for transaction execution
    let client = swarm.validators().next().unwrap().rest_client();
    wait_for_account_sequence_number(&client, txn_ctx.address, txn_ctx.sequence_number)
        .await
        .unwrap();

    // Verify that the config has been loaded correctly with new key
    let validator_account = storage.get::<AccountAddress>(OWNER_ACCOUNT).unwrap().value;
    let config_network_key = op_tool
        .validator_config(validator_account, Some(&backend))
        .await
        .unwrap()
        .fullnode_network_address
        .find_noise_proto()
        .unwrap();
    assert_eq!(new_network_key, config_network_key);

    // Verify that the validator set info contains the new network key
    let info_network_key = op_tool
        .validator_set(Some(validator_account), Some(&backend))
        .await
        .unwrap()[0]
        .fullnode_network_address
        .find_noise_proto()
        .unwrap();
    assert_eq!(new_network_key, info_network_key);
}

// TODO(https://github.com/aptos-labs/aptos-core/issues/317)
#[ignore]
#[tokio::test]
async fn test_network_key_rotation() {
    let num_nodes = 4;
    let (mut swarm, op_tool, backend, storage) =
        launch_swarm_with_op_tool_and_backend(num_nodes).await;

    // Rotate the validator network key
    let (txn_ctx, new_network_key) = op_tool
        .rotate_validator_network_key(&backend, true)
        .await
        .unwrap();
    assert!(txn_ctx.execution_result.is_none());

    // Ensure all nodes have received the transaction
    wait_for_transaction_on_all_nodes(&swarm, txn_ctx.address, txn_ctx.sequence_number).await;

    // Verify that config has been loaded correctly with new key
    let validator_account = storage.get::<AccountAddress>(OWNER_ACCOUNT).unwrap().value;
    let config_network_key = op_tool
        .validator_config(validator_account, Some(&backend))
        .await
        .unwrap()
        .validator_network_address
        .find_noise_proto()
        .unwrap();
    assert_eq!(new_network_key, config_network_key);

    // Verify that the validator set info contains the new network key
    let info_network_key = op_tool
        .validator_set(Some(validator_account), Some(&backend))
        .await
        .unwrap()[0]
        .validator_network_address
        .find_noise_proto()
        .unwrap();
    assert_eq!(new_network_key, info_network_key);

    // Restart validator
    // At this point, the `add_node` call ensures connectivity to all nodes
    let validator = swarm.validators_mut().next().unwrap();
    validator.stop();
    validator.start().unwrap();
    swarm
        .wait_for_connectivity(Instant::now() + Duration::from_secs(60))
        .await
        .unwrap();
}

// TODO(https://github.com/aptos-labs/aptos-core/issues/317)
#[ignore]
#[tokio::test]
async fn test_network_key_rotation_recovery() {
    let num_nodes = 4;
    let (mut swarm, op_tool, backend, mut storage) =
        launch_swarm_with_op_tool_and_backend(num_nodes).await;

    // Rotate the network key in storage manually and perform a key rotation using the op_tool.
    // Here, we expected the op_tool to see that the network key in storage doesn't match the one
    // on-chain, and thus it should simply forward a transaction to the blockchain.
    let rotated_network_key = storage.rotate_key(VALIDATOR_NETWORK_KEY).unwrap();
    let (txn_ctx, new_network_key) = op_tool
        .rotate_validator_network_key(&backend, true)
        .await
        .unwrap();
    assert!(txn_ctx.execution_result.is_none());
    assert_eq!(new_network_key, to_x25519(rotated_network_key).unwrap());

    // Ensure all nodes have received the transaction
    wait_for_transaction_on_all_nodes(&swarm, txn_ctx.address, txn_ctx.sequence_number).await;

    // Verify that config has been loaded correctly with new key
    let validator_account = storage.get::<AccountAddress>(OWNER_ACCOUNT).unwrap().value;
    let config_network_key = op_tool
        .validator_config(validator_account, Some(&backend))
        .await
        .unwrap()
        .validator_network_address
        .find_noise_proto()
        .unwrap();
    assert_eq!(new_network_key, config_network_key);

    // Verify that the validator set info contains the new network key
    let info_network_key = op_tool
        .validator_set(Some(validator_account), Some(&backend))
        .await
        .unwrap()[0]
        .validator_network_address
        .find_noise_proto()
        .unwrap();
    assert_eq!(new_network_key, info_network_key);

    // Restart validator
    // At this point, the `add_node` call ensures connectivity to all nodes
    let validator = swarm.validators_mut().next().unwrap();
    validator.stop();
    validator.start().unwrap();
    swarm
        .wait_for_connectivity(Instant::now() + Duration::from_secs(60))
        .await
        .unwrap();
}

// TODO(https://github.com/aptos-labs/aptos-core/issues/317)
#[ignore]
#[tokio::test]
async fn test_operator_key_rotation() {
    let (swarm, op_tool, backend, storage) = launch_swarm_with_op_tool_and_backend(1).await;

    let (txn_ctx, _) = op_tool.rotate_operator_key(&backend, true).await.unwrap();
    assert!(txn_ctx.execution_result.is_none());

    // Wait for transaction execution
    let client = swarm.validators().next().unwrap().rest_client();
    wait_for_account_sequence_number(&client, txn_ctx.address, txn_ctx.sequence_number)
        .await
        .unwrap();

    // Verify that the transaction was executed correctly
    let txn_ctx = op_tool
        .validate_transaction(txn_ctx.address, txn_ctx.sequence_number)
        .await
        .unwrap();
    assert!(txn_ctx.execution_result.unwrap().success);

    // Rotate the consensus key to verify the operator key has been updated
    let (txn_ctx, new_consensus_key) = op_tool.rotate_consensus_key(&backend, false).await.unwrap();
    assert!(txn_ctx.execution_result.unwrap().success);

    // Verify that the config has been updated correctly with the new consensus key
    let validator_account = storage.get::<AccountAddress>(OWNER_ACCOUNT).unwrap().value;
    let config_consensus_key = op_tool
        .validator_config(validator_account, Some(&backend))
        .await
        .unwrap()
        .consensus_public_key;
    assert_eq!(new_consensus_key, config_consensus_key);
}

// TODO(https://github.com/aptos-labs/aptos-core/issues/317)
#[ignore]
#[tokio::test]
async fn test_operator_key_rotation_recovery() {
    let (swarm, op_tool, backend, mut storage) = launch_swarm_with_op_tool_and_backend(1).await;

    // Rotate the operator key
    let (txn_ctx, new_operator_key) = op_tool.rotate_operator_key(&backend, false).await.unwrap();
    assert!(txn_ctx.execution_result.unwrap().success);

    // Verify that the transaction was executed correctly
    let txn_ctx = op_tool
        .validate_transaction(txn_ctx.address, txn_ctx.sequence_number)
        .await
        .unwrap();
    assert!(txn_ctx.execution_result.unwrap().success);

    // Verify that the operator key was updated on-chain
    let operator_account = storage
        .get::<AccountAddress>(OPERATOR_ACCOUNT)
        .unwrap()
        .value;
    let account_resource = op_tool.account_resource(operator_account).await.unwrap();
    let on_chain_operator_key = hex::decode(account_resource.authentication_key).unwrap();
    assert_eq!(
        AuthenticationKey::ed25519(&new_operator_key),
        AuthenticationKey::try_from(on_chain_operator_key).unwrap()
    );

    // Rotate the operator key in storage manually and perform another rotation using the op tool.
    // Here, we expected the op_tool to see that the operator key in storage doesn't match the one
    // on-chain, and thus it should simply forward a transaction to the blockchain.
    let rotated_operator_key = storage.rotate_key(OPERATOR_KEY).unwrap();
    let (txn_ctx, new_operator_key) = op_tool.rotate_operator_key(&backend, true).await.unwrap();
    assert!(txn_ctx.execution_result.is_none());
    assert_eq!(rotated_operator_key, new_operator_key);

    // Wait for transaction execution
    let client = swarm.validators().next().unwrap().rest_client();
    wait_for_account_sequence_number(&client, txn_ctx.address, txn_ctx.sequence_number)
        .await
        .unwrap();

    // Verify that the transaction was executed correctly
    let txn_ctx = op_tool
        .validate_transaction(txn_ctx.address, txn_ctx.sequence_number)
        .await
        .unwrap();
    assert!(txn_ctx.execution_result.unwrap().success);

    // Verify that the operator key was updated on-chain
    let account_resource = op_tool.account_resource(operator_account).await.unwrap();
    let on_chain_operator_key = hex::decode(account_resource.authentication_key).unwrap();
    assert_eq!(
        AuthenticationKey::ed25519(&new_operator_key),
        AuthenticationKey::try_from(on_chain_operator_key).unwrap()
    );
}

async fn test_print_account(
    _swarm: &LocalSwarm,
    op_tool: &OperationalTool,
    backend: &SecureBackend,
    storage: &mut Storage,
) {
    // Print the owner account
    let op_tool_owner_account = op_tool.print_account(OWNER_ACCOUNT, backend).await.unwrap();
    let storage_owner_account = storage.get::<AccountAddress>(OWNER_ACCOUNT).unwrap().value;
    assert_eq!(storage_owner_account, op_tool_owner_account);

    // Print the operator account
    let op_tool_operator_account = op_tool
        .print_account(OPERATOR_ACCOUNT, backend)
        .await
        .unwrap();
    let storage_operator_account = storage
        .get::<AccountAddress>(OPERATOR_ACCOUNT)
        .unwrap()
        .value;
    assert_eq!(storage_operator_account, op_tool_operator_account);
}

async fn test_print_key(
    _swarm: &LocalSwarm,
    op_tool: &OperationalTool,
    backend: &SecureBackend,
    storage: &mut Storage,
) {
    // Print the operator key
    let op_tool_operator_key = op_tool.print_key(OPERATOR_KEY, backend).await.unwrap();
    let storage_operator_key = storage.get_public_key(OPERATOR_KEY).unwrap().public_key;
    assert_eq!(storage_operator_key, op_tool_operator_key);

    // Print the consensus key
    let op_tool_consensus_key = op_tool.print_key(CONSENSUS_KEY, backend).await.unwrap();
    let storage_consensus_key = storage.get_public_key(CONSENSUS_KEY).unwrap().public_key;
    assert_eq!(storage_consensus_key, op_tool_consensus_key);
}

async fn test_print_waypoints(
    _swarm: &LocalSwarm,
    op_tool: &OperationalTool,
    backend: &SecureBackend,
    _storage: &mut Storage,
) {
    // Insert a new waypoint and genesis waypoint into storage
    let inserted_waypoint =
        Waypoint::new_any(&LedgerInfo::new(BlockInfo::empty(), HashValue::zero()));
    op_tool
        .insert_waypoint(inserted_waypoint, backend, true)
        .await
        .unwrap();

    // Print the waypoint
    let waypoint = op_tool.print_waypoint(WAYPOINT, backend).await.unwrap();
    assert_eq!(inserted_waypoint, waypoint);

    // Print the gensis waypoint
    let genesis_waypoint = op_tool
        .print_waypoint(GENESIS_WAYPOINT, backend)
        .await
        .unwrap();
    assert_eq!(inserted_waypoint, genesis_waypoint);
}

// TODO(https://github.com/aptos-labs/aptos-core/issues/317)
#[ignore]
#[tokio::test]
async fn test_validator_config() {
    let (_swarm, op_tool, backend, mut storage) = launch_swarm_with_op_tool_and_backend(1).await;

    // Fetch the initial validator config for this operator's owner
    let owner_account = storage.get::<AccountAddress>(OWNER_ACCOUNT).unwrap().value;
    let consensus_key = storage.get_public_key(CONSENSUS_KEY).unwrap().public_key;
    let original_validator_config = op_tool
        .validator_config(owner_account, Some(&backend))
        .await
        .unwrap();
    assert_eq!(
        consensus_key,
        original_validator_config.consensus_public_key
    );

    // Rotate the consensus key locally and update the validator network address using the config
    let new_consensus_key = storage.rotate_key(CONSENSUS_KEY).unwrap();
    let new_network_address = NetworkAddress::from_str("/ip4/10.0.0.16/tcp/80").unwrap();
    let txn_ctx = op_tool
        .set_validator_config(
            Some(new_network_address.clone()),
            None,
            &backend,
            false,
            false,
        )
        .await
        .unwrap();
    assert!(txn_ctx.execution_result.unwrap().success);

    // Re-fetch the validator config and verify the changes
    let new_validator_config = op_tool
        .validator_config(owner_account, Some(&backend))
        .await
        .unwrap();
    assert_eq!(new_consensus_key, new_validator_config.consensus_public_key);
    assert!(new_validator_config
        .validator_network_address
        .to_string()
        .contains(&new_network_address.to_string()));
    assert_eq!(original_validator_config.name, new_validator_config.name);
    assert_eq!(
        original_validator_config.fullnode_network_address,
        new_validator_config.fullnode_network_address
    );
}

// TODO(https://github.com/aptos-labs/aptos-core/issues/317)
#[ignore]
#[tokio::test]
async fn test_validator_set() {
    let num_nodes = 4;
    let (_env, op_tool, backend, storage) = launch_swarm_with_op_tool_and_backend(num_nodes).await;

    // Fetch the validator config and validator info for this operator's owner
    let owner_account = storage.get::<AccountAddress>(OWNER_ACCOUNT).unwrap().value;
    let validator_config = op_tool
        .validator_config(owner_account, Some(&backend))
        .await
        .unwrap();
    let validator_set_infos = op_tool
        .validator_set(Some(owner_account), Some(&backend))
        .await
        .unwrap();
    assert_eq!(1, validator_set_infos.len());

    // Compare the validator config and the validator info
    let validator_info = validator_set_infos.first().unwrap();
    assert_eq!(validator_info.account_address, owner_account);
    assert_eq!(validator_info.name, validator_config.name);
    assert_eq!(
        validator_info.consensus_public_key,
        validator_config.consensus_public_key
    );
    assert_eq!(
        validator_info.validator_network_address,
        validator_config.validator_network_address
    );
    assert_eq!(
        validator_info.fullnode_network_address,
        validator_config.fullnode_network_address
    );

    // Fetch the entire validator set and check this account is included
    let validator_set_infos = op_tool.validator_set(None, Some(&backend)).await.unwrap();
    assert_eq!(num_nodes, validator_set_infos.len());
}

async fn test_verify_validator_state(
    _swarm: &LocalSwarm,
    op_tool: &OperationalTool,
    backend: &SecureBackend,
    storage: &mut Storage,
) {
    let result = op_tool.verify_validator_state(backend).await.unwrap();
    assert!(result.is_valid_state());

    // Rotate consensus key locally, but we do not update it on-chain
    // Verify the local validator state again.
    // The local consensus key is no longer mached with that registered on-chain
    let _ = storage.rotate_key(CONSENSUS_KEY).unwrap();
    let result = op_tool.verify_validator_state(backend).await.unwrap();
    assert_eq!(result.in_validator_set, Some(true));
    assert_eq!(result.consensus_key_match, Some(false));
    assert_eq!(result.consensus_key_unique, Some(true));
    assert_eq!(result.validator_network_key_match, Some(true));
    assert_eq!(result.fullnode_network_key_match, Some(true));

    // TODO(khiemngo): consider adding test where the validator is no longer in set
    // TODO(khiemngo): consider adding test where consensus key is not unique
}

/// Creates a new account address and key for testing.
fn create_new_test_account() -> (Ed25519PrivateKey, AccountAddress) {
    let mut rng = OsRng;
    let key = Ed25519PrivateKey::generate(&mut rng);
    let auth_key = AuthenticationKey::ed25519(&key.public_key());
    let account = auth_key.derived_address();
    (key, account)
}

/// Creates a new validator operator using the given file writer and verifies
/// the operator account is correctly initialized on-chain.
async fn create_operator_with_file_writer(
    file_writer: fn(&Ed25519PublicKey, PathBuf),
    swarm: &mut LocalSwarm,
    op_tool: &OperationalTool,
    _backend: &SecureBackend,
    _storage: &mut Storage,
) {
    // Create a new operator key and account
    let (operator_key, operator_account) = create_new_test_account();

    let validator = swarm.validators().next().unwrap();
    let client = validator.rest_client();
    // Verify the corresponding account doesn't exist on-chain
    client
        .get_account_resources(operator_account)
        .await
        .unwrap_err();

    // Write the key to a file using the provided file writer
    let key_file_path = write_key_to_file(&operator_key.public_key(), validator, file_writer);

    // Create the operator account
    let backend = create_root_storage(swarm);
    let op_human_name = "new_operator";
    let (txn_ctx, account_address) = op_tool
        .create_validator_operator(
            op_human_name,
            key_file_path.to_str().unwrap(),
            &backend,
            false,
        )
        .await
        .unwrap();
    assert_eq!(operator_account, account_address);
    assert!(txn_ctx.execution_result.unwrap().success);

    // Verify the operator account now exists on-chain
    let val_config_resource_response = client
        .get_resource::<ValidatorOperatorConfigResource>(
            operator_account,
            std::str::from_utf8(&ValidatorOperatorConfigResource::resource_path()).unwrap(),
        )
        .await
        .unwrap();
    let op_config_resource = val_config_resource_response.inner().clone();
    assert_eq!(op_human_name.as_bytes(), op_config_resource.human_name);
}

/// Creates a new validator using the given file writer and verifies
/// the account is correctly initialized on-chain.
async fn create_validator_with_file_writer(
    file_writer: fn(&Ed25519PublicKey, PathBuf),
    swarm: &mut LocalSwarm,
    op_tool: &OperationalTool,
    _backend: &SecureBackend,
    _storage: &mut Storage,
) {
    // Create a new validator key and account
    let (validator_key, validator_account) = create_new_test_account();

    let validator = swarm.validators().next().unwrap();
    let client = validator.rest_client();
    // Verify the corresponding account doesn't exist on-chain
    client
        .get_account_resources(validator_account)
        .await
        .unwrap_err();

    // Write the key to a file using the provided file writer
    let key_file_path = write_key_to_file(&validator_key.public_key(), validator, file_writer);

    // Create the validator account
    let backend = create_root_storage(swarm);
    let val_human_name = "new_validator";
    let (txn_ctx, account_address) = op_tool
        .create_validator(
            val_human_name,
            key_file_path.to_str().unwrap(),
            &backend,
            true,
        )
        .await
        .unwrap();
    assert!(txn_ctx.execution_result.is_none());
    assert_eq!(validator_account, account_address);

    // Wait for transaction execution
    wait_for_account_sequence_number(&client, txn_ctx.address, txn_ctx.sequence_number)
        .await
        .unwrap();

    // Verify that the transaction was executed
    let txn_ctx = op_tool
        .validate_transaction(txn_ctx.address, txn_ctx.sequence_number)
        .await
        .unwrap();
    assert!(txn_ctx.execution_result.unwrap().success);

    // Verify the validator account now exists on-chain
    let val_config_resource_response = client
        .get_resource::<aptos_types::validator_config::ValidatorConfig>(
            validator_account,
            std::str::from_utf8(&aptos_types::validator_config::ValidatorConfig::resource_path())
                .unwrap(),
        )
        .await
        .unwrap();
    let _val_config_resource = val_config_resource_response.inner().clone();
}

/// Launches a validator swarm of a specified size, connects an operational
/// tool to the first node and fetches that node's secure backend.
pub async fn launch_swarm_with_op_tool_and_backend(
    num_nodes: usize,
) -> (LocalSwarm, OperationalTool, SecureBackend, Storage) {
    let swarm = new_local_swarm_with_aptos(num_nodes).await;
    let chain_id = swarm.chain_id();
    let validator = swarm.validators().next().unwrap();

    // Connect the operator tool to the node's JSON RPC API
    let op_tool = OperationalTool::new(validator.rest_api_endpoint().to_string(), chain_id);

    // Load validator's on disk storage
    let backend = load_validators_backend_storage(validator);
    let storage: Storage = (&backend).try_into().unwrap();

    (swarm, op_tool, backend, storage)
}

/// Writes a given key to file using a specified file writer and test environment.
fn write_key_to_file(
    key: &Ed25519PublicKey,
    node: &LocalNode,
    file_writer: fn(&Ed25519PublicKey, PathBuf),
) -> PathBuf {
    let node_config_path = node.config_path();
    let file_path = node_config_path.with_file_name("KEY_FILE");
    file_writer(key, file_path.clone());
    file_path
}

async fn wait_for_account_sequence_number(
    client: &RestClient,
    address: AccountAddress,
    seq: u64,
) -> Result<()> {
    const DEFAULT_WAIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);

    let start = std::time::Instant::now();
    while start.elapsed() < DEFAULT_WAIT_TIMEOUT {
        let txns = client
            .get_account_transactions(address, Some(seq), Some(1))
            .await?
            .into_inner();
        if txns.len() == 1 {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    bail!(
        "wait for account(address={}) transaction(seq={}) timeout",
        address,
        seq
    )
}

pub async fn wait_for_transaction_on_all_nodes(
    swarm: &LocalSwarm,
    account: AccountAddress,
    sequence_number: u64,
) {
    for validator in swarm.validators() {
        let client = validator.rest_client();
        wait_for_account_sequence_number(&client, account, sequence_number)
            .await
            .unwrap();
    }
}

async fn generate_x25519_key(path: &Path) -> x25519::PrivateKey {
    let (priv_key, _pub_key) =
        GenerateKey::generate_x25519(aptos::common::types::EncodingType::Hex, path)
            .await
            .unwrap();
    priv_key
}
