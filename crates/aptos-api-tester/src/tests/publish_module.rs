// Copyright © Aptos Foundation

use crate::utils::{
    create_and_fund_account, get_client, get_faucet_client, NetworkName, TestFailure,
};
use anyhow::{anyhow, Result};
use aptos_api_types::HexEncodedBytes;
use aptos_cached_packages::aptos_stdlib::EntryFunctionCall;
use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_logger::info;
use aptos_rest_client::Client;
use aptos_sdk::{
    bcs,
    token_client::{build_and_submit_transaction, TransactionOptions},
    types::LocalAccount,
};
use aptos_types::{
    account_address::AccountAddress,
    transaction::{EntryFunction, TransactionPayload},
};
use move_core_types::{ident_str, language_storage::ModuleId};
use std::{collections::BTreeMap, path::PathBuf};

// fail messages
static FAIL_BYTECODE: &str = "wrong bytecode";
static FAIL_MODULE_INTERACTION: &str = "module interaction isn't reflected correctly";
static ERROR_NO_BYTECODE: &str = "error while getting bytecode from blobs";
static ERROR_MODULE_INTERACTION: &str = "module interaction isn't reflected";

/// Helper function that publishes module and returns the bytecode.
async fn publish_module(client: &Client, account: &mut LocalAccount) -> Result<HexEncodedBytes> {
    // get file to compile
    let move_dir = PathBuf::from("./aptos-move/move-examples/hello_blockchain");

    // insert address
    let mut named_addresses: BTreeMap<String, AccountAddress> = BTreeMap::new();
    named_addresses.insert("hello_blockchain".to_string(), account.address());

    // build options
    let options = BuildOptions {
        named_addresses,
        ..BuildOptions::default()
    };

    // build module
    let package = BuiltPackage::build(move_dir, options)?;
    let blobs = package.extract_code();
    let metadata = package.extract_metadata()?;

    // create payload
    let payload: aptos_types::transaction::TransactionPayload =
        EntryFunctionCall::CodePublishPackageTxn {
            metadata_serialized: bcs::to_bytes(&metadata)
                .expect("PackageMetadata should deserialize"),
            code: blobs.clone(),
        }
        .encode();

    // create and submit transaction
    let pending_txn =
        build_and_submit_transaction(client, account, payload, TransactionOptions::default())
            .await?;
    client.wait_for_transaction(&pending_txn).await?;

    let blob = match blobs.get(0) {
        Some(bytecode) => bytecode.clone(),
        None => {
            info!("error: {}", ERROR_NO_BYTECODE);
            return Err(anyhow!(ERROR_NO_BYTECODE));
        },
    };

    Ok(HexEncodedBytes::from(blob))
}

/// Helper function that interacts with the message module.
async fn set_message(client: &Client, account: &mut LocalAccount, message: &str) -> Result<()> {
    // create payload
    let payload = TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(account.address(), ident_str!("message").to_owned()),
        ident_str!("set_message").to_owned(),
        vec![],
        vec![bcs::to_bytes(message)?],
    ));

    // create and submit transaction
    let pending_txn =
        build_and_submit_transaction(client, account, payload, TransactionOptions::default())
            .await?;
    client.wait_for_transaction(&pending_txn).await?;

    Ok(())
}

/// Helper function that gets back the result of the interaction.
async fn get_message(client: &Client, address: AccountAddress) -> Option<String> {
    let resource = match client
        .get_account_resource(
            address,
            format!("{}::message::MessageHolder", address.to_hex_literal()).as_str(),
        )
        .await
    {
        Ok(response) => response.into_inner()?,
        Err(_) => return None,
    };

    Some(resource.data.get("message")?.as_str()?.to_owned())
}

/// Tests module publishing and interaction. Checks that:
///   - module data exists
///   - can interact with module
///   - resources reflect interaction
pub async fn test_publishmodule(
    client: &Client,
    account: &mut LocalAccount,
) -> Result<(), TestFailure> {
    // publish module
    let blob = publish_module(client, account).await?;

    // check module data
    let response = client
        .get_account_module(account.address(), "message")
        .await?;

    let expected_bytecode = &blob;
    let actual_bytecode = &response.inner().bytecode;

    if expected_bytecode != actual_bytecode {
        info!(
            "fail: {}, expected {:?}, got {:?}",
            FAIL_BYTECODE, expected_bytecode, actual_bytecode
        );
        return Err(TestFailure::Fail(FAIL_BYTECODE));
    }

    // interact with module
    let message = "test message";
    set_message(client, account, message).await?;

    // check that the message is sent
    let expected_message = message.to_string();
    let actual_message = match get_message(client, account.address()).await {
        Some(message) => message,
        None => {
            info!("error: {}", ERROR_MODULE_INTERACTION);
            return Err(TestFailure::Error(anyhow!(ERROR_MODULE_INTERACTION)));
        },
    };

    if expected_message != actual_message {
        info!(
            "fail: {}, expected {:?}, got {:?}",
            FAIL_MODULE_INTERACTION, expected_message, actual_message
        );
        return Err(TestFailure::Fail(FAIL_MODULE_INTERACTION));
    }

    Ok(())
}

pub async fn setup_and_run_publishmodule(network_name: NetworkName) -> Result<(), TestFailure> {
    // spin up clients
    let client = get_client(network_name);
    let faucet_client = get_faucet_client(network_name);

    // create and fund accounts
    let mut account = create_and_fund_account(&faucet_client).await?;

    // run test
    test_publishmodule(&client, &mut account).await
}
