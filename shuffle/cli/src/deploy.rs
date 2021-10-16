// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::shared::{build_move_packages, get_shuffle_dir, send};
use anyhow::{anyhow, Result};
use diem_crypto::PrivateKey;
use diem_sdk::{
    client::BlockingClient,
    transaction_builder::TransactionFactory,
    types::{
        transaction::{Module, TransactionPayload},
        LocalAccount,
    },
};
use diem_types::{
    account_state::AccountState, account_state_blob::AccountStateBlob, chain_id::ChainId,
    transaction::authenticator::AuthenticationKey,
};
use generate_key::load_key;
use move_binary_format::{file_format::CompiledModule, normalized};
use move_lang::compiled_unit::{CompiledUnit, NamedCompiledModule};
use move_package::compilation::compiled_package::CompiledPackage;
use std::{collections::HashSet, convert::TryFrom, path::Path};

/// Deploys shuffle's main Move Package to the sender's address.
pub fn handle(project_path: &Path) -> Result<()> {
    let account_key_path = get_shuffle_dir()
        .join("accounts")
        .join("latest")
        .join("dev.key");
    if !account_key_path.exists() {
        return Err(anyhow!(
            "An account hasn't been created yet! Run shuffle account first."
        ));
    }
    let compiled_package = build_move_packages(project_path)?;
    publish_packages_as_transaction(&account_key_path, compiled_package)
}

fn publish_packages_as_transaction(
    account_key_path: &Path,
    compiled_package: CompiledPackage,
) -> Result<()> {
    let new_account_key = load_key(account_key_path);
    let json_rpc_url = format!("http://0.0.0.0:{}", 8080); // TODO: Hardcoded to local devnet
    let factory = TransactionFactory::new(ChainId::test());
    println!("Connecting to {}", json_rpc_url);

    let client = BlockingClient::new(json_rpc_url);

    println!("Using Public Key {}", &new_account_key.public_key());
    let derived_address =
        AuthenticationKey::ed25519(&new_account_key.public_key()).derived_address();
    println!(
        "Sending txn from address {}",
        derived_address.to_hex_literal()
    );

    // Send a module transaction
    let seq_number = client
        .get_account(derived_address)?
        .into_inner()
        .ok_or_else(|| anyhow::anyhow!("missing AccountView"))?
        .sequence_number;
    let mut new_account = LocalAccount::new(derived_address, new_account_key, seq_number);
    send_module_transaction(&compiled_package, &client, &mut new_account, &factory)?;
    check_module_exists(&client, &new_account)
}

pub fn send_module_transaction(
    compiled_package: &CompiledPackage,
    client: &BlockingClient,
    account: &mut LocalAccount,
    factory: &TransactionFactory,
) -> Result<()> {
    let compiled_units = compiled_package.clone().compiled_units;
    let mut uniq_modules: HashSet<String> = HashSet::new(); // Apparently modules can appear twice in compiled units, ensure uniq
    for unit in compiled_units {
        match unit {
            CompiledUnit::Module(NamedCompiledModule { name, module, .. }) => {
                let namecpy = name.to_string();
                if uniq_modules.contains(&namecpy) {
                    continue;
                }
                println!("Deploying Module: {}", namecpy);
                uniq_modules.insert(namecpy);
                let mut binary = vec![];
                module.serialize(&mut binary)?;
                let publish_txn = account.sign_with_transaction_builder(
                    factory.payload(TransactionPayload::Module(Module::new(binary))),
                );

                send(client, publish_txn)?;
            }
            _ => {
                continue;
            }
        }
    }
    println!("Success!");
    Ok(())
}

pub fn check_module_exists(client: &BlockingClient, account: &LocalAccount) -> Result<()> {
    let account_state_blob: AccountStateBlob = {
        let blob = client
            .get_account_state_with_proof(account.address(), None, None)?
            .into_inner()
            .blob
            .ok_or_else(|| anyhow::anyhow!("missing account state blob"))?;
        bcs::from_bytes(&blob)?
    };
    let account_state = AccountState::try_from(&account_state_blob)?;
    let mut modules = vec![];
    for module_bytes in account_state.get_modules() {
        modules.push(normalized::Module::new(
            &CompiledModule::deserialize(module_bytes)
                .map_err(|e| anyhow!("Failure deserializing module: {:?}", e))?,
        ));
    }
    println!("move modules length: {}", modules.len());
    println!("move modules name: {}", modules[0].name);

    Ok(())
}
