// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use std::future::Future;

use anyhow::{anyhow, ensure, Context, Result};
use aptos_api_types::U64;
use aptos_rest_client::{Account, Client, FaucetClient};
use aptos_sdk::coin_client::CoinClient;
use aptos_sdk::types::LocalAccount;
use aptos_types::account_address::AccountAddress;
use once_cell::sync::Lazy;
use url::Url;

// global parameters (todo: make into clap)
static HAS_FAUCET_ACCESS: bool = true;

// network urls
static DEVNET_NODE_URL: Lazy<Url> =
    Lazy::new(|| Url::parse("https://fullnode.devnet.aptoslabs.com").unwrap());
static DEVNET_FAUCET_URL: Lazy<Url> =
    Lazy::new(|| Url::parse("https://faucet.devnet.aptoslabs.com").unwrap());
static TESTNET_NODE_URL: Lazy<Url> =
    Lazy::new(|| Url::parse("https://fullnode.testnet.aptoslabs.com").unwrap());
static TESTNET_FAUCET_URL: Lazy<Url> =
    Lazy::new(|| Url::parse("https://faucet.testnet.aptoslabs.com").unwrap());

// static accounts to use
static TEST_ACCOUNT_1: Lazy<AccountAddress> = Lazy::new(|| {
    AccountAddress::from_hex_literal(
        "0xf6cee5c359839a321a08af6b8cfe4a6e439db46f4942e21b6845aa57d770efdb",
    )
    .unwrap()
});

// return values
static SKIP_NO_FAUCET_ACCESS: &str = "This test requires faucet access.";
static ERROR_CLIENT_RESPONSE: &str = "Client responded with error.";
static ERROR_FAUCET_FUND: &str = "Funding from faucet failed.";
static ERROR_COIN_TRANSFER: &str = "Coin transfer failed.";
static FAIL_WRONG_AUTH_KEY: &str = "Returned wrong authentication key.";
static FAIL_WRONG_SEQ_NUMBER: &str = "Returned wrong sequence number.";
static FAIL_WRONG_BALANCE: &str = "Returned wrong balance.";
static FAIL_WRONG_BALANCE_AT_VERSION: &str = "Returned wrong balance at the given version.";
static SUCCESS: &str = "success";

#[derive(Debug)]
enum TestResult {
    Success,
    Fail(&'static str),
    Error(anyhow::Error),
}

/// Calls get_account_balance_at_version on a newly created account on devnet. Requires faucet.
async fn probe_getaccountbalanceatversion_1() -> Result<&'static str> {
    // check faucet access
    if !HAS_FAUCET_ACCESS {
        return Ok(SKIP_NO_FAUCET_ACCESS);
    }

    // create the rest client
    let client = Client::new(DEVNET_NODE_URL.clone());
    let faucet_client = FaucetClient::new(DEVNET_FAUCET_URL.clone(), DEVNET_NODE_URL.clone());
    let coin_client = CoinClient::new(&client);

    // create and fund an account
    let mut giray = LocalAccount::generate(&mut rand::rngs::OsRng);
    faucet_client
        .fund(giray.address(), 100_000_000)
        .await
        .context(ERROR_FAUCET_FUND)?;

    // create and fund second account
    let giray2 = LocalAccount::generate(&mut rand::rngs::OsRng);
    faucet_client
        .fund(giray2.address(), 100_000_000)
        .await
        .context(ERROR_FAUCET_FUND)?;

    // transfer coins from first account to the second
    let txn_hash = coin_client
        .transfer(&mut giray, giray2.address(), 1_000, None)
        .await
        .context(ERROR_COIN_TRANSFER)?;
    let response = client
        .wait_for_transaction(&txn_hash)
        .await
        .context(ERROR_COIN_TRANSFER)?;

    // get transaction version number
    let version = response.inner().version().context(ERROR_COIN_TRANSFER)?;

    // ask for account balance with a lower version number
    let response = client
        .get_account_balance_at_version(giray2.address(), version - 1)
        .await
        .context(ERROR_CLIENT_RESPONSE)?;

    // check balance before transaction
    let expected_balance = U64(100_000_000);
    let actual_balance = response.inner().coin.value;

    ensure!(
        expected_balance == actual_balance,
        "{} expected {}, got {}",
        FAIL_WRONG_BALANCE_AT_VERSION,
        expected_balance,
        actual_balance,
    );

    // ask for account balance with the given version number
    let response = client
        .get_account_balance_at_version(giray2.address(), version)
        .await
        .context(ERROR_CLIENT_RESPONSE)?;

    // check balance right after transaction
    let expected_balance = U64(100_001_000);
    let actual_balance = response.inner().coin.value;

    ensure!(
        expected_balance == actual_balance,
        "{} expected {}, got {}",
        FAIL_WRONG_BALANCE_AT_VERSION,
        expected_balance,
        actual_balance,
    );

    // ask for account balance with a higher version number
    let response = client
        .get_account_balance_at_version(giray2.address(), version + 1)
        .await
        .context(ERROR_CLIENT_RESPONSE)?;

    // check balance long after transaction
    let expected_balance = U64(100_001_000);
    let actual_balance = response.inner().coin.value;

    ensure!(
        expected_balance == actual_balance,
        "{} expected {}, got {}",
        FAIL_WRONG_BALANCE_AT_VERSION,
        expected_balance,
        actual_balance,
    );

    Ok(SUCCESS)
}

async fn handle_result<Fut: Future<Output = TestResult>>(fut: Fut) -> TestResult {
    let result = fut.await;
    println!("{:?}", result);

    result
}

// Tests that the account data for a newly created account is correct.
async fn test_accountdata(client: &Client, account: &LocalAccount) -> TestResult {
    // ask for account data
    let response = match client.get_account(account.address()).await {
        Ok(response) => response,
        Err(e) => return TestResult::Error(e.into()),
    };

    // check account data
    let expected_account = Account {
        authentication_key: account.authentication_key(),
        sequence_number: account.sequence_number(),
    };
    let actual_account = response.inner();

    if !(expected_account.authentication_key == actual_account.authentication_key) {
        return TestResult::Fail(FAIL_WRONG_AUTH_KEY);
    }
    if !(expected_account.sequence_number == actual_account.sequence_number) {
        return TestResult::Fail(FAIL_WRONG_SEQ_NUMBER);
    }

    TestResult::Success
}

// Tests that the balance of an account is correct.
async fn test_accountbalance(
    client: &Client,
    account: &LocalAccount,
    expected_balance: u64,
) -> TestResult {
    // ask for account balance
    let response = match client.get_account_balance(account.address()).await {
        Ok(response) => response,
        Err(e) => return TestResult::Error(e.into()),
    };

    // check balance
    let expected_balance = U64(expected_balance);
    let actual_balance = response.inner().coin.value;

    if !(expected_balance == actual_balance) {
        return TestResult::Fail(FAIL_WRONG_BALANCE);
    }

    TestResult::Success
}

async fn testnet_1() -> Result<()> {
    // create clients
    let client: Client = Client::new(TESTNET_NODE_URL.clone());
    let faucet_client = FaucetClient::new(TESTNET_FAUCET_URL.clone(), TESTNET_NODE_URL.clone());

    // Step 1: Test new account creation
    let giray = LocalAccount::generate(&mut rand::rngs::OsRng);
    faucet_client.fund(giray.address(), 100_000_000).await?;

    // this test is critical to pass for the next tests
    let result = handle_result(test_accountdata(&client, &giray)).await;
    match result {
        TestResult::Success => {},
        _ => return Err(anyhow!("returning early because account creation failed")),
    }

    // Step 2: Test account balance
    handle_result(test_accountbalance(&client, &giray, 100_000_000)).await;

    // Step 3: Test coin transfer

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("{:?}", probe_getaccountbalanceatversion_1().await);
    let _ = testnet_1().await;

    Ok(())
}
