// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use anyhow::{Context, Result};
use aptos_api_types::U64;
use aptos_rest_client::{Client, FaucetClient};
use aptos_sdk::coin_client::CoinClient;
use aptos_sdk::types::LocalAccount;
use aptos_types::account_address::AccountAddress;
use aptos_types::transaction::authenticator::AuthenticationKey;
use once_cell::sync::Lazy;
use rand::Rng;
use std::str::FromStr;
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

// static accounts to use
static TEST_ACCOUNT_1: Lazy<AccountAddress> = Lazy::new(|| {
    AccountAddress::from_hex_literal(
        "0x7fed760c508a8885e1438c90f81dea6940ad53b05527fadddd3851f53342d7c4",
    )
    .unwrap()
});
static TEST_ACCOUNT_2: Lazy<AccountAddress> = Lazy::new(|| {
    AccountAddress::from_hex_literal(
        "0xf6cee5c359839a321a08af6b8cfe4a6e439db46f4942e21b6845aa57d770efdb",
    )
    .unwrap()
});

// return values
static SKIP_NO_FAUCET_ACCESS: &str = "This test requires faucet access.";
static ERROR_CLIENT_RESPONSE: &str = "Client responded with error.";
static ERROR_FAUCET_FUND: &str = "Funding from faucet failed.";
static ERROR_OTHER: &str = "Something went wrong.";
static ERROR_COIN_TRANSFER: &str = "Coin transfer failed.";
static FAIL_WRONG_AUTH_KEY: &str = "Returned wrong authentication key.";
static FAIL_WRONG_SEQ_NUMBER: &str = "Returned wrong sequence number.";
static FAIL_WRONG_BALANCE: &str = "Returned wrong balance.";
static FAIL_WRONG_BALANCE_AT_VERSION: &str = "Returned wrong balance at the given version.";
static SUCCESS: &str = "success";

// Calls get_account on a newly created account. Requires faucet.
async fn probe_getaccount_1() -> Result<&'static str> {
    // check faucet access
    if !HAS_FAUCET_ACCESS {
        return Ok(SKIP_NO_FAUCET_ACCESS);
    }

    // create the rest client
    let client = Client::new(DEVNET_NODE_URL.clone());
    let faucet_client = FaucetClient::new(DEVNET_FAUCET_URL.clone(), DEVNET_NODE_URL.clone());

    // create and fund an account
    let giray = LocalAccount::generate(&mut rand::rngs::OsRng);
    faucet_client
        .fund(giray.address(), 100_000_000)
        .await
        .context(ERROR_FAUCET_FUND)?;

    // ask for account data
    let response = client
        .get_account(giray.address())
        .await
        .context(ERROR_CLIENT_RESPONSE)?;
    let response = response.inner();

    // check authentication key
    let expected_auth_key = giray.authentication_key();
    let actual_auth_key = response.authentication_key;
    if !(actual_auth_key == expected_auth_key) {
        return Ok(FAIL_WRONG_AUTH_KEY);
    }

    // check sequence number
    let expected_seq_num = giray.sequence_number();
    let actual_seq_num = response.sequence_number;
    if !(actual_seq_num == expected_seq_num) {
        return Ok(FAIL_WRONG_SEQ_NUMBER);
    }

    Ok(SUCCESS)
}

// Calls get_account on a static account.
async fn probe_getaccount_2() -> Result<&'static str> {
    // create the rest client
    let client = Client::new(TESTNET_NODE_URL.clone());

    // ask for account data
    let response = client
        .get_account(*TEST_ACCOUNT_1)
        .await
        .context(ERROR_CLIENT_RESPONSE)?;
    let response = response.inner();

    // check authentication key
    let expected_auth_key = AuthenticationKey::from_str(
        "7fed760c508a8885e1438c90f81dea6940ad53b05527fadddd3851f53342d7c4",
    )
    .context(ERROR_OTHER)?;
    let actual_auth_key = response.authentication_key;
    if !(actual_auth_key == expected_auth_key) {
        return Ok(FAIL_WRONG_AUTH_KEY);
    }

    // check sequence number
    let expected_seq_num = 1;
    let actual_seq_num = response.sequence_number;
    if !(actual_seq_num == expected_seq_num) {
        return Ok(FAIL_WRONG_SEQ_NUMBER);
    }

    Ok(SUCCESS)
}

// Calls get_account_balance on a newly created account. Requires faucet.
async fn probe_getaccountbalance_1() -> Result<&'static str> {
    // check faucet access
    if !HAS_FAUCET_ACCESS {
        return Ok(SKIP_NO_FAUCET_ACCESS);
    }

    // create the rest client
    let client = Client::new(DEVNET_NODE_URL.clone());
    let faucet_client = FaucetClient::new(DEVNET_FAUCET_URL.clone(), DEVNET_NODE_URL.clone());

    // create and fund an account
    let giray = LocalAccount::generate(&mut rand::rngs::OsRng);
    faucet_client
        .fund(giray.address(), 100_000_000)
        .await
        .context(ERROR_FAUCET_FUND)?;

    // fund the account further with some random amount
    let random_number: u64 = rand::thread_rng().gen_range(50_000_000, 100_000_000);
    faucet_client.fund(giray.address(), random_number).await?;

    // ask for account balance
    let response = client
        .get_account_balance(giray.address())
        .await
        .context(ERROR_CLIENT_RESPONSE)?;
    let response = response.inner();

    // check balance
    let expected_balance = U64(100_000_000 + random_number);
    let actual_balance = response.coin.value;
    if !(actual_balance == expected_balance) {
        return Ok(FAIL_WRONG_BALANCE);
    }

    Ok(SUCCESS)
}

// Calls get_account_balance on a static account.
async fn probe_getaccountbalance_2() -> Result<&'static str> {
    // create the rest client
    let client = Client::new(TESTNET_NODE_URL.clone());

    // ask for account balance
    let response = client
        .get_account_balance(*TEST_ACCOUNT_1)
        .await
        .context(ERROR_CLIENT_RESPONSE)?;
    let response = response.inner();

    // check balance
    let expected_balance = U64(176_767_177);
    let actual_balance = response.coin.value;
    if !(actual_balance == expected_balance) {
        return Ok(FAIL_WRONG_BALANCE);
    }

    Ok(SUCCESS)
}

// Calls get_account_balance_at_version on a newly created account. Requires faucet.
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
    let response = response.inner();

    // check balance before transaction
    let expected_balance = U64(100_000_000);
    let actual_balance = response.coin.value;
    if !(actual_balance == expected_balance) {
        return Ok(FAIL_WRONG_BALANCE_AT_VERSION);
    }

    // ask for account balance with the given version number
    let response = client
        .get_account_balance_at_version(giray2.address(), version)
        .await
        .context(ERROR_CLIENT_RESPONSE)?;
    let response = response.inner();

    // check balance right after transaction
    let expected_balance = U64(100_001_000);
    let actual_balance = response.coin.value;
    if !(actual_balance == expected_balance) {
        return Ok(FAIL_WRONG_BALANCE_AT_VERSION);
    }

<<<<<<< HEAD
    // use the faucet to ensure that there exists a higher version number
    faucet_client
        .fund(giray.address(), 100_000_000)
        .await
        .context(ERROR_FAUCET_FUND)?;

=======
>>>>>>> 84b40d670f (Add tests for get_account_balance_at_version)
    // ask for account balance with a higher version number
    let response = client
        .get_account_balance_at_version(giray2.address(), version + 1)
        .await
        .context(ERROR_CLIENT_RESPONSE)?;
    let response = response.inner();

    // check balance long after transaction
    let expected_balance = U64(100_001_000);
    let actual_balance = response.coin.value;
    if !(actual_balance == expected_balance) {
        return Ok(FAIL_WRONG_BALANCE_AT_VERSION);
    }

    Ok(SUCCESS)
}

// Calls get_account_balance_at_version on a static account.
async fn probe_getaccountbalanceatversion_2() -> Result<&'static str> {
    // create the rest client
    let client = Client::new(TESTNET_NODE_URL.clone());

    // ask for account balance before funding
    let response = client
        .get_account_balance_at_version(*TEST_ACCOUNT_1, 564_589_719)
        .await
        .context(ERROR_CLIENT_RESPONSE)?;
    let response = response.inner();

    // check balance before funding
    let expected_balance = U64(200_000_000);
    let actual_balance = response.coin.value;
    if !(actual_balance == expected_balance) {
        return Ok(FAIL_WRONG_BALANCE_AT_VERSION);
    }

    // ask for account balance right after funding
    let response = client
        .get_account_balance_at_version(*TEST_ACCOUNT_1, 564_589_720)
        .await
        .context(ERROR_CLIENT_RESPONSE)?;
    let response = response.inner();

    // check balance right after funding
    let expected_balance = U64(300_000_000);
    let actual_balance = response.coin.value;
    if !(actual_balance == expected_balance) {
        return Ok(FAIL_WRONG_BALANCE_AT_VERSION);
    }

    // ask for account balance long after funding
    let response = client
        .get_account_balance_at_version(*TEST_ACCOUNT_1, 564_589_721)
        .await
        .context(ERROR_CLIENT_RESPONSE)?;
    let response = response.inner();

    // check balance right after funding
    let expected_balance = U64(300_000_000);
    let actual_balance = response.coin.value;
    if !(actual_balance == expected_balance) {
        return Ok(FAIL_WRONG_BALANCE_AT_VERSION);
    }

    // ask for account balance right after transfer
    let response = client
        .get_account_balance_at_version(*TEST_ACCOUNT_1, 564_591_130)
        .await
        .context(ERROR_CLIENT_RESPONSE)?;
    let response = response.inner();

    // check balance right after funding
    let expected_balance = U64(176_767_177);
    let actual_balance = response.coin.value;
    if !(actual_balance == expected_balance) {
        return Ok(FAIL_WRONG_BALANCE_AT_VERSION);
    }

    Ok(SUCCESS)
}

#[tokio::main]
async fn main() -> Result<()> {
    match probe_getaccount_1().await {
        Ok(result) => println!("{}", result),
        Err(e) => println!("{}", e),
    }
    match probe_getaccount_2().await {
        Ok(result) => println!("{}", result),
        Err(e) => println!("{}", e),
    }
    match probe_getaccountbalance_1().await {
        Ok(result) => println!("{}", result),
        Err(e) => println!("{}", e),
    }
    match probe_getaccountbalance_2().await {
        Ok(result) => println!("{}", result),
        Err(e) => println!("{}", e),
    }
    match probe_getaccountbalanceatversion_1().await {
        Ok(result) => println!("{}", result),
        Err(e) => println!("{:?}", e),
    }
    match probe_getaccountbalanceatversion_2().await {
        Ok(result) => println!("{}", result),
        Err(e) => println!("{:?}", e),
    }

    Ok(())
}
