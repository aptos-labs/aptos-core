// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{account, deploy, shared};
use anyhow::{anyhow, Context, Result};
use diem_config::config::{NodeConfig, DEFAULT_PORT};
use diem_crypto::PrivateKey;
use diem_sdk::{
    client::BlockingClient, transaction_builder::TransactionFactory, types::LocalAccount,
};
use diem_types::{chain_id::ChainId, transaction::authenticator::AuthenticationKey};
use std::{collections::HashMap, path::Path, process::Command};

pub fn handle(project_path: &Path) -> Result<()> {
    let _config = shared::read_config(project_path)?;
    shared::generate_typescript_libraries(project_path)?;

    let shuffle_dir = &shared::get_shuffle_dir();
    if !Path::new(shuffle_dir.as_path()).is_dir() {
        return Err(anyhow!(
            "A node hasn't been created yet! Run shuffle node first"
        ));
    }
    println!("{:?}", shuffle_dir);
    let config_path = shuffle_dir.join("nodeconfig/0").join("node.yaml");
    let config = NodeConfig::load(&config_path)
        .with_context(|| format!("Failed to load NodeConfig from file: {:?}", config_path))?;

    let json_rpc_url = format!("http://0.0.0.0:{}", config.json_rpc.address.port());
    println!("Connecting to {}...", json_rpc_url);
    let client = BlockingClient::new(json_rpc_url);
    let factory = TransactionFactory::new(ChainId::test());

    let mut root_account = account::get_root_account(&client, shuffle_dir);

    let latest_dir = &shuffle_dir.join("accounts").join("latest");
    let dev_key_filepath = &latest_dir.join("dev.key");

    // ================= Set up account ========================

    // TODO: generate random key by using let new_account_key = generate_key::generate_key();
    let new_account_key = generate_key::load_key(&dev_key_filepath);
    let public_key = new_account_key.public_key();
    let derived_address = AuthenticationKey::ed25519(&public_key).derived_address();
    let mut new_account = LocalAccount::new(derived_address, new_account_key, 0);

    account::create_account_onchain(&mut root_account, &new_account, &factory, &client)?;

    // ================= Send a module transaction ========================

    let account_seq_num = client
        .get_account(new_account.address())?
        .into_inner()
        .unwrap()
        .sequence_number;
    *new_account.sequence_number_mut() = account_seq_num;
    println!(
        "Deploy move module in {} ----------",
        project_path.to_string_lossy().to_string()
    );

    let compiled_package = deploy::build_move_packages(project_path)?;
    deploy::send_module_transaction(&compiled_package, &client, &mut new_account, &factory)?;
    deploy::check_module_exists(&client, &new_account)?;

    // ================= Run shuffle test ========================

    let tests_path_string = project_path
        .join("e2e")
        .as_path()
        .to_string_lossy()
        .to_string();

    let mut filtered_envs: HashMap<String, String> = HashMap::new();
    filtered_envs.insert(
        String::from("PROJECT_PATH"),
        project_path.to_str().unwrap().to_string(),
    );
    filtered_envs.insert(
        String::from("SHUFFLE_HOME"),
        shared::get_shuffle_dir().to_str().unwrap().to_string(),
    );

    Command::new("deno")
        .args([
            "test",
            "--unstable",
            tests_path_string.as_str(),
            "--allow-env=PROJECT_PATH,SHUFFLE_HOME",
            "--allow-read",
            format!(
                "--allow-net=:{},:{}",
                DEFAULT_PORT,
                config.json_rpc.address.port()
            )
            .as_str(),
        ])
        .envs(&filtered_envs)
        .spawn()
        .expect("deno failed to start, is it installed? brew install deno")
        .wait()?;
    Ok(())
}
