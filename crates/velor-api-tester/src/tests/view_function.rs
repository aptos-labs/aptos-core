// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    consts::FUND_AMOUNT,
    persistent_check,
    strings::{
        CHECK_ACCOUNT_DATA, CHECK_VIEW_ACCOUNT_BALANCE, ERROR_BAD_BALANCE_STRING,
        ERROR_COULD_NOT_FUND_ACCOUNT, ERROR_COULD_NOT_VIEW, ERROR_NO_BALANCE_STRING,
        FAIL_WRONG_BALANCE, SETUP,
    },
    time_fn,
    utils::{
        check_balance, create_and_fund_account, emit_step_metrics, NetworkName, TestFailure,
        TestName,
    },
};
use anyhow::anyhow;
use velor_api_types::{ViewRequest, U64};
use velor_logger::error;
use velor_rest_client::Client;
use velor_sdk::types::LocalAccount;
use velor_types::account_address::AccountAddress;

/// Tests view function use. Checks that:
///  - view function returns correct value
pub async fn test(network_name: NetworkName, run_id: &str) -> Result<(), TestFailure> {
    // setup
    let (client, account) = emit_step_metrics(
        time_fn!(setup, network_name),
        TestName::ViewFunction,
        SETUP,
        network_name,
        run_id,
    )?;

    // check account data persistently
    emit_step_metrics(
        time_fn!(
            persistent_check::address,
            CHECK_ACCOUNT_DATA,
            check_account_data,
            &client,
            account.address()
        ),
        TestName::ViewFunction,
        CHECK_ACCOUNT_DATA,
        network_name,
        run_id,
    )?;

    // check account balance from view function persistently
    emit_step_metrics(
        time_fn!(
            persistent_check::address,
            CHECK_VIEW_ACCOUNT_BALANCE,
            check_view_account_balance,
            &client,
            account.address()
        ),
        TestName::ViewFunction,
        CHECK_VIEW_ACCOUNT_BALANCE,
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
    let account = match create_and_fund_account(&faucet_client, TestName::ViewFunction).await {
        Ok(account) => account,
        Err(e) => {
            error!(
                "test: {} part: {} ERROR: {}, with error {:?}",
                TestName::ViewFunction.to_string(),
                SETUP,
                ERROR_COULD_NOT_FUND_ACCOUNT,
                e
            );
            return Err(e.into());
        },
    };

    Ok((client, account))
}

async fn check_account_data(client: &Client, account: AccountAddress) -> Result<(), TestFailure> {
    check_balance(TestName::ViewFunction, client, account, U64(FUND_AMOUNT)).await?;

    Ok(())
}

async fn check_view_account_balance(
    client: &Client,
    address: AccountAddress,
) -> Result<(), TestFailure> {
    // expected
    let expected = U64(FUND_AMOUNT);

    // actual

    // get client response
    let response = match client
        .view(
            &ViewRequest {
                function: "0x1::coin::balance".parse()?,
                type_arguments: vec!["0x1::velor_coin::VelorCoin".parse()?],
                arguments: vec![serde_json::Value::String(address.to_hex_literal())],
            },
            None,
        )
        .await
    {
        Ok(response) => response,
        Err(e) => {
            error!(
                "test: {} part: {} ERROR: {}, with error {:?}",
                TestName::ViewFunction.to_string(),
                CHECK_VIEW_ACCOUNT_BALANCE,
                ERROR_COULD_NOT_VIEW,
                e
            );
            return Err(e.into());
        },
    };

    // get the string value from the serde_json value
    let value = match response.inner()[0].as_str() {
        Some(value) => value,
        None => {
            error!(
                "test: {} part: {} ERROR: {}, with error {:?}",
                TestName::ViewFunction.to_string(),
                CHECK_VIEW_ACCOUNT_BALANCE,
                ERROR_NO_BALANCE_STRING,
                response.inner()
            );
            return Err(anyhow!(ERROR_NO_BALANCE_STRING).into());
        },
    };

    // parse the string into a U64
    let actual = match value.parse::<u64>() {
        Ok(value) => U64(value),
        Err(e) => {
            error!(
                "test: {} part: {} ERROR: {}, with error {:?}",
                TestName::ViewFunction.to_string(),
                CHECK_VIEW_ACCOUNT_BALANCE,
                ERROR_BAD_BALANCE_STRING,
                e
            );
            return Err(e.into());
        },
    };

    // compare
    if expected != actual {
        error!(
            "test: {} part: {} FAIL: {}, expected {:?}, got {:?}",
            TestName::ViewFunction.to_string(),
            CHECK_VIEW_ACCOUNT_BALANCE,
            FAIL_WRONG_BALANCE,
            expected,
            actual
        );
    }

    Ok(())
}
