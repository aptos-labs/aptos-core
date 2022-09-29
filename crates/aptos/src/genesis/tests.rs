// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::OptionalPoolAddressArgs;
use crate::common::utils::read_from_file;
use crate::genesis::git::{from_yaml, BALANCES_FILE, EMPLOYEE_VESTING_ACCOUNTS_FILE};
use crate::genesis::git::{FRAMEWORK_NAME, OWNER_FILE};
use crate::genesis::keys::{GenerateLayoutTemplate, PUBLIC_KEYS_FILE};
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
use aptos_genesis::config::{
    AccountBalanceMap, EmployeePoolConfig, EmployeePoolMap, HostAndPort, Layout,
    OwnerConfiguration, ValidatorConfiguration,
};
use aptos_genesis::keys::PublicIdentity;
use aptos_keygen::KeyGen;
use aptos_temppath::TempPath;
use aptos_types::account_address::AccountAddress;
use aptos_types::chain_id::ChainId;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr,
};
use vm_genesis::{AccountBalance, TestValidator};

const INITIAL_BALANCE: u64 = 100_000_000_000_000;

/// Test the E2E genesis flow since it doesn't require a node to run
#[tokio::test]
async fn test_genesis_e2e_flow() {
    let dir = TempPath::new();
    dir.create_as_dir().unwrap();
    let git_options = create_users(2, &dir, &mut vec![]).await;

    // Now generate genesis
    let output_dir = TempPath::new();
    output_dir.create_as_dir().unwrap();
    let output_dir = PathBuf::from(output_dir.path());
    generate_genesis(git_options, output_dir.clone(), false).await;

    // TODO: Verify that these are good
    let waypoint_file = output_dir.join("waypoint.txt");
    assert!(waypoint_file.exists());
    let genesis_file = output_dir.join("genesis.blob");
    assert!(genesis_file.exists());
}

#[tokio::test]
async fn test_mainnet_genesis_e2e_flow() {
    let dir = TempPath::new();
    dir.create_as_dir().unwrap();
    let git_options = create_users(2, &dir, &mut vec![10, 0]).await;

    let account_1 = AccountAddress::from_hex_literal("0x101").unwrap();
    let account_2 = AccountAddress::from_hex_literal("0x102").unwrap();
    let employee_1 = AccountAddress::from_hex_literal("0x201").unwrap();
    let employee_2 = AccountAddress::from_hex_literal("0x202").unwrap();
    let employee_3 = AccountAddress::from_hex_literal("0x203").unwrap();
    let employee_4 = AccountAddress::from_hex_literal("0x204").unwrap();
    let admin_dir = generate_keys(dir.path(), 2).await;
    let admin_identity: PublicIdentity = from_yaml(
        &String::from_utf8(read_from_file(admin_dir.join(PUBLIC_KEYS_FILE).as_path()).unwrap())
            .unwrap(),
    )
    .unwrap();

    // Create initial balances and employee vesting account files.
    let git_dir = git_options.local_repository_dir.as_ref().unwrap().as_path();

    // Load the standalone staking pools
    let staker_0 = get_owner_address(git_options.clone(), 0);
    let staker_1 = get_owner_address(git_options.clone(), 1);

    create_account_balances_file(
        PathBuf::from(git_dir),
        vec![
            staker_0,
            staker_1,
            account_1,
            account_2,
            employee_1,
            employee_2,
            employee_3,
            employee_4,
            admin_identity.account_address,
        ],
    )
    .await;
    create_employee_vesting_accounts_file(
        PathBuf::from(git_dir),
        admin_identity,
        &vec![vec![employee_1, employee_2], vec![employee_3, employee_4]],
        &[true, false],
    )
    .await;

    // Now generate genesis
    let output_dir = TempPath::new();
    output_dir.create_as_dir().unwrap();
    let output_dir = PathBuf::from(output_dir.path());
    generate_genesis(git_options, output_dir.clone(), true).await;

    // TODO: Verify that these are good
    let waypoint_file = output_dir.join("waypoint.txt");
    assert!(waypoint_file.exists());
    let genesis_file = output_dir.join("genesis.blob");
    assert!(genesis_file.exists());
}

fn get_owner_address(git_options: GitOptions, index: u64) -> AccountAddress {
    let git_client = git_options.get_client().unwrap();
    let owner_config: OwnerConfiguration = git_client
        .get(
            Path::new(&format!("user-{}", index))
                .join(OWNER_FILE)
                .as_path(),
        )
        .unwrap();
    owner_config.owner_account_address
}

async fn create_users(
    num_users: u8,
    dir: &TempPath,
    commission_rates: &mut Vec<u64>,
) -> GitOptions {
    let mut users: HashMap<String, PathBuf> = HashMap::new();
    for i in 0..num_users {
        let name = format!("user-{}", i);
        let output_dir = generate_keys(dir.path(), i).await;
        users.insert(name, output_dir);
    }

    let names = users.keys().map(|key| key.to_string()).collect();
    let mut key_gen = KeyGen::from_seed([num_users.saturating_add(1); 32]);
    // First step is setup the local git repo
    let root_private_key = key_gen.generate_ed25519_private_key();
    let git_options = setup_git_dir(&root_private_key, names, ChainId::test()).await;

    for (name, user_dir) in users.iter() {
        let commission_rate = if commission_rates.is_empty() {
            0
        } else {
            commission_rates.remove(0)
        };
        set_validator_config(
            name.to_string(),
            git_options.clone(),
            user_dir.as_path(),
            commission_rate,
        )
        .await;
    }
    git_options
}

/// Generate genesis and waypoint
async fn generate_genesis(git_options: GitOptions, output_dir: PathBuf, mainnet: bool) {
    let command = GenerateGenesis {
        prompt_options: PromptOptions::yes(),
        git_options,
        output_dir: Some(output_dir),
        mainnet,
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
    layout.total_supply = Some(INITIAL_BALANCE * 9);

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
async fn set_validator_config(
    username: String,
    git_options: GitOptions,
    keys_dir: &Path,
    commission_percentage: u64,
) {
    let command = SetValidatorConfiguration {
        username,
        git_options,
        owner_public_identity_file: Some(PathBuf::from(keys_dir).join(PUBLIC_KEYS_FILE)),
        validator_host: HostAndPort::from_str("localhost:6180").unwrap(),
        stake_amount: 100_000_000_000_000,
        full_node_host: None,
        operator_public_identity_file: None,
        voter_public_identity_file: None,
        commission_percentage,
        join_during_genesis: true,
    };

    command.execute().await.unwrap()
}

async fn create_account_balances_file(path: PathBuf, addresses: Vec<AccountAddress>) {
    let account_balances: Vec<AccountBalance> = addresses
        .iter()
        .map(|account_address| AccountBalance {
            account_address: *account_address,
            balance: INITIAL_BALANCE,
        })
        .collect();

    let balance_map = AccountBalanceMap::try_from(account_balances).unwrap();

    write_to_file(
        &path.join(BALANCES_FILE),
        BALANCES_FILE,
        serde_yaml::to_string(&balance_map).unwrap().as_bytes(),
    )
    .unwrap();
}

async fn create_employee_vesting_accounts_file(
    path: PathBuf,
    admin_identity: PublicIdentity,
    employee_groups: &Vec<Vec<AccountAddress>>,
    join_during_genesis: &[bool],
) {
    TestValidator::new_test_set(Some(employee_groups.len()), Some(INITIAL_BALANCE));
    let employee_vesting_accounts: Vec<_> = employee_groups
        .iter()
        .enumerate()
        .map(|(index, accounts)| {
            let validator_config = if *join_during_genesis.get(index).unwrap() {
                let admin_identity = admin_identity.clone();

                ValidatorConfiguration {
                    owner_account_address: admin_identity.account_address,
                    owner_account_public_key: admin_identity.account_public_key.clone(),
                    operator_account_address: admin_identity.account_address,
                    operator_account_public_key: admin_identity.account_public_key.clone(),
                    voter_account_address: admin_identity.account_address,
                    voter_account_public_key: admin_identity.account_public_key,
                    consensus_public_key: admin_identity.consensus_public_key,
                    proof_of_possession: admin_identity.consensus_proof_of_possession,
                    validator_network_public_key: admin_identity.validator_network_public_key,
                    validator_host: Some(HostAndPort::from_str("localhost:8080").unwrap()),
                    full_node_network_public_key: admin_identity.full_node_network_public_key,
                    full_node_host: Some(HostAndPort::from_str("localhost:8081").unwrap()),
                    stake_amount: 2 * INITIAL_BALANCE,
                    commission_percentage: 0,
                    join_during_genesis: true,
                }
            } else {
                ValidatorConfiguration {
                    owner_account_address: admin_identity.account_address,
                    owner_account_public_key: admin_identity.account_public_key.clone(),
                    operator_account_address: admin_identity.account_address,
                    operator_account_public_key: admin_identity.account_public_key.clone(),
                    voter_account_address: admin_identity.account_address,
                    voter_account_public_key: admin_identity.account_public_key.clone(),
                    consensus_public_key: None,
                    proof_of_possession: None,
                    validator_network_public_key: None,
                    validator_host: None,
                    full_node_network_public_key: None,
                    full_node_host: None,
                    stake_amount: 2 * INITIAL_BALANCE,
                    commission_percentage: 0,
                    join_during_genesis: false,
                }
            };

            EmployeePoolConfig {
                accounts: accounts.clone(),
                validator: validator_config,
                vesting_schedule_numerators: vec![3, 3, 3, 3, 1],
                vesting_schedule_denominator: 48,
                beneficiary_resetter: admin_identity.account_address,
            }
        })
        .collect();
    let employee_vesting_map = EmployeePoolMap {
        inner: employee_vesting_accounts,
    };
    write_to_file(
        &path.join(EMPLOYEE_VESTING_ACCOUNTS_FILE),
        EMPLOYEE_VESTING_ACCOUNTS_FILE,
        serde_yaml::to_string(&employee_vesting_map)
            .unwrap()
            .as_bytes(),
    )
    .unwrap();
}
