// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::shared::{get_home_path, send, Home};
use anyhow::{anyhow, Context, Result};
use diem_config::config::NodeConfig;
use diem_crypto::PrivateKey;

use diem_infallible::duration_since_epoch;
use diem_sdk::{
    client::BlockingClient,
    transaction_builder::{Currency, TransactionFactory},
    types::LocalAccount,
};
use diem_types::{
    account_config, chain_id::ChainId, transaction::authenticator::AuthenticationKey,
};
use generate_key::load_key;
use shuffle_transaction_builder::framework::encode_create_parent_vasp_account_script_function;
use std::{io, path::Path};

// Creates new account from randomly generated private/public key pair.
pub fn handle() -> Result<()> {
    let home = Home::new(get_home_path().as_path())?;
    if !home.get_shuffle_path().is_dir() {
        return Err(anyhow!(
            "A node hasn't been created yet! Run shuffle node first"
        ));
    }

    if home.get_latest_path().exists() {
        let wants_another_key = confirm_user_decision(&home);
        if wants_another_key {
            let time = duration_since_epoch();
            let archive_dir = home.create_archive_dir(time)?;
            home.archive_old_key(&archive_dir)?;
            home.archive_old_address(&archive_dir)?;
        } else {
            return Ok(());
        }
    }

    let config = NodeConfig::load(&home.get_node_config_path()).with_context(|| {
        format!(
            "Failed to load NodeConfig from file: {:?}",
            home.get_node_config_path()
        )
    })?;
    let json_rpc_url = format!("http://0.0.0.0:{}", config.json_rpc.address.port());
    println!("Connecting to {}...", json_rpc_url);
    let client = BlockingClient::new(json_rpc_url);
    let factory = TransactionFactory::new(ChainId::test());

    let mut root_account = get_root_account(&client, home.get_root_key_path());

    home.generate_shuffle_accounts_path()?;
    home.generate_shuffle_latest_path()?;

    let new_account_key = home.generate_key_file()?;
    let public_key = new_account_key.public_key();
    home.generate_address_file(&public_key)?;
    let new_account = LocalAccount::new(
        AuthenticationKey::ed25519(&public_key).derived_address(),
        new_account_key,
        0,
    );

    // Create a new account.
    create_account_onchain(&mut root_account, &new_account, &factory, &client)?;

    home.generate_shuffle_test_path()?;
    let test_key = home.generate_testkey_file()?;
    let public_test_key = test_key.public_key();
    home.generate_testkey_address_file(&test_key.public_key())?;
    let test_account = LocalAccount::new(
        AuthenticationKey::ed25519(&public_test_key).derived_address(),
        test_key,
        0,
    );

    create_account_onchain(&mut root_account, &test_account, &factory, &client)
}

pub fn confirm_user_decision(home: &Home) -> bool {
    let key_path = home.get_latest_key_path();
    let prev_key = generate_key::load_key(&key_path);
    println!(
        "Private Key already exists: {}",
        ::hex::encode(prev_key.to_bytes())
    );
    println!("Are you sure you want to generate a new key? [y/n]");

    let mut user_response = String::new();
    io::stdin()
        .read_line(&mut user_response)
        .expect("Failed to read line");
    let user_response = user_response.trim().to_owned();

    if user_response != "y" && user_response != "n" {
        println!("Please restart and enter either y or n");
        return false;
    } else if user_response == "n" {
        return false;
    }

    true
}

pub fn get_root_account(client: &BlockingClient, root_key_path: &Path) -> LocalAccount {
    let root_account_key = load_key(root_key_path);

    let root_seq_num = client
        .get_account(account_config::treasury_compliance_account_address())
        .unwrap()
        .into_inner()
        .unwrap()
        .sequence_number;
    LocalAccount::new(
        account_config::treasury_compliance_account_address(),
        root_account_key,
        root_seq_num,
    )
}

pub fn create_account_onchain(
    root_account: &mut LocalAccount,
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
        let create_new_account_txn = root_account.sign_with_transaction_builder(factory.payload(
            encode_create_parent_vasp_account_script_function(
                Currency::XUS.type_tag(),
                0,
                new_account.address(),
                new_account.authentication_key().prefix().to_vec(),
                vec![],
                false,
            ),
        ));
        send(client, create_new_account_txn)?;
        println!("Successfully created account {}", new_account.address());
    }
    println!(
        "Private key: {}",
        ::hex::encode(new_account.private_key().to_bytes())
    );
    println!("Public key: {}", new_account.public_key());
    Ok(())
}
