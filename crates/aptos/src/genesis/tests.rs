// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::OptionalPoolAddressArgs;
use crate::common::utils::read_from_file;
use crate::genesis::git::FRAMEWORK_NAME;
use crate::genesis::git::{from_yaml, to_yaml};
use crate::genesis::keys::{GenerateLayoutTemplate, PUBLIC_KEYS_FILE};
use crate::genesis::BALANCES_FILE;
use crate::{
    common::{
        types::{PromptOptions, RngArgs},
        utils::write_to_file,
    },
    genesis::{
        git::{GitOptions, SetupGit},
        keys::{GenerateKeys, SetValidatorConfiguration},
        GenerateGenesis,
    },
    CliCommand, CliTypedResult,
};
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    PrivateKey,
};
use aptos_genesis::config::{HostAndPort, Layout};
use aptos_genesis::keys::PublicIdentity;
use aptos_keygen::KeyGen;
use aptos_temppath::TempPath;
use aptos_types::account_address::AccountAddress;
use aptos_types::chain_id::ChainId;
use std::collections::BTreeMap;
use std::fs::read_to_string;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr,
};
use vm_genesis::InitialBalance;

async fn setup_genesis_test(
    num_users: u8,
) -> (
    ChainId,
    HashMap<String, PathBuf>,
    Vec<String>,
    Ed25519PrivateKey,
    TempPath,
) {
    let chain_id = ChainId::test();
    let mut users: HashMap<String, PathBuf> = HashMap::new();
    let dir = TempPath::new();
    dir.create_as_dir().unwrap();

    // Create users
    for i in 0..num_users {
        let name = format!("user-{}", i);
        let dir = generate_keys(dir.path(), i).await;
        users.insert(name, dir);
    }

    // Retrieve names of users
    let names: Vec<_> = users.keys().map(|key| key.to_string()).collect();

    // Generate the root key
    let mut keygen = KeyGen::from_seed([5u8; 32]);
    let root_private_key = keygen.generate_ed25519_private_key();

    (chain_id, users, names, root_private_key, dir)
}

async fn generate_genesis_in_temp_folder(git_options: GitOptions) -> (PathBuf, PathBuf) {
    let output_dir = TempPath::new();
    output_dir.create_as_dir().unwrap();
    let output_dir = PathBuf::from(output_dir.path());
    generate_genesis(git_options, output_dir.clone()).await;

    // TODO: Verify that these are good
    let waypoint_file = output_dir.join("waypoint.txt");
    assert!(waypoint_file.exists());
    let genesis_file = output_dir.join("genesis.blob");
    assert!(genesis_file.exists());

    // Return both for more assertions
    (genesis_file, waypoint_file)
}

/// Test the E2E genesis flow since it doesn't require a node to run
#[tokio::test]
async fn test_genesis_e2e_flow() {
    const NUM_USERS: u8 = 5;
    let (chain_id, users, names, root_private_key, _repo_dir) = setup_genesis_test(NUM_USERS).await;

    // First step is setup the local git repo
    let git_options = setup_git_dir(&root_private_key, names, chain_id, &[]).await;

    // Add keys
    for (name, user_dir) in users.iter() {
        add_public_keys(
            name.to_string(),
            git_options.clone(),
            user_dir.as_path(),
            None,
            None,
        )
        .await;
    }

    // Now generate genesis
    generate_genesis_in_temp_folder(git_options).await;
}

#[tokio::test]
async fn test_genesis_e2e_flow_with_balances() {
    const NUM_USERS: u8 = 4;
    let (chain_id, users, names, root_private_key, _repo_dir) = setup_genesis_test(NUM_USERS).await;

    // Build initial balances
    let mut initial_balances: Vec<_> = users
        .iter()
        .map(|(_, path)| {
            let public_identity = load_public_identity(path);
            InitialBalance {
                address: public_identity.account_address,
                balance: 100_000_000_000_000,
            }
        })
        .collect();

    // Add one more arbitrary balance, not in the validator set
    initial_balances.push(InitialBalance {
        address: AccountAddress::from_str("0x1337").unwrap(),
        balance: 100,
    });

    // First step is setup the local git repo
    let git_options = setup_git_dir(
        &root_private_key,
        names,
        chain_id,
        initial_balances.as_slice(),
    )
    .await;

    // Add keys
    for (name, user_dir) in users.iter() {
        add_public_keys(
            name.to_string(),
            git_options.clone(),
            user_dir.as_path(),
            None,
            None,
        )
        .await;
    }

    // Now generate genesis
    generate_genesis_in_temp_folder(git_options).await;
}

#[tokio::test]
async fn test_genesis_e2e_flow_with_operators() {
    const NUM_USERS: u8 = 3;
    let (chain_id, users, names, root_private_key, dir) = setup_genesis_test(NUM_USERS).await;

    // First step is setup the local git repo
    let git_options = setup_git_dir(&root_private_key, names, chain_id, &[]).await;

    let operator_dir = generate_keys(dir.path(), 5).await;
    let voter_dir = generate_keys(dir.path(), 6).await;

    // Add keys
    for (name, user_dir) in users.iter() {
        add_public_keys(
            name.to_string(),
            git_options.clone(),
            user_dir.as_path(),
            Some(operator_dir.join(PUBLIC_KEYS_FILE)),
            Some(voter_dir.join(PUBLIC_KEYS_FILE)),
        )
        .await;
    }

    // Now generate genesis
    generate_genesis_in_temp_folder(git_options).await;
}

#[tokio::test]
async fn test_genesis_e2e_flow_with_operators_and_balances() {
    const NUM_USERS: u8 = 3;
    let (chain_id, users, names, root_private_key, dir) = setup_genesis_test(NUM_USERS).await;

    let operator_dir = generate_keys(dir.path(), 5).await;
    let operator_address = load_public_identity(operator_dir.as_path()).account_address;
    let voter_dir = generate_keys(dir.path(), 6).await;
    let voter_address = load_public_identity(voter_dir.as_path()).account_address;
    // Build initial balances
    let mut initial_balances: Vec<_> = users
        .iter()
        .map(|(_, path)| {
            let public_identity = load_public_identity(path);
            InitialBalance {
                address: public_identity.account_address,
                balance: 100_000_000_000_000,
            }
        })
        .collect();

    // Add the operator and voter to balances
    initial_balances.push(InitialBalance {
        address: operator_address,
        balance: 12345,
    });
    initial_balances.push(InitialBalance {
        address: voter_address,
        balance: 1337,
    });

    // First step is setup the local git repo
    let git_options = setup_git_dir(
        &root_private_key,
        names,
        chain_id,
        initial_balances.as_slice(),
    )
    .await;

    // Add keys
    for (name, user_dir) in users.iter() {
        add_public_keys(
            name.to_string(),
            git_options.clone(),
            user_dir.as_path(),
            Some(operator_dir.join(PUBLIC_KEYS_FILE)),
            Some(voter_dir.join(PUBLIC_KEYS_FILE)),
        )
        .await;
    }

    // Now generate genesis
    generate_genesis_in_temp_folder(git_options).await;
}

/// Generate genesis and waypoint
async fn generate_genesis(git_options: GitOptions, output_dir: PathBuf) {
    let command = GenerateGenesis {
        prompt_options: PromptOptions::yes(),
        git_options,
        output_dir: Some(output_dir),
    };
    let _ = command.execute().await.unwrap();
}

/// Setup a temporary repo location and add all required pieces
async fn setup_git_dir(
    root_private_key: &Ed25519PrivateKey,
    users: Vec<String>,
    chain_id: ChainId,
    initial_balances: &[InitialBalance],
) -> GitOptions {
    let git_options = git_options();
    let layout_file = TempPath::new();
    layout_file.create_as_file().unwrap();
    let layout_file = layout_file.path();

    create_layout_file(
        layout_file,
        root_private_key.public_key(),
        users,
        chain_id,
        !initial_balances.is_empty(),
    )
    .await;
    let setup_command = SetupGit {
        git_options: git_options.clone(),
        layout_file: PathBuf::from(layout_file),
    };

    setup_command
        .execute()
        .await
        .expect("Should not fail creating repo folder");

    let git_dir = git_options.local_repository_dir.as_ref().unwrap();
    // Add framework
    add_framework_to_dir(git_dir.as_path());
    // Add balances (if applicable)
    add_balances(git_dir.as_path(), initial_balances).unwrap();
    git_options
}

/// Add framework to git directory
fn add_framework_to_dir(git_dir: &Path) {
    cached_packages::head_release_bundle()
        .write(git_dir.join(FRAMEWORK_NAME))
        .unwrap()
}

fn add_balances(git_dir: &Path, initial_balances: &[InitialBalance]) -> CliTypedResult<()> {
    if !initial_balances.is_empty() {
        // Translate balances to map format
        let balances: Vec<_> = initial_balances
            .iter()
            .map(|initial_balance| {
                let mut map = BTreeMap::new();
                map.insert(initial_balance.address, initial_balance.balance);
                map
            })
            .collect();

        write_to_file(
            git_dir.join(BALANCES_FILE).as_path(),
            BALANCES_FILE,
            to_yaml(balances.as_slice())?.as_bytes(),
        )
    } else {
        Ok(())
    }
}

/// Local git options for testing
fn git_options() -> GitOptions {
    let temp_path = TempPath::new();
    let path = PathBuf::from(temp_path.path());
    GitOptions {
        local_repository_dir: Some(path),
        ..Default::default()
    }
}

/// Create a layout file for the repo
async fn create_layout_file(
    file: &Path,
    root_public_key: Ed25519PublicKey,
    users: Vec<String>,
    chain_id: ChainId,
    has_initial_balances: bool,
) {
    GenerateLayoutTemplate {
        output_file: PathBuf::from(file),
        prompt_options: PromptOptions::yes(),
    }
    .execute()
    .await
    .expect("Expected to create layout template");

    // Update layout file
    let mut layout: Layout =
        from_yaml(&String::from_utf8(read_from_file(file).unwrap()).unwrap()).unwrap();
    layout.root_key = Some(root_public_key);
    layout.users = users;
    layout.chain_id = chain_id;
    layout.is_test = true;
    layout.has_initial_balances = has_initial_balances;

    write_to_file(
        file,
        "Layout file",
        serde_yaml::to_string(&layout).unwrap().as_bytes(),
    )
    .unwrap();
}

/// Generate keys for a "user"
async fn generate_keys(dir: &Path, index: u8) -> PathBuf {
    let output_dir = dir.join(index.to_string());
    let command = GenerateKeys {
        pool_address_args: OptionalPoolAddressArgs { pool_address: None },
        rng_args: RngArgs::from_seed([index; 32]),
        prompt_options: PromptOptions::yes(),
        output_dir: Some(output_dir.clone()),
    };
    let _ = command.execute().await.unwrap();

    output_dir
}

/// Set validator configuration for a user
async fn add_public_keys(
    username: String,
    git_options: GitOptions,
    keys_dir: &Path,
    operator_identity: Option<PathBuf>,
    voter_identity: Option<PathBuf>,
) {
    let command = SetValidatorConfiguration {
        username,
        git_options,
        owner_public_identity_file: Some(PathBuf::from(keys_dir).join(PUBLIC_KEYS_FILE)),
        validator_host: HostAndPort::from_str("localhost:6180").unwrap(),
        stake_amount: 100_000_000_000_000,
        full_node_host: None,
        operator_public_identity_file: operator_identity,
        voter_public_identity_file: voter_identity,
    };

    command.execute().await.unwrap()
}

fn load_public_identity(user_dir: &Path) -> PublicIdentity {
    from_yaml(&read_to_string(user_dir.join(PUBLIC_KEYS_FILE).as_path()).unwrap()).unwrap()
}
