// Copyright © Aptos Foundation

use crate::utils::{get_client, get_faucet_client, NetworkName, TestFailure};
use anyhow::anyhow;
use aptos_api_types::U64;
use aptos_logger::info;
use aptos_rest_client::{Account, Client, FaucetClient};
use aptos_sdk::types::LocalAccount;
use futures::Future;
use std::time::Duration;
use tokio::time::Instant;

static FAIL_WRONG_ACCOUNT_DATA: &str = "wrong account data";
static FAIL_WRONG_BALANCE: &str = "wrong balance";
static ERROR_COULD_NOT_CREATE_ACCOUNT: &str = "faucet client failed to create account";
static ERROR_COULD_NOT_CHECK: &str = "persistency check never started";
static ERROR_NO_ACCOUNT_DATA: &str = "can't find account data";

/// Tests new account creation. Checks that:
///   - account data exists
///   - account balance reflects funded amount
pub async fn test_newaccount(network_name: NetworkName) -> Result<(), TestFailure> {
    // setup
    let (client, faucet_client, account) = setup(network_name).await?;

    // check account data persistently
    persistent_check(check_account_data, &client, &account).await?;

    // fund account
    faucet_client.fund(account.address(), 100_000_000).await?;

    // check account balance persistently
    persistent_check(check_account_balance, &client, &account).await?;

    Ok(())
}

async fn persistent_check<'a, 'b, F, Fut>(
    f: F,
    client: &'a Client,
    account: &'b LocalAccount,
) -> Result<(), TestFailure>
where
    F: Fn(&'a Client, &'b LocalAccount) -> Fut,
    Fut: Future<Output = Result<(), TestFailure>>,
{
    // set a default error in case checks never start
    let mut result: Result<(), TestFailure> = Err(anyhow!(ERROR_COULD_NOT_CHECK).into());
    let timer = Instant::now();

    // try to get a good result for 30 seconds
    while Instant::now().duration_since(timer) < Duration::from_secs(30) {
        result = f(client, account).await;
        if result.is_ok() {
            break;
        }
    }

    // return last failure if no good result occurs
    result
}

async fn setup(
    network_name: NetworkName,
) -> Result<(Client, FaucetClient, LocalAccount), TestFailure> {
    // spin up clients
    let client = get_client(network_name);
    let faucet_client = get_faucet_client(network_name);

    // create account
    let account = LocalAccount::generate(&mut rand::rngs::OsRng);
    if let Err(e) = faucet_client.create_account(account.address()).await {
        info!("test: new_account part: setup ERROR: {}, with error {:?}", ERROR_COULD_NOT_CREATE_ACCOUNT, e);
        return Err(e.into());
    };

    Ok((client, faucet_client, account))
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

async fn check_account_balance(client: &Client, account: &LocalAccount) -> Result<(), TestFailure> {
    // expected
    let expected = U64(100_000_000);

    // actual
    let actual = client
        .get_account_balance(account.address())
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
