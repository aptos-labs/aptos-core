// Copyright © Aptos Foundation

use crate::{
    consts::FUND_AMOUNT,
    fail_message::{
        ERROR_COULD_NOT_CREATE_ACCOUNT, ERROR_COULD_NOT_FUND_ACCOUNT, ERROR_NO_ACCOUNT_DATA,
        FAIL_WRONG_ACCOUNT_DATA,
    },
    persistent_check, time_fn,
    utils::{
        check_balance, create_account, emit_step_metrics, get_client, get_faucet_client,
        NetworkName, TestFailure, TestName,
    },
};
use aptos_api_types::U64;
use aptos_logger::info;
use aptos_rest_client::{Account, Client, FaucetClient};
use aptos_sdk::types::LocalAccount;
use aptos_types::account_address::AccountAddress;

/// Tests new account creation. Checks that:
///   - account data exists
///   - account balance reflects funded amount
pub async fn test(network_name: NetworkName, run_id: &str) -> Result<(), TestFailure> {
    // setup
    let (client, faucet_client, account) = emit_step_metrics(
        time_fn!(setup, network_name),
        TestName::NewAccount,
        "setup",
        network_name,
        run_id,
    )?;

    // check account data persistently
    emit_step_metrics(
        time_fn!(
            persistent_check::account,
            "check_account_data",
            check_account_data,
            &client,
            &account
        ),
        TestName::NewAccount,
        "check_account_data",
        network_name,
        run_id,
    )?;

    // fund account
    emit_step_metrics(
        time_fn!(fund, &faucet_client, account.address()),
        TestName::NewAccount,
        "fund",
        network_name,
        run_id,
    )?;

    // check account balance persistently
    emit_step_metrics(
        time_fn!(
            persistent_check::address,
            "check_account_balance",
            check_account_balance,
            &client,
            account.address()
        ),
        TestName::NewAccount,
        "check_account_balance",
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
    let client = get_client(network_name);
    let faucet_client = get_faucet_client(network_name);

    // create account
    let account = match create_account(&faucet_client).await {
        Ok(account) => account,
        Err(e) => {
            info!(
                "test: new_account part: setup ERROR: {}, with error {:?}",
                ERROR_COULD_NOT_CREATE_ACCOUNT, e
            );
            return Err(e.into());
        },
    };
    info!(
        "test: new_account part: setup creating account: {}",
        account.address()
    );

    Ok((client, faucet_client, account))
}

async fn fund(faucet_client: &FaucetClient, address: AccountAddress) -> Result<(), TestFailure> {
    // fund account
    if let Err(e) = faucet_client.fund(address, FUND_AMOUNT).await {
        info!(
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
            info!(
                "test: new_account part: check_account_data ERROR: {}, with error {:?}",
                ERROR_NO_ACCOUNT_DATA, e
            );
            return Err(e.into());
        },
    };

    // compare
    if expected != actual {
        info!(
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
