// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{new::MESSAGE_EXAMPLE_PATH, shared::send};
use anyhow::{anyhow, Context, Result};
use diem_config::config::NodeConfig;
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
use move_package::compilation::{
    compiled_package::{CompiledPackage, OnDiskCompiledPackage},
    package_layout::CompiledPackageLayout,
};
use std::{collections::HashSet, convert::TryFrom, path::Path};

// TODO: Deploys the Message Move Package to the sender's address
// 0x24163afcc6e33b0a9473852e18327fa9. This has been hardcoded via named
// addresses in diem_framework_named_addresses.
pub fn handle(project_path: &Path, account_key_path: &Path) -> Result<()> {
    let _compiled_packages = build_move_packages(project_path)?;
    // TODO: Feed the compiled packages into method below for remote publishing
    publish_packages_as_transaction(project_path, account_key_path)
}

/// Builds the packages in the shuffle project using the move package system.
fn build_move_packages(project_path: &Path) -> Result<CompiledPackage> {
    println!("Building Examples...");
    let pkgdir = project_path.join(MESSAGE_EXAMPLE_PATH);
    let config = move_package::BuildConfig {
        dev_mode: true,
        test_mode: false,
        generate_docs: false,
        generate_abis: true,
    };

    config.compile_package(pkgdir.as_path(), &mut std::io::stdout())
}

fn publish_packages_as_transaction(project_path: &Path, account_key_path: &Path) -> Result<()> {
    let config_path = project_path.join("nodeconfig/0").join("node.yaml");
    let config = NodeConfig::load(&config_path)
        .with_context(|| format!("Failed to load NodeConfig from file: {:?}", config_path))?;
    let new_account_key = load_key(account_key_path);
    let json_rpc_url = format!("http://0.0.0.0:{}", config.json_rpc.address.port());
    let factory = TransactionFactory::new(ChainId::test());
    println!("Connecting to {}", json_rpc_url);

    let client = BlockingClient::new(json_rpc_url);

    let derived_address =
        AuthenticationKey::ed25519(&new_account_key.public_key()).derived_address();
    println!(
        "Sending txn from address {}",
        derived_address.to_hex_literal()
    );
    let mut new_account = LocalAccount::new(derived_address, new_account_key, 0);

    // ================= Send a module transaction ========================

    let pkg_path = project_path
        .join(MESSAGE_EXAMPLE_PATH)
        .join(CompiledPackageLayout::Root.path())
        .join(MESSAGE_EXAMPLE_PATH)
        .join(CompiledPackageLayout::BuildInfo.path());
    let package = OnDiskCompiledPackage::from_path(pkg_path.as_path())?.into_compiled_package()?;
    let compiled_units = package.compiled_units;
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
                let publish_txn = new_account.sign_with_transaction_builder(
                    factory.payload(TransactionPayload::Module(Module::new(binary))),
                );

                send(&client, publish_txn)?;
            }
            _ => {
                continue;
            }
        }
    }
    println!("Success!");

    // ================= Get modules in the account  ========================
    // Assumes we've deployed to the shuffle developer's address.

    let account_state_blob: AccountStateBlob = {
        let blob = client
            .get_account_state_with_proof(new_account.address(), None, None)
            .unwrap()
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
