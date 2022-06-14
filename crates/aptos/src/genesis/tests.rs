// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

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
use move_deps::move_binary_format::access::ModuleAccess;
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
        output_dir,
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
    let layout_file = create_layout_file(root_private_key.public_key(), users, chain_id);
    let layout_file = PathBuf::from(layout_file.path());
    let setup_command = SetupGit {
        git_options: git_options.clone(),
        layout_file,
    };

    setup_command
        .execute()
        .await
        .expect("Should not fail creating repo folder");

    // Add framework
    add_framework_to_dir(git_options.local_repository_dir.as_ref().unwrap().as_path());
    git_options
}

/// Add framework modules to git directory
fn add_framework_to_dir(git_dir: &Path) {
    let framework_dir = git_dir.join("framework");
    cached_framework_packages::modules_with_blobs().for_each(|(blob, module)| {
        let module_name = module.name();
        let file = framework_dir.join(format!("{}.mv", module_name));
        write_to_file(file.as_path(), module_name.as_str(), blob).unwrap();
    });
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
fn create_layout_file(
    root_public_key: Ed25519PublicKey,
    users: Vec<String>,
    chain_id: ChainId,
) -> TempPath {
    let layout = Layout {
        root_key: root_public_key,
        users,
        chain_id,
        allow_new_validators: false,
        initial_lockup_period_duration_secs: 0,
        initial_balances: Default::default(),
    };
    let file = TempPath::new();
    file.create_as_file().unwrap();

    write_to_file(
        file.path(),
        "Layout file",
        serde_yaml::to_string(&layout).unwrap().as_bytes(),
    )
    .unwrap();
    file
}

/// Generate keys for a "user"
async fn generate_keys(dir: &Path, index: u8) -> PathBuf {
    let output_dir = dir.join(index.to_string());
    let command = GenerateKeys {
        rng_args: RngArgs::from_seed([index; 32]),
        prompt_options: PromptOptions::yes(),
        output_dir: output_dir.clone(),
    };
    let _ = command.execute().await.unwrap();

    output_dir
}

/// Set validator configuration for a user
async fn add_public_keys(username: String, git_options: GitOptions, keys_dir: &Path) {
    let command = SetValidatorConfiguration {
        username,
        git_options,
        keys_dir: PathBuf::from(keys_dir),
        validator_host: HostAndPort::from_str("localhost:6180").unwrap(),
        full_node_host: None,
        stake_amount: 1,
    };

    command.execute().await.unwrap()
}
