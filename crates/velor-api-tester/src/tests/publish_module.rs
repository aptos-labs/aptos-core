// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    consts::FUND_AMOUNT,
    persistent_check,
    strings::{
        BUILD_MODULE, CHECK_ACCOUNT_DATA, CHECK_MESSAGE, CHECK_MODULE_DATA,
        ERROR_COULD_NOT_BUILD_PACKAGE, ERROR_COULD_NOT_CREATE_AND_SUBMIT_TRANSACTION,
        ERROR_COULD_NOT_FINISH_TRANSACTION, ERROR_COULD_NOT_FUND_ACCOUNT,
        ERROR_COULD_NOT_SERIALIZE, ERROR_NO_BYTECODE, ERROR_NO_MESSAGE, ERROR_NO_METADATA,
        ERROR_NO_MODULE, FAIL_WRONG_MESSAGE, FAIL_WRONG_MODULE, PUBLISH_MODULE, SETUP, SET_MESSAGE,
    },
    time_fn,
    tokenv1_client::{build_and_submit_transaction, TransactionOptions},
    utils::{
        check_balance, create_and_fund_account, emit_step_metrics, NetworkName, TestFailure,
        TestName,
    },
};
use anyhow::{anyhow, Result};
use velor_api_types::{HexEncodedBytes, U64};
use velor_cached_packages::velor_stdlib::EntryFunctionCall;
use velor_framework::{BuildOptions, BuiltPackage};
use velor_logger::error;
use velor_rest_client::Client;
use velor_sdk::{bcs, types::LocalAccount};
use velor_types::{
    account_address::AccountAddress,
    transaction::{EntryFunction, TransactionPayload},
};
use move_core_types::{ident_str, language_storage::ModuleId};
use std::{collections::BTreeMap, path::PathBuf};

static MODULE_NAME: &str = "message";
static TEST_MESSAGE: &str = "test message";

/// Tests module publishing and interaction. Checks that:
///   - can publish module
///   - module data exists
///   - can interact with module
///   - interaction is reflected correctly
pub async fn test(network_name: NetworkName, run_id: &str) -> Result<(), TestFailure> {
    // setup
    let (client, mut account) = emit_step_metrics(
        time_fn!(setup, network_name),
        TestName::PublishModule,
        SETUP,
        network_name,
        run_id,
    )?;

    // persistently check that API returns correct account data (auth key and sequence number)
    emit_step_metrics(
        time_fn!(
            persistent_check::address,
            CHECK_ACCOUNT_DATA,
            check_account_data,
            &client,
            account.address()
        ),
        TestName::PublishModule,
        CHECK_ACCOUNT_DATA,
        network_name,
        run_id,
    )?;

    // build module
    let package = emit_step_metrics(
        time_fn!(build_module, account.address()),
        TestName::PublishModule,
        BUILD_MODULE,
        network_name,
        run_id,
    )?;

    // publish module
    let blob = emit_step_metrics(
        time_fn!(publish_module, &client, &mut account, package),
        TestName::PublishModule,
        PUBLISH_MODULE,
        network_name,
        run_id,
    )?;

    // persistently check that API returns correct module package data
    emit_step_metrics(
        time_fn!(
            persistent_check::address_bytes,
            CHECK_MODULE_DATA,
            check_module_data,
            &client,
            account.address(),
            &blob
        ),
        TestName::PublishModule,
        CHECK_MODULE_DATA,
        network_name,
        run_id,
    )?;

    // set message
    emit_step_metrics(
        time_fn!(set_message, &client, &mut account),
        TestName::PublishModule,
        SET_MESSAGE,
        network_name,
        run_id,
    )?;

    // persistently check that the message is correct
    emit_step_metrics(
        time_fn!(
            persistent_check::address,
            CHECK_MESSAGE,
            check_message,
            &client,
            account.address()
        ),
        TestName::PublishModule,
        CHECK_MESSAGE,
        network_name,
        run_id,
    )?;

    Ok(())
}

// Steps

async fn setup(network_name: NetworkName) -> Result<(Client, LocalAccount), TestFailure> {
    // spin up clients
    let client = network_name.get_client();
    let faucet_client = network_name.get_faucet_client();

    // create account
    let account = match create_and_fund_account(&faucet_client, TestName::PublishModule).await {
        Ok(account) => account,
        Err(e) => {
            error!(
                "test: publish_module part: setup ERROR: {}, with error {:?}",
                ERROR_COULD_NOT_FUND_ACCOUNT, e
            );
            return Err(e.into());
        },
    };

    Ok((client, account))
}

async fn check_account_data(client: &Client, account: AccountAddress) -> Result<(), TestFailure> {
    check_balance(TestName::PublishModule, client, account, U64(FUND_AMOUNT)).await?;

    Ok(())
}

async fn build_module(address: AccountAddress) -> Result<BuiltPackage, TestFailure> {
    // get file to compile
    let move_dir = PathBuf::from("./velor-move/move-examples/hello_blockchain");

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
            error!(
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
            error!(
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
            error!(
                "test: publish_module part: publish_module ERROR: {}, with error {:?}",
                ERROR_COULD_NOT_SERIALIZE, e
            );
            return Err(anyhow!(e).into());
        },
    };

    // create payload
    let payload: velor_types::transaction::TransactionPayload =
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
                error!(
                    "test: publish_module part: publish_module ERROR: {}, with error {:?}",
                    ERROR_COULD_NOT_CREATE_AND_SUBMIT_TRANSACTION, e
                );
                return Err(e.into());
            },
        };

    // wait for transaction to finish
    if let Err(e) = client.wait_for_transaction(&pending_txn).await {
        error!(
            "test: publish_module part: publish_module ERROR: {}, with error {:?}",
            ERROR_COULD_NOT_FINISH_TRANSACTION, e
        );
        return Err(e.into());
    };

    // get blob for later comparison
    let blob = match blobs.first() {
        Some(bytecode) => HexEncodedBytes::from(bytecode.clone()),
        None => {
            error!(
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
            error!(
                "test: publish_module part: check_module_data ERROR: {}, with error {:?}",
                ERROR_NO_MODULE, e
            );
            return Err(e.into());
        },
    };
    let actual = &response.inner().bytecode;

    // compare
    if expected != actual {
        error!(
            "test: publish_module part: check_module_data FAIL: {}, expected {:?}, got {:?}",
            FAIL_WRONG_MODULE, expected, actual
        );
        return Err(TestFailure::Fail(FAIL_WRONG_MODULE));
    }

    Ok(())
}

async fn set_message(client: &Client, account: &mut LocalAccount) -> Result<(), TestFailure> {
    // set up message
    let message = match bcs::to_bytes(TEST_MESSAGE) {
        Ok(data) => data,
        Err(e) => {
            error!(
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
                error!(
                    "test: publish_module part: set_message ERROR: {}, with error {:?}",
                    ERROR_COULD_NOT_CREATE_AND_SUBMIT_TRANSACTION, e
                );
                return Err(e.into());
            },
        };

    // wait for transaction to finish
    if let Err(e) = client.wait_for_transaction(&pending_txn).await {
        error!(
            "test: publish_module part: set_message ERROR: {}, with error {:?}",
            ERROR_COULD_NOT_FINISH_TRANSACTION, e
        );
        return Err(e.into());
    };

    Ok(())
}

async fn check_message(client: &Client, address: AccountAddress) -> Result<(), TestFailure> {
    // expected
    let expected = TEST_MESSAGE.to_string();

    // actual
    let actual = match get_message(client, address).await {
        Some(message) => message,
        None => {
            error!(
                "test: publish_module part: check_message ERROR: {}",
                ERROR_NO_MESSAGE
            );
            return Err(anyhow!(ERROR_NO_MESSAGE).into());
        },
    };

    // compare
    if expected != actual {
        error!(
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
