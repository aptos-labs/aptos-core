// Copyright © Aptos Foundation

use crate::{
    fail_message::{
        ERROR_COULD_NOT_CREATE_ACCOUNT, ERROR_COULD_NOT_FUND_ACCOUNT, ERROR_NO_ACCOUNT_DATA,
        ERROR_NO_BALANCE, FAIL_WRONG_ACCOUNT_DATA, FAIL_WRONG_BALANCE,
    },
    persistent_check,
    utils::{create_account, get_client, get_faucet_client, NetworkName, TestFailure},
};
use aptos_api_types::U64;
use aptos_logger::info;
use aptos_rest_client::{Account, Client, FaucetClient};
use aptos_sdk::types::LocalAccount;
use aptos_types::account_address::AccountAddress;

static FUND_AMOUNT: u64 = 1_000_000;

/// Tests new account creation. Checks that:
///   - account data exists
///   - account balance reflects funded amount
pub async fn test(network_name: NetworkName) -> Result<(), TestFailure> {
    // setup
    let (client, faucet_client, account) = setup(network_name).await?;

    // check account data persistently
    persistent_check::account("check_account_data", check_account_data, &client, &account).await?;

    // fund account
    fund(&faucet_client, account.address()).await?;

    // check account balance persistently
    persistent_check::address(
        "check_account_balance",
        check_account_balance,
        &client,
        account.address(),
    )
    .await?;

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
    // expected
    let expected = U64(FUND_AMOUNT);

    // actual
    let actual = match client.get_account_balance(address).await {
        Ok(response) => response.into_inner().coin.value,
        Err(e) => {
            info!(
                "test: new_account part: check_account_balance ERROR: {}, with error {:?}",
                ERROR_NO_BALANCE, e
            );
            return Err(e.into());
        },
    };

    // compare
    if expected != actual {
        info!(
            "test: new_account part: check_account_balance FAIL: {}, expected {:?}, got {:?}",
            FAIL_WRONG_BALANCE, expected, actual
        );
        return Err(TestFailure::Fail(FAIL_WRONG_BALANCE));
    }

    Ok(())
}
