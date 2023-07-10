// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use std::future::Future;

use anyhow::{anyhow, Result};
use aptos_api_types::U64;
use aptos_rest_client::{Account, Client, FaucetClient};
use aptos_sdk::coin_client::CoinClient;
use aptos_sdk::token_client::TokenClient;
use aptos_sdk::types::LocalAccount;
use aptos_types::account_address::AccountAddress;
use once_cell::sync::Lazy;
use url::Url;

// network urls
// static DEVNET_NODE_URL: Lazy<Url> =
//     Lazy::new(|| Url::parse("https://fullnode.devnet.aptoslabs.com").unwrap());
// static DEVNET_FAUCET_URL: Lazy<Url> =
//     Lazy::new(|| Url::parse("https://faucet.devnet.aptoslabs.com").unwrap());
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

#[derive(Debug)]
enum TestResult {
    Success,
    Fail(&'static str),
    Error(anyhow::Error),
}

async fn handle_result<Fut: Future<Output = TestResult>>(fut: Fut) -> TestResult {
    let result = fut.await;
    println!("{:?}", result);

    result
}

/// Tests new account creation. Checks that:
///   - account data exists
///   - account balance reflects funded amount
async fn test_newaccount(
    client: &Client,
    account: &LocalAccount,
    amount_funded: u64,
) -> TestResult {
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

    if expected_account.authentication_key != actual_account.authentication_key {
        return TestResult::Fail("wrong authentication key");
    }
    if expected_account.sequence_number != actual_account.sequence_number {
        return TestResult::Fail("wrong sequence number");
    }

    // check account balance
    let expected_balance = U64(amount_funded);
    let actual_balance = match client.get_account_balance(account.address()).await {
        Ok(response) => response.inner().coin.value,
        Err(e) => return TestResult::Error(e.into()),
    };

    if expected_balance != actual_balance {
        return TestResult::Fail("wrong balance");
    }

    TestResult::Success
}

/// Tests coin transfer. Checks that:
///   - receiver balance reflects transferred amount
///   - receiver balance shows correct amount at the previous version
async fn test_cointransfer(
    client: &Client,
    coin_client: &CoinClient<'_>,
    account: &mut LocalAccount,
    address: AccountAddress,
    amount: u64,
) -> TestResult {
    // get starting balance
    let starting_receiver_balance = match client.get_account_balance(address).await {
        Ok(response) => u64::from(response.inner().coin.value),
        Err(e) => return TestResult::Error(e.into()),
    };

    // transfer coins to static account
    let pending_txn = match coin_client.transfer(account, address, amount, None).await {
        Ok(txn) => txn,
        Err(e) => return TestResult::Error(e),
    };
    let response = match client.wait_for_transaction(&pending_txn).await {
        Ok(response) => response,
        Err(e) => return TestResult::Error(e.into()),
    };

    // check receiver balance
    let expected_receiver_balance = U64(starting_receiver_balance + amount);
    let actual_receiver_balance = match client.get_account_balance(address).await {
        Ok(response) => response.inner().coin.value,
        Err(e) => return TestResult::Error(e.into()),
    };

    if expected_receiver_balance != actual_receiver_balance {
        return TestResult::Fail("wrong balance after coin transfer");
    }

    // check account balance with a lower version number
    let version = match response.inner().version() {
        Some(version) => version,
        _ => return TestResult::Error(anyhow!("transaction did not return version")),
    };

    let expected_balance_at_version = U64(starting_receiver_balance);
    let actual_balance_at_version = match client
        .get_account_balance_at_version(address, version - 1)
        .await
    {
        Ok(response) => response.inner().coin.value,
        Err(e) => return TestResult::Error(e.into()),
    };

    if expected_balance_at_version != actual_balance_at_version {
        return TestResult::Fail("wrong balance at version before the coin transfer");
    }

    // TODO: do we want to check transaction details returned by the API?
    TestResult::Success
}

async fn test_mintnft(
    client: &Client,
    token_client: &TokenClient<'_>,
    account: &mut LocalAccount,
) -> TestResult {
    // create collection
    let collection_name = "test collection";
    let pending_txn = match token_client
        .create_collection(
            account,
            &collection_name,
            "collection desc",
            "collection",
            1000,
            None,
        )
        .await
    {
        Ok(txn) => txn,
        Err(e) => return TestResult::Error(e),
    };
    match client.wait_for_transaction(&pending_txn).await {
        Ok(_) => {},
        Err(e) => return TestResult::Error(e.into()),
    }

    // create token
    let pending_txn = match token_client
        .create_token(
            account,
            &collection_name,
            "test token",
            "token desc",
            10,
            "token",
            10,
            None,
            None,
        )
        .await
    {
        Ok(txn) => txn,
        Err(e) => return TestResult::Error(e),
    };
    match client.wait_for_transaction(&pending_txn).await {
        Ok(_) => {},
        Err(e) => return TestResult::Error(e.into()),
    }

    TestResult::Success
}

async fn testnet_1() -> Result<()> {
    // create clients
    let client: Client = Client::new(TESTNET_NODE_URL.clone());
    let faucet_client = FaucetClient::new(TESTNET_FAUCET_URL.clone(), TESTNET_NODE_URL.clone());
    let coin_client = CoinClient::new(&client);
    let token_client = TokenClient::new(&client);

    // create and fund account for tests
    let mut giray = LocalAccount::generate(&mut rand::rngs::OsRng);
    faucet_client.fund(giray.address(), 100_000_000).await?;
    println!("{:?}", giray.address());

    // Step 1: Test new account creation and funding
    // this test is critical to pass for the next tests
    let result = handle_result(test_newaccount(&client, &giray, 100_000_000)).await;
    match result {
        TestResult::Success => {},
        _ => return Err(anyhow!("returning early because new account test failed")),
    }

    // Step 2: Test coin transfer
    handle_result(test_cointransfer(
        &client,
        &coin_client,
        &mut giray,
        *TEST_ACCOUNT_1,
        1_000,
    ))
    .await;

    // Step 3: Test NFT minting
    handle_result(test_mintnft(&client, &token_client, &mut giray)).await;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = testnet_1().await;

    Ok(())
}
