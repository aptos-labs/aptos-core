// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::shared::{get_shuffle_dir, send};
use anyhow::{Context, Result};
use diem_config::config::NodeConfig;
use diem_crypto::PrivateKey;
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
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::time::SystemTime;
use structopt::StructOpt;

// Creates new account from randomly generated private/public key pair.
pub fn handle() -> Result<()> {
    let node_config_path = &get_shuffle_dir();
    if !Path::new(node_config_path.as_path()).is_dir() {
        println!("A node hasn't been created yet! Run shuffle node first");
        return Ok(());
    }
    let dev_keys_dir = &node_config_path.join("keys");
    let address_dir = &node_config_path.join("addresses");

    if !Path::new(dev_keys_dir.as_path()).is_dir() && !Path::new(address_dir.as_path()).is_dir() {
        fs::create_dir(dev_keys_dir)?;
        fs::create_dir(address_dir)?;
        fs::set_permissions(dev_keys_dir, fs::Permissions::from_mode(0o700))?;
        fs::set_permissions(address_dir, fs::Permissions::from_mode(0o700))?;
    }

    let time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let dev_key_filepath = &dev_keys_dir.join(time.as_secs().to_string() + " dev.key");
    let new_account_key = generate_key::generate_and_save_key(dev_key_filepath);
    fs::set_permissions(dev_key_filepath, fs::Permissions::from_mode(0o600))?;

    let config_path = node_config_path.join("nodeconfig/0").join("node.yaml");
    let config = NodeConfig::load(&config_path)
        .with_context(|| format!("Failed to load NodeConfig from file: {:?}", config_path))?;
    let root_key_path = node_config_path.join("nodeconfig").join("mint.key");
    let root_account_key = load_key(root_key_path);
    let json_rpc_url = format!("http://0.0.0.0:{}", config.json_rpc.address.port());
    println!("Connecting to {}...", json_rpc_url);

    let client = BlockingClient::new(json_rpc_url);
    let root_seq_num = client
        .get_account(account_config::treasury_compliance_account_address())?
        .into_inner()
        .unwrap()
        .sequence_number;
    let mut root_account = LocalAccount::new(
        account_config::treasury_compliance_account_address(),
        root_account_key,
        root_seq_num,
    );
    let new_account = LocalAccount::new(
        AuthenticationKey::ed25519(&new_account_key.public_key()).derived_address(),
        new_account_key,
        0,
    );

    if client
        .get_account(new_account.address())
        .unwrap()
        .into_inner()
        .is_some()
    {
        println!("Account already exists: {}", new_account.address());
        println!(
            "Private key: {}",
            ::hex::encode(new_account.private_key().to_bytes())
        );
        println!("Public key: {}", new_account.public_key());
        return Ok(());
    }

    // Create a new account.
    println!("Create a new account...",);
    let create_new_account_txn = root_account.sign_with_transaction_builder(
        TransactionFactory::new(ChainId::test()).payload(
            encode_create_parent_vasp_account_script_function(
                Currency::XUS.type_tag(),
                0,
                new_account.address(),
                new_account.authentication_key().prefix().to_vec(),
                vec![],
                false,
            ),
        ),
    );
    send(&client, create_new_account_txn)?;
    println!("Successfully created account {}", new_account.address());
    println!(
        "Private key: {}",
        ::hex::encode(new_account.private_key().to_bytes())
    );
    println!("Public key: {}", new_account.public_key());

    let account_filepath = &address_dir.join(time.as_secs().to_string() + ".address");

    let mut file = fs::File::create(account_filepath)?;
    file.write_all(new_account
        .address()
        .to_string()
        .as_ref()
    )?;

    Ok(())
}

#[derive(Debug, StructOpt)]
pub enum AccountCommand {
    // Rotates dev.key with new private key and saves old key in /keys
    #[structopt(about = "Creates new account with randomly generated private/public key")]
    New,
}

pub fn handle_package_commands(cmd: AccountCommand) -> Result<()> {
    match cmd {
        AccountCommand::New => {
            handle()?;
        }
    }
    Ok(())
}
