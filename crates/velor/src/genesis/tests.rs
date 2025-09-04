// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{
        types::{OptionalPoolAddressArgs, PromptOptions, RngArgs},
        utils::{read_from_file, write_to_file},
    },
    genesis::{
        git::{
            from_yaml, GitOptions, SetupGit, BALANCES_FILE, EMPLOYEE_VESTING_ACCOUNTS_FILE,
            FRAMEWORK_NAME,
        },
        keys::{GenerateKeys, GenerateLayoutTemplate, SetValidatorConfiguration, PUBLIC_KEYS_FILE},
        GenerateGenesis,
    },
    CliCommand,
};
use velor_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    PrivateKey,
};
use velor_genesis::{
    config::{
        AccountBalanceMap, EmployeePoolConfig, EmployeePoolMap, HostAndPort, Layout,
        ValidatorConfiguration,
    },
    keys::PublicIdentity,
};
use velor_keygen::KeyGen;
use velor_temppath::TempPath;
use velor_types::{account_address::AccountAddress, chain_id::ChainId};
use velor_vm_genesis::{AccountBalance, TestValidator};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr,
};

const INITIAL_BALANCE: u64 = 100_000_000_000_000;

/// Test the E2E genesis flow since it doesn't require a node to run
#[tokio::test]
async fn test_genesis_e2e_flow() {
    let is_mainnet = false;
    let dir = TempPath::new();
    dir.create_as_dir().unwrap();
    let git_options = create_users(2, 0, &dir, &mut vec![], is_mainnet).await;

    // Now generate genesis
    let output_dir = TempPath::new();
    output_dir.create_as_dir().unwrap();
    let output_dir = PathBuf::from(output_dir.path());
    generate_genesis(git_options, output_dir.clone(), is_mainnet).await;

    // TODO: Verify that these are good
    let waypoint_file = output_dir.join("waypoint.txt");
    assert!(waypoint_file.exists());
    let genesis_file = output_dir.join("genesis.blob");
    assert!(genesis_file.exists());
}

#[tokio::test]
async fn test_mainnet_genesis_e2e_flow() {
    let is_mainnet = true;
    let dir = TempPath::new();
    dir.create_as_dir().unwrap();
    let git_options = create_users(2, 4, &dir, &mut vec![10, 1], is_mainnet).await;
    let account_1 = AccountAddress::from_hex_literal("0x101").unwrap();
    let account_2 = AccountAddress::from_hex_literal("0x102").unwrap();
    let employee_1 = AccountAddress::from_hex_literal("0x201").unwrap();
    let employee_2 = AccountAddress::from_hex_literal("0x202").unwrap();
    let employee_3 = AccountAddress::from_hex_literal("0x203").unwrap();
    let employee_4 = AccountAddress::from_hex_literal("0x204").unwrap();

    let owner_identity1 = load_identity(dir.path(), "owner-0");
    let owner_identity2 = load_identity(dir.path(), "owner-1");
    let operator_identity1 = load_identity(dir.path(), "operator-0");
    let operator_identity2 = load_identity(dir.path(), "operator-1");
    let voter_identity1 = load_identity(dir.path(), "voter-0");
    let voter_identity2 = load_identity(dir.path(), "voter-1");
    let admin_identity1 = load_identity(dir.path(), "other-0");
    let admin_identity2 = load_identity(dir.path(), "other-1");
    let employee_operator_identity1 = load_identity(dir.path(), "other-2");
    let employee_operator_identity2 = load_identity(dir.path(), "other-3");

    // Create initial balances and employee vesting account files.
    let git_dir = git_options.local_repository_dir.as_ref().unwrap().as_path();

    create_account_balances_file(PathBuf::from(git_dir), vec![
        owner_identity1.account_address,
        owner_identity2.account_address,
        operator_identity1.account_address,
        operator_identity2.account_address,
        voter_identity1.account_address,
        voter_identity2.account_address,
        account_1,
        account_2,
        employee_1,
        employee_2,
        employee_3,
        employee_4,
        admin_identity1.account_address,
        admin_identity2.account_address,
        employee_operator_identity1.account_address,
        employee_operator_identity2.account_address,
    ])
    .await;
    create_employee_vesting_accounts_file(
        PathBuf::from(git_dir),
        &[admin_identity1, admin_identity2],
        &[employee_operator_identity1, employee_operator_identity2],
        &[vec![employee_1, employee_2], vec![employee_3, employee_4]],
        &[true, false],
    )
    .await;

    // Now generate genesis
    let output_dir = TempPath::new();
    output_dir.create_as_dir().unwrap();
    let output_dir = PathBuf::from(output_dir.path());
    generate_genesis(git_options, output_dir.clone(), is_mainnet).await;

    // TODO: Verify that these are good
    let waypoint_file = output_dir.join("waypoint.txt");
    assert!(waypoint_file.exists());
    let genesis_file = output_dir.join("genesis.blob");
    assert!(genesis_file.exists());
}

pub fn load_identity(base_dir: &Path, name: &str) -> PublicIdentity {
    let path = base_dir.join(name).join(PUBLIC_KEYS_FILE);
    from_yaml(&String::from_utf8(read_from_file(path.as_path()).unwrap()).unwrap()).unwrap()
}

async fn create_users(
    num_validators: u8,
    num_other_users: u8,
    dir: &TempPath,
    commission_rates: &mut Vec<u64>,
    is_mainnet: bool,
) -> GitOptions {
    let mut users: HashMap<String, PathBuf> = HashMap::new();
    for i in 0..num_validators {
        let name = format!("owner-{}", i);
        let output_dir = generate_keys(dir.path(), &name).await;
        users.insert(name, output_dir);

        let name = format!("operator-{}", i);
        let output_dir = generate_keys(dir.path(), &name).await;
        users.insert(name, output_dir);

        let name = format!("voter-{}", i);
        let output_dir = generate_keys(dir.path(), &name).await;
        users.insert(name, output_dir);
    }
    for i in 0..num_other_users {
        let name = format!("other-{}", i);
        let output_dir = generate_keys(dir.path(), &name).await;
        users.insert(name, output_dir);
    }

    // Get the validator's names
    let validator_names = users
        .keys()
        .map(|key| key.to_string())
        .filter(|name| name.starts_with("owner"))
        .collect();
    let mut key_gen = KeyGen::from_seed([num_validators.saturating_add(1); 32]);

    // First step is setup the local git repo
    let root_private_key = if !is_mainnet {
        Some(key_gen.generate_ed25519_private_key())
    } else {
        None
    };
    let git_options =
        setup_git_dir(root_private_key.as_ref(), validator_names, ChainId::test()).await;

    // Only write validators to folders
    for i in 0..num_validators {
        let owner_name = format!("owner-{}", i);
        let owner_identity = users.get(&owner_name).unwrap().join(PUBLIC_KEYS_FILE);
        let operator_identity = users
            .get(&format!("operator-{}", i))
            .unwrap()
            .join(PUBLIC_KEYS_FILE);
        let voter_identity = users
            .get(&format!("voter-{}", i))
            .unwrap()
            .join(PUBLIC_KEYS_FILE);
        let commission_rate = if commission_rates.is_empty() {
            0
        } else {
            commission_rates.remove(0)
        };
        set_validator_config(
            owner_name,
            git_options.clone(),
            owner_identity.as_path(),
            operator_identity.as_path(),
            voter_identity.as_path(),
            commission_rate,
            i as u16,
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
    root_private_key: Option<&Ed25519PrivateKey>,
    users: Vec<String>,
    chain_id: ChainId,
) -> GitOptions {
    let git_options = git_options();
    let layout_file = TempPath::new();
    layout_file.create_as_file().unwrap();
    let layout_file = layout_file.path();

    create_layout_file(
        layout_file,
        root_private_key.map(|inner| inner.public_key()),
        users,
        chain_id,
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

    // Add framework
    add_framework_to_dir(git_options.local_repository_dir.as_ref().unwrap().as_path());
    git_options
}

/// Add framework to git directory
fn add_framework_to_dir(git_dir: &Path) {
    velor_cached_packages::head_release_bundle()
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
    root_public_key: Option<Ed25519PublicKey>,
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
    layout.root_key = root_public_key;
    layout.users = users;
    layout.chain_id = chain_id;
    layout.is_test = true;
    layout.total_supply = Some(INITIAL_BALANCE * 16);

    write_to_file(
        file,
        "Layout file",
        serde_yaml::to_string(&layout).unwrap().as_bytes(),
    )
    .unwrap();
}

/// Generate keys for a "user"
async fn generate_keys(dir: &Path, name: &str) -> PathBuf {
    let output_dir = dir.join(name);
    let command = GenerateKeys {
        pool_address_args: OptionalPoolAddressArgs { pool_address: None },
        rng_args: RngArgs::from_string_seed(name),
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
    owner_identity_file: &Path,
    operator_identity_file: &Path,
    voter_identity_file: &Path,
    commission_percentage: u64,
    port: u16,
) {
    let command = SetValidatorConfiguration {
        username,
        git_options,
        owner_public_identity_file: Some(owner_identity_file.to_path_buf()),
        validator_host: HostAndPort::from_str(&format!("localhost:{}", port)).unwrap(),
        stake_amount: 100_000_000_000_000,
        full_node_host: None,
        operator_public_identity_file: Some(operator_identity_file.to_path_buf()),
        voter_public_identity_file: Some(voter_identity_file.to_path_buf()),
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
    admin_identities: &[PublicIdentity],
    operator_identities: &[PublicIdentity],
    employee_groups: &[Vec<AccountAddress>],
    join_during_genesis: &[bool],
) {
    TestValidator::new_test_set(Some(employee_groups.len()), Some(INITIAL_BALANCE));
    let employee_vesting_accounts: Vec<_> = employee_groups
        .iter()
        .enumerate()
        .map(|(index, accounts)| {
            let admin_identity = admin_identities[index].clone();
            let operator_identity = operator_identities[index].clone();
            let validator_config = if *join_during_genesis.get(index).unwrap() {
                ValidatorConfiguration {
                    owner_account_address: admin_identity.account_address.into(),
                    owner_account_public_key: admin_identity.account_public_key.clone(),
                    operator_account_address: operator_identity.account_address.into(),
                    operator_account_public_key: operator_identity.account_public_key.clone(),
                    voter_account_address: admin_identity.account_address.into(),
                    voter_account_public_key: admin_identity.account_public_key,
                    consensus_public_key: operator_identity.consensus_public_key,
                    proof_of_possession: operator_identity.consensus_proof_of_possession,
                    validator_network_public_key: operator_identity.validator_network_public_key,
                    validator_host: Some(HostAndPort::from_str("localhost:8080").unwrap()),
                    full_node_network_public_key: operator_identity.full_node_network_public_key,
                    full_node_host: Some(HostAndPort::from_str("localhost:8081").unwrap()),
                    stake_amount: 2 * INITIAL_BALANCE,
                    commission_percentage: 0,
                    join_during_genesis: true,
                }
            } else {
                ValidatorConfiguration {
                    owner_account_address: admin_identity.account_address.into(),
                    owner_account_public_key: admin_identity.account_public_key.clone(),
                    operator_account_address: operator_identity.account_address.into(),
                    operator_account_public_key: operator_identity.account_public_key,
                    voter_account_address: admin_identity.account_address.into(),
                    voter_account_public_key: admin_identity.account_public_key,
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
                accounts: accounts.iter().map(|addr| addr.into()).collect(),
                validator: validator_config,
                vesting_schedule_numerators: vec![3, 3, 3, 3, 1],
                vesting_schedule_denominator: 48,
                beneficiary_resetter: AccountAddress::from_hex_literal("0x101").unwrap().into(),
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
