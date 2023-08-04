// Copyright © Aptos Foundation

use crate::{
    fail_message::{
        ERROR_COULD_NOT_CREATE_ACCOUNT, ERROR_COULD_NOT_CREATE_TRANSACTION,
        ERROR_COULD_NOT_FINISH_TRANSACTION, ERROR_COULD_NOT_FUND_ACCOUNT, ERROR_NO_BALANCE,
        ERROR_NO_VERSION, FAIL_WRONG_BALANCE, FAIL_WRONG_BALANCE_AT_VERSION,
    },
    persistent_check,
    utils::{
        create_account, create_and_fund_account, get_client, get_faucet_client, NetworkName,
        TestFailure,
    },
};
use anyhow::{anyhow, Result};
use aptos_api_types::U64;
use aptos_logger::info;
use aptos_rest_client::Client;
use aptos_sdk::{coin_client::CoinClient, types::LocalAccount};
use aptos_types::account_address::AccountAddress;

static TRANSFER_AMOUNT: u64 = 1_000;

/// Tests coin transfer. Checks that:
///   - receiver balance reflects transferred amount
///   - receiver balance shows correct amount at the previous version
pub async fn test(network_name: NetworkName) -> Result<(), TestFailure> {
    // setup
    let (client, mut account, receiver) = setup(network_name).await?;
    let coin_client = CoinClient::new(&client);

    // transfer coins to the receiver
    let version = transfer_coins(&client, &coin_client, &mut account, receiver).await?;

    // check receiver balance persistently
    persistent_check::address(
        "check_account_balance",
        check_account_balance,
        &client,
        receiver,
    )
    .await?;

    // check receiver balance at previous version persistently
    persistent_check::address_version(
        "check_account_balance_at_version",
        check_account_balance_at_version,
        &client,
        receiver,
        version,
    )
    .await?;

    Ok(())
}

// Steps

async fn setup(
    network_name: NetworkName,
) -> Result<(Client, LocalAccount, AccountAddress), TestFailure> {
    // spin up clients
    let client = get_client(network_name);
    let faucet_client = get_faucet_client(network_name);

    // create account
    let account = match create_and_fund_account(&faucet_client).await {
        Ok(account) => account,
        Err(e) => {
            info!(
                "test: coin_transfer part: setup ERROR: {}, with error {:?}",
                ERROR_COULD_NOT_FUND_ACCOUNT, e
            );
            return Err(e.into());
        },
    };

    // create receiver
    let receiver = match create_account(&faucet_client).await {
        Ok(account) => account.address(),
        Err(e) => {
            info!(
                "test: coin_transfer part: setup ERROR: {}, with error {:?}",
                ERROR_COULD_NOT_CREATE_ACCOUNT, e
            );
            return Err(e.into());
        },
    };

    Ok((client, account, receiver))
}

async fn transfer_coins(
    client: &Client,
    coin_client: &CoinClient<'_>,
    account: &mut LocalAccount,
    receiver: AccountAddress,
) -> Result<u64, TestFailure> {
    // create transaction
    let pending_txn = match coin_client
        .transfer(account, receiver, TRANSFER_AMOUNT, None)
        .await
    {
        Ok(pending_txn) => pending_txn,
        Err(e) => {
            info!(
                "test: coin_transfer part: transfer_coins ERROR: {}, with error {:?}",
                ERROR_COULD_NOT_CREATE_TRANSACTION, e
            );
            return Err(e.into());
        },
    };

    // wait and get version
    let response = match client.wait_for_transaction(&pending_txn).await {
        Ok(response) => response,
        Err(e) => {
            info!(
                "test: coin_transfer part: transfer_coins ERROR: {}, with error {:?}",
                ERROR_COULD_NOT_FINISH_TRANSACTION, e
            );
            return Err(e.into());
        },
    };

    let version = match response.inner().version() {
        Some(version) => version,
        None => {
            info!(
                "test: coin_transfer part: transfer_coins ERROR: {}",
                ERROR_NO_VERSION
            );
            return Err(anyhow!(ERROR_NO_VERSION).into());
        },
    };

    // return version
    Ok(version)
}

async fn check_account_balance(
    client: &Client,
    address: AccountAddress,
) -> Result<(), TestFailure> {
    // expected
    let expected = U64(TRANSFER_AMOUNT);

    // actual
    let actual = match client.get_account_balance(address).await {
        Ok(response) => response.into_inner().coin.value,
        Err(e) => {
            info!(
                "test: coin_transfer part: check_account_balance ERROR: {}, with error {:?}",
                ERROR_NO_BALANCE, e
            );
            return Err(e.into());
        },
    };

    // compare
    if expected != actual {
        info!(
            "test: coin_transfer part: check_account_balance FAIL: {}, expected {:?}, got {:?}",
            FAIL_WRONG_BALANCE, expected, actual
        );
        return Err(TestFailure::Fail(FAIL_WRONG_BALANCE));
    }

    Ok(())
}

async fn check_account_balance_at_version(
    client: &Client,
    address: AccountAddress,
    transaction_version: u64,
) -> Result<(), TestFailure> {
    // expected
    let expected = U64(0);

    // actual
    let actual = match client
        .get_account_balance_at_version(address, transaction_version - 1)
        .await
    {
        Ok(response) => response.into_inner().coin.value,
        Err(e) => {
            info!(
                "test: coin_transfer part: check_account_balance_at_version ERROR: {}, with error {:?}",
                ERROR_NO_BALANCE, e
            );
            return Err(e.into());
        },
    };

    // compare
    if expected != actual {
        info!(
            "test: coin_transfer part: check_account_balance_at_version FAIL: {}, expected {:?}, got {:?}",
            FAIL_WRONG_BALANCE_AT_VERSION, expected, actual
        );
        return Err(TestFailure::Fail(FAIL_WRONG_BALANCE_AT_VERSION));
    }

    Ok(())
}
