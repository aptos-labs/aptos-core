// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::shared::{get_shuffle_dir, send};
use anyhow::{anyhow, Context, Result};
use diem_config::config::NodeConfig;
use diem_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    PrivateKey,
};
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
use std::{fs, fs::File, io, io::Write, path::Path};

const NEW_KEY_FILE_CONTENT: &[u8] = include_bytes!("../new_account.key");

// Creates new account from randomly generated private/public key pair.
pub fn handle() -> Result<()> {
    let shuffle_dir = get_shuffle_dir();
    if !Path::new(shuffle_dir.as_path()).is_dir() {
        return Err(anyhow!(
            "A node hasn't been created yet! Run shuffle node first"
        ));
    }
    let config_path = &shuffle_dir.join("nodeconfig/0").join("node.yaml");
    let config = NodeConfig::load(&config_path)
        .with_context(|| format!("Failed to load NodeConfig from file: {:?}", config_path))?;

    let json_rpc_url = format!("http://0.0.0.0:{}", config.json_rpc.address.port());
    println!("Connecting to {}...", json_rpc_url);
    let client = BlockingClient::new(json_rpc_url);
    let factory = TransactionFactory::new(ChainId::test());

    let mut root_account = get_root_account(&client, &shuffle_dir);

    generate_shuffle_accounts_dir(&shuffle_dir)?;
    let new_account_key = generate_key_file(&shuffle_dir).unwrap();
    let public_key = new_account_key.public_key();
    generate_address_file(&shuffle_dir, &public_key)?;

    let new_account = LocalAccount::new(
        AuthenticationKey::ed25519(&public_key).derived_address(),
        new_account_key,
        0,
    );

    // Create a new account.
    create_account_onchain(&mut root_account, &new_account, &factory, &client)
}

pub fn confirm_user_decision(latest_dir: &Path) -> Result<String> {
    let key_path = latest_dir.join("dev.key");
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
    let mut user_response = user_response.trim().to_owned();

    while user_response != "y" && user_response != "n" {
        println!("Please enter y or n");
        user_response = String::new();
        io::stdin()
            .read_line(&mut user_response)
            .expect("Failed to read line");
        user_response = user_response.trim().to_owned();
    }

    Ok(user_response)
}

pub fn save_old_key(accounts_dir: &Path, saved_key_path: &Path) -> Result<()> {
    let old_key_path = accounts_dir.join("latest").join("dev.key");
    let saved_key_path = saved_key_path.join("dev.key");
    fs::copy(old_key_path, saved_key_path)?;
    Ok(())
}

pub fn save_old_address(accounts_dir: &Path, saved_address_path: &Path) -> Result<()> {
    let old_address_path = accounts_dir.join("latest").join("address");
    let saved_address_path = saved_address_path.join("address");
    fs::copy(old_address_path, saved_address_path)?;
    Ok(())
}

pub fn generate_shuffle_accounts_dir(shuffle_dir: &Path) -> Result<()> {
    let account_dir = &shuffle_dir.join("accounts");
    if !account_dir.as_path().is_dir() {
        fs::create_dir(account_dir)?;
    }
    let latest_dir = &account_dir.join("latest");
    if !latest_dir.as_path().is_dir() {
        fs::create_dir(latest_dir)?;
    }

    Ok(())
}

pub fn generate_key_file(shuffle_dir: &Path) -> Result<Ed25519PrivateKey> {
    let latest_dir = &shuffle_dir.join("accounts").join("latest");
    let dev_key_filepath = &latest_dir.join("dev.key");
    fs::write(dev_key_filepath, NEW_KEY_FILE_CONTENT)?;
    let private_key = generate_key::generate_and_save_key(&dev_key_filepath);
    Ok(private_key)
}

pub fn generate_address_file(shuffle_dir: &Path, public_key: &Ed25519PublicKey) -> Result<()> {
    let latest_dir = &shuffle_dir.join("accounts").join("latest");
    let address = AuthenticationKey::ed25519(public_key).derived_address();
    let account_filepath = &latest_dir.join("address");
    let mut file = File::create(account_filepath)?;
    file.write_all(address.to_string().as_ref())?;
    Ok(())
}

pub fn get_root_account(client: &BlockingClient, shuffle_dir: &Path) -> LocalAccount {
    let root_key_path = shuffle_dir.join("nodeconfig").join("mint.key");
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

#[cfg(test)]
mod test {
    use crate::account::{
        generate_address_file, generate_key_file, generate_shuffle_accounts_dir, save_old_address,
        save_old_key,
    };
    use diem_crypto::PrivateKey;
    use diem_infallible::duration_since_epoch;
    use generate_key::generate_key;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_generate_user_shuffle_dirs() {
        let dir = tempdir().unwrap();
        generate_shuffle_accounts_dir(&dir.path().to_path_buf())
            .expect("Directories weren't created");
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
    fn test_generate_key_path() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("accounts").join("latest"))
            .expect("Directories weren't created");
        generate_key_file(&dir.path().to_path_buf()).expect("Key file wasn't generated");
        assert_eq!(
            dir.path()
                .join("accounts")
                .join("latest")
                .join("dev.key")
                .as_path()
                .exists(),
            true
        );
    }

    #[test]
    fn test_generate_address_path() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("accounts").join("latest"))
            .expect("Directories weren't created");
        let private_key = generate_key();
        let public_key = private_key.public_key();
        generate_address_file(dir.path(), &public_key).expect("Address file wasn't generated");
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

    #[test]
    fn test_save_old_key() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("accounts").join("latest"))
            .expect("Directories weren't created");
        let time = duration_since_epoch();
        fs::create_dir_all(dir.path().join("accounts").join(time.as_secs().to_string()))
            .expect("Directories weren't created");

        generate_key_file(&dir.path().to_path_buf()).expect("Key file wasn't generated");

        let accounts_dir = &dir.path().join("accounts");
        let saved_dir = &dir.path().join("accounts").join(time.as_secs().to_string());
        save_old_key(accounts_dir, saved_dir).expect("Old key wasn't saved");
        let saved_key = generate_key::load_key(saved_dir.join("dev.key"));

        let old_key_path = &dir.path().join("accounts").join("latest").join("dev.key");
        let old_private_key = generate_key::load_key(old_key_path);

        assert_eq!(old_private_key, saved_key);
    }

    #[test]
    fn test_save_old_address() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("accounts").join("latest"))
            .expect("Directories weren't created");
        let time = duration_since_epoch();
        fs::create_dir_all(dir.path().join("accounts").join(time.as_secs().to_string()))
            .expect("Directories weren't created");

        let private_key = generate_key();
        let public_key = private_key.public_key();
        generate_address_file(dir.path(), &public_key).expect("Address file wasn't generated");

        let accounts_dir = &dir.path().join("accounts");
        let saved_dir = &dir.path().join("accounts").join(time.as_secs().to_string());
        save_old_address(accounts_dir, saved_dir).expect("Old address wasn't saved");
        let saved_address = fs::read_to_string(saved_dir.join("address")).unwrap();

        let old_address_path = &dir.path().join("accounts").join("latest").join("address");
        let old_address = fs::read_to_string(old_address_path).unwrap();

        assert_eq!(saved_address, old_address);
    }
}
