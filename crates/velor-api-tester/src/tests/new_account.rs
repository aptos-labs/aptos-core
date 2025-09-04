// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    consts::FUND_AMOUNT,
    persistent_check,
    strings::{
        CHECK_ACCOUNT_BALANCE, CHECK_ACCOUNT_DATA, ERROR_COULD_NOT_CREATE_ACCOUNT,
        ERROR_COULD_NOT_FUND_ACCOUNT, ERROR_NO_ACCOUNT_DATA, FAIL_WRONG_ACCOUNT_DATA, FUND, SETUP,
    },
    time_fn,
    utils::{check_balance, create_account, emit_step_metrics, NetworkName, TestFailure, TestName},
};
use velor_api_types::U64;
use velor_logger::error;
use velor_rest_client::{Account, Client, FaucetClient};
use velor_sdk::types::LocalAccount;
use velor_types::account_address::AccountAddress;

/// Tests new account creation. Checks that:
///   - account data exists
///   - account balance reflects funded amount
pub async fn test(network_name: NetworkName, run_id: &str) -> Result<(), TestFailure> {
    // setup
    let (client, faucet_client, account) = emit_step_metrics(
        time_fn!(setup, network_name),
        TestName::NewAccount,
        SETUP,
        network_name,
        run_id,
    )?;

    // persistently check that API returns correct account data (auth key and sequence number)
    emit_step_metrics(
        time_fn!(
            persistent_check::account,
            CHECK_ACCOUNT_DATA,
            check_account_data,
            &client,
            &account
        ),
        TestName::NewAccount,
        CHECK_ACCOUNT_DATA,
        network_name,
        run_id,
    )?;

    // fund account
    emit_step_metrics(
        time_fn!(fund, &faucet_client, account.address()),
        TestName::NewAccount,
        FUND,
        network_name,
        run_id,
    )?;

    // persistently check that account balance is correct
    emit_step_metrics(
        time_fn!(
            persistent_check::address,
            CHECK_ACCOUNT_BALANCE,
            check_account_balance,
            &client,
            account.address()
        ),
        TestName::NewAccount,
        CHECK_ACCOUNT_BALANCE,
        network_name,
        run_id,
    )?;

    Ok(())
}

// Steps

async fn setup(
    network_name: NetworkName,
) -> Result<(Client, FaucetClient, LocalAccount), TestFailure> {
    // spin up clients
    let client = network_name.get_client();
    let faucet_client = network_name.get_faucet_client();

    // create account
    let account = match create_account(&faucet_client, TestName::NewAccount).await {
        Ok(account) => account,
        Err(e) => {
            error!(
                "test: new_account part: setup ERROR: {}, with error {:?}",
                ERROR_COULD_NOT_CREATE_ACCOUNT, e
            );
            return Err(e.into());
        },
    };

    Ok((client, faucet_client, account))
}

async fn fund(faucet_client: &FaucetClient, address: AccountAddress) -> Result<(), TestFailure> {
    // fund account
    if let Err(e) = faucet_client.fund(address, FUND_AMOUNT).await {
        error!(
            "test: new_account part: fund ERROR: {}, with error {:?}",
            ERROR_COULD_NOT_FUND_ACCOUNT, e
        );
        return Err(e.into());
    }

    Ok(())
}

async fn check_account_data(client: &Client, account: &LocalAccount) -> Result<(), TestFailure> {
    // expected
    let expected = Account {
        authentication_key: account.authentication_key(),
        sequence_number: account.sequence_number(),
    };

    // actual
    let actual = match client.get_account(account.address()).await {
        Ok(response) => response.into_inner(),
        Err(e) => {
            error!(
                "test: new_account part: check_account_data ERROR: {}, with error {:?}",
                ERROR_NO_ACCOUNT_DATA, e
            );
            return Err(e.into());
        },
    };

    // compare
    if expected != actual {
        error!(
            "test: new_account part: check_account_data FAIL: {}, expected {:?}, got {:?}",
            FAIL_WRONG_ACCOUNT_DATA, expected, actual
        );
        return Err(TestFailure::Fail(FAIL_WRONG_ACCOUNT_DATA));
    }

    Ok(())
}

async fn check_account_balance(
    client: &Client,
    address: AccountAddress,
) -> Result<(), TestFailure> {
    check_balance(TestName::NewAccount, client, address, U64(FUND_AMOUNT)).await
}
