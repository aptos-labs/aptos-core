// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{
        types::{OptionalPoolAddressArgs, PromptOptions, RngArgs},
        utils::{read_from_file, write_to_file},
    },
    genesis::{
        git::{from_yaml, GitOptions, SetupGit, FRAMEWORK_NAME},
        keys::{GenerateKeys, GenerateLayoutTemplate, SetValidatorConfiguration, PUBLIC_KEYS_FILE},
        GenerateGenesis,
    },
    CliCommand,
};
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    PrivateKey,
};
use aptos_genesis::config::{HostAndPort, Layout};
use aptos_keygen::KeyGen;
use aptos_temppath::TempPath;
use aptos_types::chain_id::ChainId;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr,
};

/// Test the E2E genesis flow since it doesn't require a node to run
#[tokio::test]
async fn test_genesis_e2e_flow() {
    const NUM_USERS: u8 = 2;
    let chain_id = ChainId::test();
    let mut users: HashMap<String, PathBuf> = HashMap::new();
    let dir = TempPath::new();
    dir.create_as_dir().unwrap();

    // Create users
    for i in 0..NUM_USERS {
        let name = format!("user-{}", i);
        let dir = generate_keys(dir.path(), i).await;
        users.insert(name, dir);
    }

    let names: Vec<_> = users.keys().map(|key| key.to_string()).collect();

    let mut keygen = KeyGen::from_seed([NUM_USERS.saturating_add(1); 32]);

    // First step is setup the local git repo
    let root_private_key = keygen.generate_ed25519_private_key();
    let git_options = setup_git_dir(&root_private_key, names, chain_id).await;

    // Add keys
    for (name, user_dir) in users.iter() {
        add_public_keys(name.to_string(), git_options.clone(), user_dir.as_path()).await;
    }

    // Now generate genesis
    let output_dir = TempPath::new();
    output_dir.create_as_dir().unwrap();
    let output_dir = PathBuf::from(output_dir.path());
    generate_genesis(git_options, output_dir.clone()).await;

    // TODO: Verify that these are good
    let waypoint_file = output_dir.join("waypoint.txt");
    assert!(waypoint_file.exists());
    let genesis_file = output_dir.join("genesis.blob");
    assert!(genesis_file.exists());
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
) -> GitOptions {
    let git_options = git_options();
    let layout_file = TempPath::new();
    layout_file.create_as_file().unwrap();
    let layout_file = layout_file.path();

    create_layout_file(layout_file, root_private_key.public_key(), users, chain_id).await;
    let setup_command = SetupGit {
        git_options: git_options.clone(),
        layout_file: PathBuf::from(layout_file),
    };

    setup_command
        .execute()
        .await
        .expect("Should not fail creating repo folder");

    // Add framework
    add_framework_to_dir(git_options.local_repository_dir.as_ref().unwrap().as_path());
    git_options
}

/// Add framework to git directory
fn add_framework_to_dir(git_dir: &Path) {
    cached_packages::head_release_bundle()
        .write(git_dir.join(FRAMEWORK_NAME))
        .unwrap()
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
async fn add_public_keys(username: String, git_options: GitOptions, keys_dir: &Path) {
    let command = SetValidatorConfiguration {
        username,
        git_options,
        owner_public_identity_file: Some(PathBuf::from(keys_dir).join(PUBLIC_KEYS_FILE)),
        validator_host: HostAndPort::from_str("localhost:6180").unwrap(),
        stake_amount: 100_000_000_000_000,
        full_node_host: None,
        operator_public_identity_file: None,
        voter_public_identity_file: None,
    };

    command.execute().await.unwrap()
}
