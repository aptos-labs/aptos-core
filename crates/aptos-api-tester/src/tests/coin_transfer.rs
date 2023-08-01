// Copyright © Aptos Foundation

use std::time::Duration;

use crate::utils::{get_client, get_faucet_client, NetworkName, TestFailure};
use anyhow::{anyhow, Result};
use aptos_api_types::U64;
use aptos_logger::info;
use aptos_rest_client::Client;
use aptos_sdk::{coin_client::CoinClient, types::LocalAccount};
use aptos_types::account_address::AccountAddress;
use futures::Future;
use tokio::time::Instant;

static ERROR_MODULE_INTERACTION: &str = "module interaction isn't reflected";
static ERROR_NO_VERSION: &str = "transaction did not return version";

static FAIL_WRONG_BALANCE: &str = "wrong balance";
static ERROR_COULD_NOT_CHECK: &str = "persistency check never started";
static ERROR_COULD_NOT_CREATE_ACCOUNT: &str = "faucet client failed to create account";

/// Tests coin transfer. Checks that:
///   - receiver balance reflects transferred amount
///   - receiver balance shows correct amount at the previous version
pub async fn test_cointransfer(network_name: NetworkName) -> Result<(), TestFailure> {
    // setup
    let (client, mut account, receiver) = setup(network_name).await?;
    let coin_client = CoinClient::new(&client);

    // transfer coins to the receiver
    let version = transfer_coins(&client, &coin_client, &mut account, receiver).await?;

    // check receiver balance persistently
    persistent_check(check_account_balance, &client, receiver).await?;

    // check receiver balance at previous version persistently
    persistent_check_2(check_account_balance_at_version, &client, receiver, version).await?;

    Ok(())
}

async fn persistent_check<'a, F, Fut>(
    f: F,
    client: &'a Client,
    address: AccountAddress,
) -> Result<(), TestFailure>
where
    F: Fn(&'a Client, AccountAddress) -> Fut,
    Fut: Future<Output = Result<(), TestFailure>>,
{
    // set a default error in case checks never start
    let mut result: Result<(), TestFailure> = Err(anyhow!(ERROR_COULD_NOT_CHECK).into());
    let timer = Instant::now();

    // try to get a good result for 30 seconds
    while Instant::now().duration_since(timer) < Duration::from_secs(30) {
        result = f(client, address).await;
        if result.is_ok() {
            break;
        }
    }

    // return last failure if no good result occurs
    result
}

async fn persistent_check_2<'a, F, Fut>(
    f: F,
    client: &'a Client,
    address: AccountAddress,
    version: u64,
) -> Result<(), TestFailure>
where
    F: Fn(&'a Client, AccountAddress, u64) -> Fut,
    Fut: Future<Output = Result<(), TestFailure>>,
{
    // set a default error in case checks never start
    let mut result: Result<(), TestFailure> = Err(anyhow!(ERROR_COULD_NOT_CHECK).into());
    let timer = Instant::now();

    // try to get a good result for 30 seconds
    while Instant::now().duration_since(timer) < Duration::from_secs(30) {
        result = f(client, address, version).await;
        if result.is_ok() {
            break;
        }
    }

    // return last failure if no good result occurs
    result
}

async fn setup(
    network_name: NetworkName,
) -> Result<(Client, LocalAccount, AccountAddress), TestFailure> {
    // spin up clients
    let client = get_client(network_name);
    let faucet_client = get_faucet_client(network_name);

    // create account
    let account = LocalAccount::generate(&mut rand::rngs::OsRng);
    if let Err(e) = faucet_client.create_account(account.address()).await {
        info!(
            "test: new_account part: setup ERROR: {}, with error {:?}",
            ERROR_COULD_NOT_CREATE_ACCOUNT, e
        );
        return Err(e.into());
    };
    faucet_client.fund(account.address(), 100_000_000).await?;

    // create receiver
    let receiver = LocalAccount::generate(&mut rand::rngs::OsRng);
    if let Err(e) = faucet_client.create_account(receiver.address()).await {
        info!(
            "test: new_account part: setup ERROR: {}, with error {:?}",
            ERROR_COULD_NOT_CREATE_ACCOUNT, e
        );
        return Err(e.into());
    };

    Ok((client, account, receiver.address()))
}

async fn transfer_coins(
    client: &Client,
    coin_client: &CoinClient<'_>,
    account: &mut LocalAccount,
    receiver: AccountAddress,
) -> Result<u64, TestFailure> {
    let pending_txn = coin_client.transfer(account, receiver, 1_000, None).await?;
    let version = match client.wait_for_transaction(&pending_txn).await?.inner().version() {
        Some(version) => version,
        _ => {
            info!("error: {}", ERROR_MODULE_INTERACTION);
            return Err(TestFailure::Error(anyhow!(ERROR_NO_VERSION)));
        },
    };

    Ok(version)
}

async fn check_account_balance(
    client: &Client,
    address: AccountAddress,
) -> Result<(), TestFailure> {
    // expected
    let expected = U64(1_000);

    // actual
    let actual = client
        .get_account_balance(address)
        .await?
        .into_inner()
        .coin
        .value;

    // compare
    if expected != actual {
        info!(
            "fail: {}, expected {:?}, got {:?}",
            FAIL_WRONG_BALANCE, expected, actual
        );
        return Err(TestFailure::Fail(FAIL_WRONG_BALANCE));
    }

    Ok(())
}

async fn check_account_balance_at_version(
    client: &Client,
    address: AccountAddress,
    version: u64,
) -> Result<(), TestFailure> {
    // expected
    let expected = U64(0);

    // actual
    let actual = client
        .get_account_balance_at_version(address, version - 1)
        .await?
        .into_inner()
        .coin
        .value;

    // compare
    if expected != actual {
        info!(
            "fail: {}, expected {:?}, got {:?}",
            FAIL_WRONG_BALANCE, expected, actual
        );
        return Err(TestFailure::Fail(FAIL_WRONG_BALANCE));
    }

    Ok(())
}
