// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::shared::send;
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
use std::path::PathBuf;

pub fn handle(project_dir: PathBuf, account_key_path: PathBuf) -> Result<()> {
    let config_path = project_dir.join("nodeconfig/0").join("node.yaml");
    let config = NodeConfig::load(&config_path)
        .with_context(|| format!("Failed to load NodeConfig from file: {:?}", config_path))?;
    let root_key_path = project_dir.join("nodeconfig").join("mint.key");
    let root_account_key = load_key(root_key_path);
    let new_account_key = load_key(account_key_path.as_path());
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
    println!("Using Public Key: {}", &new_account_key.public_key());
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

    Ok(())
}
