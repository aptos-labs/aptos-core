// Copyright © Aptos Foundation

use crate::{
    fail_message::{
        ERROR_COULD_NOT_BUILD_PACKAGE, ERROR_COULD_NOT_CREATE_TRANSACTION,
        ERROR_COULD_NOT_FINISH_TRANSACTION, ERROR_COULD_NOT_FUND_ACCOUNT,
        ERROR_COULD_NOT_SERIALIZE, ERROR_NO_BYTECODE, ERROR_NO_MESSAGE, ERROR_NO_METADATA,
        ERROR_NO_MODULE, FAIL_WRONG_MESSAGE, FAIL_WRONG_MODULE,
    },
    persistent_check,
    utils::{create_and_fund_account, get_client, get_faucet_client, NetworkName, TestFailure},
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

static MODULE_NAME: &str = "message";
static MESSAGE: &str = "test message";

pub async fn test(network_name: NetworkName) -> Result<(), TestFailure> {
    // setup
    let (client, mut account) = setup(network_name).await?;

    // build module
    let package = build_module(account.address()).await?;

    // publish module
    let blob = publish_module(&client, &mut account, package).await?;

    // check module data persistently
    persistent_check::address_bytes(
        "check_module_data",
        check_module_data,
        &client,
        account.address(),
        &blob,
    )
    .await?;

    // set message
    set_message(&client, &mut account).await?;

    // check message persistently
    persistent_check::address("check_message", check_message, &client, account.address()).await?;

    Ok(())
}

// Steps

async fn setup(network_name: NetworkName) -> Result<(Client, LocalAccount), TestFailure> {
    // spin up clients
    let client = get_client(network_name);
    let faucet_client = get_faucet_client(network_name);

    // create account
    let account = match create_and_fund_account(&faucet_client).await {
        Ok(account) => account,
        Err(e) => {
            info!(
                "test: publish_module part: setup ERROR: {}, with error {:?}",
                ERROR_COULD_NOT_FUND_ACCOUNT, e
            );
            return Err(e.into());
        },
    };

    Ok((client, account))
}

async fn build_module(address: AccountAddress) -> Result<BuiltPackage, TestFailure> {
    // get file to compile
    let move_dir = PathBuf::from("./aptos-move/move-examples/hello_blockchain");

    // insert address
    let mut named_addresses: BTreeMap<String, AccountAddress> = BTreeMap::new();
    named_addresses.insert("hello_blockchain".to_string(), address);

    // build options
    let options = BuildOptions {
        named_addresses,
        ..BuildOptions::default()
    };

    // build module
    let package = match BuiltPackage::build(move_dir, options) {
        Ok(package) => package,
        Err(e) => {
            info!(
                "test: publish_module part: publish_module ERROR: {}, with error {:?}",
                ERROR_COULD_NOT_BUILD_PACKAGE, e
            );
            return Err(e.into());
        },
    };

    Ok(package)
}

async fn publish_module(
    client: &Client,
    account: &mut LocalAccount,
    package: BuiltPackage,
) -> Result<HexEncodedBytes, TestFailure> {
    // get bytecode
    let blobs = package.extract_code();

    // get metadata
    let metadata = match package.extract_metadata() {
        Ok(data) => data,
        Err(e) => {
            info!(
                "test: publish_module part: publish_module ERROR: {}, with error {:?}",
                ERROR_NO_METADATA, e
            );
            return Err(e.into());
        },
    };

    // serialize metadata
    let metadata_serialized = match bcs::to_bytes(&metadata) {
        Ok(data) => data,
        Err(e) => {
            info!(
                "test: publish_module part: publish_module ERROR: {}, with error {:?}",
                ERROR_COULD_NOT_SERIALIZE, e
            );
            return Err(anyhow!(e).into());
        },
    };

    // create payload
    let payload: aptos_types::transaction::TransactionPayload =
        EntryFunctionCall::CodePublishPackageTxn {
            metadata_serialized,
            code: blobs.clone(),
        }
        .encode();

    // create transaction
    let pending_txn =
        match build_and_submit_transaction(client, account, payload, TransactionOptions::default())
            .await
        {
            Ok(txn) => txn,
            Err(e) => {
                info!(
                    "test: publish_module part: publish_module ERROR: {}, with error {:?}",
                    ERROR_COULD_NOT_CREATE_TRANSACTION, e
                );
                return Err(e.into());
            },
        };

    // wait for transaction to finish
    if let Err(e) = client.wait_for_transaction(&pending_txn).await {
        info!(
            "test: publish_module part: publish_module ERROR: {}, with error {:?}",
            ERROR_COULD_NOT_FINISH_TRANSACTION, e
        );
        return Err(e.into());
    };

    // get blob for later comparison
    let blob = match blobs.get(0) {
        Some(bytecode) => HexEncodedBytes::from(bytecode.clone()),
        None => {
            info!(
                "test: publish_module part: publish_module ERROR: {}",
                ERROR_NO_BYTECODE
            );
            return Err(anyhow!(ERROR_NO_BYTECODE).into());
        },
    };

    Ok(blob)
}

async fn check_module_data(
    client: &Client,
    address: AccountAddress,
    expected: &HexEncodedBytes,
) -> Result<(), TestFailure> {
    // actual
    let response = match client.get_account_module(address, MODULE_NAME).await {
        Ok(response) => response,
        Err(e) => {
            info!(
                "test: publish_module part: check_module_data ERROR: {}, with error {:?}",
                ERROR_NO_MODULE, e
            );
            return Err(e.into());
        },
    };
    let actual = &response.inner().bytecode;

    // compare
    if expected != actual {
        info!(
            "test: publish_module part: check_module_data FAIL: {}, expected {:?}, got {:?}",
            FAIL_WRONG_MODULE, expected, actual
        );
        return Err(TestFailure::Fail(FAIL_WRONG_MODULE));
    }

    Ok(())
}

async fn set_message(client: &Client, account: &mut LocalAccount) -> Result<(), TestFailure> {
    // set up message
    let message = match bcs::to_bytes(MESSAGE) {
        Ok(data) => data,
        Err(e) => {
            info!(
                "test: publish_module part: set_message ERROR: {}, with error {:?}",
                ERROR_COULD_NOT_SERIALIZE, e
            );
            return Err(anyhow!(e).into());
        },
    };

    // create payload
    let payload = TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(account.address(), ident_str!(MODULE_NAME).to_owned()),
        ident_str!("set_message").to_owned(),
        vec![],
        vec![message],
    ));

    // create transaction
    let pending_txn =
        match build_and_submit_transaction(client, account, payload, TransactionOptions::default())
            .await
        {
            Ok(txn) => txn,
            Err(e) => {
                info!(
                    "test: publish_module part: set_message ERROR: {}, with error {:?}",
                    ERROR_COULD_NOT_CREATE_TRANSACTION, e
                );
                return Err(e.into());
            },
        };

    // wait for transaction to finish
    if let Err(e) = client.wait_for_transaction(&pending_txn).await {
        info!(
            "test: publish_module part: set_message ERROR: {}, with error {:?}",
            ERROR_COULD_NOT_FINISH_TRANSACTION, e
        );
        return Err(e.into());
    };

    Ok(())
}

async fn check_message(client: &Client, address: AccountAddress) -> Result<(), TestFailure> {
    // expected
    let expected = MESSAGE.to_string();

    // actual
    let actual = match get_message(client, address).await {
        Some(message) => message,
        None => {
            info!(
                "test: publish_module part: check_message ERROR: {}",
                ERROR_NO_MESSAGE
            );
            return Err(anyhow!(ERROR_NO_MESSAGE).into());
        },
    };

    // compare
    if expected != actual {
        info!(
            "test: publish_module part: check_message FAIL: {}, expected {:?}, got {:?}",
            FAIL_WRONG_MESSAGE, expected, actual
        );
        return Err(TestFailure::Fail(FAIL_WRONG_MESSAGE));
    }

    Ok(())
}

// Utils

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
