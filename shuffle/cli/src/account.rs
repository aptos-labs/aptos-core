// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::shared::{get_shuffle_dir, send};
use anyhow::anyhow;
use anyhow::{Context, Result};
use diem_config::config::NodeConfig;
use diem_crypto::ed25519::Ed25519PrivateKey;
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
use std::fs::File;
use std::io::Write;
use std::path::Path;
use structopt::StructOpt;

const NEW_KEY_FILE_CONTENT: &[u8] = include_bytes!("../new_account.key");

// Creates new account from randomly generated private/public key pair.
pub fn handle() -> Result<()> {
    let shuffle_dir = &get_shuffle_dir();
    if !Path::new(shuffle_dir.as_path()).is_dir() {
        return Err(anyhow!(
            "A node hasn't been created yet! Run shuffle node first"
        ));
    }
    println!("{:?}", shuffle_dir);
    let config_path = shuffle_dir.join("nodeconfig/0").join("node.yaml");
    let config = NodeConfig::load(&config_path)
        .with_context(|| format!("Failed to load NodeConfig from file: {:?}", config_path))?;

    let root_key_path = shuffle_dir.join("nodeconfig").join("mint.key");
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

    generate_user_shuffle_dirs(shuffle_dir)?;
    let new_account_key = generate_key_files(shuffle_dir).unwrap();

    let public_key = new_account_key.public_key();
    let new_account = LocalAccount::new(
        AuthenticationKey::ed25519(&public_key).derived_address(),
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
    Ok(())
}

// generates /latest/accounts directories
pub fn generate_user_shuffle_dirs(shuffle_dir: &Path) -> Result<()> {
    let account_dir = &shuffle_dir.join("accounts");
    if !Path::new(account_dir.as_path()).is_dir() {
        fs::create_dir(account_dir)?;
    }
    let latest_dir = &account_dir.join("latest");
    if !Path::new(latest_dir.as_path()).is_dir() {
        fs::create_dir(latest_dir)?;
    }

    Ok(())
}

// generates the dev.key and address files
pub fn generate_key_files(shuffle_dir: &Path) -> Result<Ed25519PrivateKey> {
    let latest_dir = &shuffle_dir.join("accounts").join("latest");
    let dev_key_filepath = &latest_dir.join("dev.key");
    fs::write(dev_key_filepath, NEW_KEY_FILE_CONTENT)?;
    let private_key = generate_key::load_key(&dev_key_filepath);
    let public_key = private_key.public_key();
    let address = AuthenticationKey::ed25519(&public_key).derived_address();
    let account_filepath = &latest_dir.join("address");
    let mut file = File::create(account_filepath)?;
    file.write_all(address.to_string().as_ref())?;
    Ok(private_key)
}

#[derive(Debug, StructOpt)]
pub enum AccountCommand {
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

#[cfg(test)]
mod test {
    use crate::account::{generate_key_files, generate_user_shuffle_dirs};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_generate_user_shuffle_dirs() {
        let dir = tempdir().unwrap();

        generate_user_shuffle_dirs(&dir.path().to_path_buf());
        assert_eq!(dir.path().join("accounts").as_path().is_dir(), true);
        assert_eq!(
            dir.path()
                .join("accounts")
                .join("latest")
                .as_path()
                .is_dir(),
            true
        );
    }

    #[test]
    fn test_generate_user_shuffle_paths() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("accounts").join("latest"));
        generate_key_files(&dir.path().to_path_buf());
        assert_eq!(
            dir.path()
                .join("accounts")
                .join("latest")
                .join("dev.key")
                .as_path()
                .exists(),
            true
        );
        assert_eq!(
            dir.path()
                .join("accounts")
                .join("latest")
                .join("address")
                .as_path()
                .exists(),
            true
        );
    }
}
