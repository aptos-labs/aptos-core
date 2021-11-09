// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::shared::{send_transaction, Home, Network, NetworkHome, LOCALHOST_NAME};
use anyhow::{anyhow, Result};
use diem_crypto::PrivateKey;

use diem_infallible::duration_since_epoch;
use diem_sdk::{
    client::BlockingClient,
    transaction_builder::{Currency, TransactionFactory},
    types::LocalAccount,
};
use diem_types::{
    account_address::AccountAddress,
    account_config,
    chain_id::ChainId,
    transaction::{authenticator::AuthenticationKey, ScriptFunction, TransactionPayload},
};
use generate_key::load_key;
use move_core_types::{
    ident_str,
    language_storage::{ModuleId, TypeTag},
};
use std::{
    io,
    path::{Path, PathBuf},
};

// Creates new account from randomly generated private/public key pair.
pub fn handle(home: &Home, root: Option<PathBuf>, network: Network) -> Result<()> {
    let network_home =
        NetworkHome::new(home.get_networks_path().join(network.get_name()).as_path());
    check_nodeconfig_exists_if_localhost_used(home, &network)?;

    if network_home.get_latest_account_path().exists() {
        match user_wants_another_key(&network_home) {
            true => archive_current_files_in_latest(&network_home)?,
            false => return Ok(()),
        }
    }

    network_home.generate_network_base_path_if_nonexistent()?;

    println!("Connecting to {}...", network.get_json_rpc_url()?);
    let client = BlockingClient::new(network.get_json_rpc_url()?);
    let factory = TransactionFactory::new(ChainId::test());

    if let Some(input_root_key) = root {
        network_home.save_root_key(input_root_key.as_path())?
    }

    network_home.generate_network_accounts_path_if_nonexistent()?;
    network_home.generate_network_latest_account_path_if_nonexistent()?;

    let mut treasury_account = get_treasury_account(&client, home.get_root_key_path())?;
    let new_account = generate_new_account(&network_home)?;
    create_local_account(&mut treasury_account, &new_account, &factory, &client)?;

    network_home.generate_shuffle_test_path_if_nonexistent()?;
    let test_account = generate_test_account(&network_home)?;
    create_local_account(&mut treasury_account, &test_account, &factory, &client)
}

fn check_nodeconfig_exists_if_localhost_used(home: &Home, network: &Network) -> Result<()> {
    match network.get_name().as_str() {
        LOCALHOST_NAME => match home.get_node_config_path().is_dir() {
            true => Ok(()),
            false => Err(anyhow!(
                "A node hasn't been created yet! Run shuffle node first"
            )),
        },
        _ => Ok(()),
    }
}

fn user_wants_another_key(network_home: &NetworkHome) -> bool {
    let key_path = network_home.get_latest_account_key_path();
    let prev_public_key = generate_key::load_key(&key_path).public_key();
    println!(
        "Public Key already exists: {}",
        ::hex::encode(prev_public_key.to_bytes())
    );
    println!("Are you sure you want to generate a new key? [y/n]");

    let user_response = ask_user_if_they_want_key(String::new());
    delegate_user_response(user_response.as_str())
}

fn ask_user_if_they_want_key(mut user_response: String) -> String {
    io::stdin()
        .read_line(&mut user_response)
        .expect("Failed to read line");
    user_response.trim().to_owned()
}

fn delegate_user_response(user_response: &str) -> bool {
    match user_response {
        "y" => true,
        "n" => false,
        _ => {
            println!("Please restart and enter either y or n");
            false
        }
    }
}

fn archive_current_files_in_latest(network_home: &NetworkHome) -> Result<()> {
    let time = duration_since_epoch();
    let archive_dir = network_home.create_archive_dir(time)?;
    network_home.archive_old_key(&archive_dir)?;
    network_home.archive_old_address(&archive_dir)
}

fn generate_new_account(network_home: &NetworkHome) -> Result<LocalAccount> {
    let new_account_key = network_home.generate_key_file()?;
    let public_key = new_account_key.public_key();
    network_home.generate_address_file(&public_key)?;
    Ok(LocalAccount::new(
        AuthenticationKey::ed25519(&public_key).derived_address(),
        new_account_key,
        0,
    ))
}

fn generate_test_account(network_home: &NetworkHome) -> Result<LocalAccount> {
    let test_key = network_home.generate_testkey_file()?;
    let public_test_key = test_key.public_key();
    network_home.generate_testkey_address_file(&test_key.public_key())?;
    Ok(LocalAccount::new(
        AuthenticationKey::ed25519(&public_test_key).derived_address(),
        test_key,
        0,
    ))
}

pub fn get_treasury_account(client: &BlockingClient, root_key_path: &Path) -> Result<LocalAccount> {
    let treasury_account_key = load_key(root_key_path);
    let treasury_seq_num = client
        .get_account(account_config::treasury_compliance_account_address())?
        .into_inner()
        .unwrap()
        .sequence_number;
    Ok(LocalAccount::new(
        account_config::treasury_compliance_account_address(),
        treasury_account_key,
        treasury_seq_num,
    ))
}

pub fn create_local_account(
    treasury_account: &mut LocalAccount,
    new_account: &LocalAccount,
    factory: &TransactionFactory,
    client: &BlockingClient,
) -> Result<()> {
    println!("Creating a new account onchain...");
    if client
        .get_account(new_account.address())
        .unwrap()
        .into_inner()
        .is_some()
    {
        println!("Account already exists: {}", new_account.address());
    } else {
        let create_new_account_txn = treasury_account.sign_with_transaction_builder(
            factory.payload(encode_create_parent_vasp_account_script_function(
                Currency::XUS.type_tag(),
                0,
                new_account.address(),
                new_account.authentication_key().prefix().to_vec(),
                vec![],
                false,
            )),
        );
        send_transaction(client, create_new_account_txn)?;
        println!("Successfully created account {}", new_account.address());
    }
    println!("Public key: {}", new_account.public_key());
    Ok(())
}

fn encode_create_parent_vasp_account_script_function(
    coin_type: TypeTag,
    sliding_nonce: u64,
    new_account_address: AccountAddress,
    auth_key_prefix: Vec<u8>,
    human_name: Vec<u8>,
    add_all_currencies: bool,
) -> TransactionPayload {
    TransactionPayload::ScriptFunction(ScriptFunction::new(
        ModuleId::new(
            AccountAddress::new([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]),
            ident_str!("AccountCreationScripts").to_owned(),
        ),
        ident_str!("create_parent_vasp_account").to_owned(),
        vec![coin_type],
        vec![
            bcs::to_bytes(&sliding_nonce).unwrap(),
            bcs::to_bytes(&new_account_address).unwrap(),
            bcs::to_bytes(&auth_key_prefix).unwrap(),
            bcs::to_bytes(&human_name).unwrap(),
            bcs::to_bytes(&add_all_currencies).unwrap(),
        ],
    ))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::shared;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_check_nodeconfig_exists_if_localhost_used() {
        let tmpdir = tempdir().unwrap();
        let dir_path = tmpdir.path();
        let home = Home::new(dir_path).unwrap();
        assert_eq!(
            check_nodeconfig_exists_if_localhost_used(&home, &shared::Network::default()).is_err(),
            true
        );
        fs::create_dir_all(dir_path.join(".shuffle/nodeconfig")).unwrap();
        assert_eq!(
            check_nodeconfig_exists_if_localhost_used(&home, &shared::Network::default()).is_err(),
            false
        );
    }

    #[test]
    fn test_delegate_user_response() {
        assert_eq!(delegate_user_response("a"), false);
        assert_eq!(delegate_user_response("n"), false);
        assert_eq!(delegate_user_response("y"), true);
    }

    #[test]
    fn test_ask_user_if_they_want_key() {
        assert_eq!(ask_user_if_they_want_key("y".to_string()), "y".to_string());
        assert_eq!(ask_user_if_they_want_key("y ".to_string()), "y".to_string());
        assert_eq!(ask_user_if_they_want_key("n".to_string()), "n".to_string());
        assert_eq!(ask_user_if_they_want_key("n ".to_string()), "n".to_string());
    }
}
