// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use anyhow::{ensure, Context, Result};
use aptos_api_types::{Block, HashValue, U64};
use aptos_rest_client::{Account, Client, FaucetClient};
use aptos_sdk::coin_client::CoinClient;
use aptos_sdk::types::LocalAccount;
use aptos_types::account_address::AccountAddress;
use aptos_types::transaction::authenticator::AuthenticationKey;
use once_cell::sync::Lazy;
use rand::Rng;
use std::str::FromStr;
use url::Url;

// global parameters (todo: make into clap)
static HAS_FAUCET_ACCESS: bool = false;

// network urls
static DEVNET_NODE_URL: Lazy<Url> =
    Lazy::new(|| Url::parse("https://fullnode.devnet.aptoslabs.com").unwrap());
static DEVNET_FAUCET_URL: Lazy<Url> =
    Lazy::new(|| Url::parse("https://faucet.devnet.aptoslabs.com").unwrap());
static TESTNET_NODE_URL: Lazy<Url> =
    Lazy::new(|| Url::parse("https://fullnode.testnet.aptoslabs.com").unwrap());

// static accounts to use
// don't send coins to TEST_ACCOUNT_1
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
static FAIL_WRONG_BLOCK_HEIGHT: &str = "Returned wrong block height.";
static FAIL_WRONG_BLOCK_HASH: &str = "Returned wrong block hash.";
static FAIL_WRONG_BLOCK_TIMESTAMP: &str = "Returned wrong block timestamp.";
static FAIL_WRONG_FIRST_VERSION: &str = "Returned wrong block first version.";
static FAIL_WRONG_LAST_VERSION: &str = "Returned wrong block last version.";
static FAIL_WRONG_TRANSACTIONS: &str = "Returned wrong transactions.";
static SUCCESS: &str = "success";

/// Calls get_account on a newly created account on devnet. Requires faucet.
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

    // check account data
    let expected_account = Account {
        authentication_key: giray.authentication_key(),
        sequence_number: giray.sequence_number(),
    };
    let actual_account = response.inner();

    ensure!(
        expected_account.authentication_key == actual_account.authentication_key,
        "{} expected {}, got {}",
        FAIL_WRONG_AUTH_KEY,
        expected_account.authentication_key,
        actual_account.authentication_key
    );
    ensure!(
        expected_account.sequence_number == actual_account.sequence_number,
        "{} expected {}, got {}",
        FAIL_WRONG_SEQ_NUMBER,
        expected_account.sequence_number,
        actual_account.sequence_number
    );

    Ok(SUCCESS)
}

/// Calls get_account on a static account on testnet.
async fn probe_getaccount_2() -> Result<&'static str> {
    // create the rest client
    let client = Client::new(TESTNET_NODE_URL.clone());

    // ask for account data
    let response = client
        .get_account(*TEST_ACCOUNT_1)
        .await
        .context(ERROR_CLIENT_RESPONSE)?;

    // check account data
    let expected_account = Account {
        authentication_key: AuthenticationKey::from_str(
            "7fed760c508a8885e1438c90f81dea6940ad53b05527fadddd3851f53342d7c4",
        )
        .context(ERROR_OTHER)?,
        sequence_number: 1,
    };
    let actual_account = response.inner();

    ensure!(
        expected_account.authentication_key == actual_account.authentication_key,
        "{} expected {}, got {}",
        FAIL_WRONG_AUTH_KEY,
        expected_account.authentication_key,
        actual_account.authentication_key
    );
    ensure!(
        expected_account.sequence_number == actual_account.sequence_number,
        "{} expected {}, got {}",
        FAIL_WRONG_SEQ_NUMBER,
        expected_account.sequence_number,
        actual_account.sequence_number
    );

    Ok(SUCCESS)
}

/// Calls get_account_balance on a newly created account on devnet. Requires faucet.
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
    faucet_client
        .fund(giray.address(), random_number)
        .await
        .context(ERROR_FAUCET_FUND)?;

    // ask for account balance
    let response = client
        .get_account_balance(giray.address())
        .await
        .context(ERROR_CLIENT_RESPONSE)?;

    // check balance
    let expected_balance = U64(100_000_000 + random_number);
    let actual_balance = response.inner().coin.value;

    ensure!(
        expected_balance == actual_balance,
        "{} expected {}, got {}",
        FAIL_WRONG_BALANCE,
        expected_balance,
        actual_balance,
    );

    Ok(SUCCESS)
}

/// Calls get_account_balance on a static account on testnet.
async fn probe_getaccountbalance_2() -> Result<&'static str> {
    // create the rest client
    let client = Client::new(TESTNET_NODE_URL.clone());

    // ask for account balance
    let response = client
        .get_account_balance(*TEST_ACCOUNT_1)
        .await
        .context(ERROR_CLIENT_RESPONSE)?;

    // check balance
    let expected_balance = U64(176_767_177);
    let actual_balance = response.inner().coin.value;

    ensure!(
        expected_balance == actual_balance,
        "{} expected {}, got {}",
        FAIL_WRONG_BALANCE,
        expected_balance,
        actual_balance,
    );

    Ok(SUCCESS)
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

/// Calls get_account_balance_at_version on a static account on testnet.
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

    ensure!(
        expected_balance == actual_balance,
        "{} expected {}, got {}",
        FAIL_WRONG_BALANCE_AT_VERSION,
        expected_balance,
        actual_balance,
    );

    // ask for account balance right after funding
    let response = client
        .get_account_balance_at_version(*TEST_ACCOUNT_1, 564_589_720)
        .await
        .context(ERROR_CLIENT_RESPONSE)?;

    // check balance right after funding
    let expected_balance = U64(300_000_000);
    let actual_balance = response.inner().coin.value;

    ensure!(
        expected_balance == actual_balance,
        "{} expected {}, got {}",
        FAIL_WRONG_BALANCE_AT_VERSION,
        expected_balance,
        actual_balance,
    );

    // ask for account balance long after funding
    let response = client
        .get_account_balance_at_version(*TEST_ACCOUNT_1, 564_589_721)
        .await
        .context(ERROR_CLIENT_RESPONSE)?;

    // check balance right after funding
    let expected_balance = U64(300_000_000);
    let actual_balance = response.inner().coin.value;

    ensure!(
        expected_balance == actual_balance,
        "{} expected {}, got {}",
        FAIL_WRONG_BALANCE_AT_VERSION,
        expected_balance,
        actual_balance,
    );

    // ask for account balance right after transfer
    let response = client
        .get_account_balance_at_version(*TEST_ACCOUNT_1, 564_591_130)
        .await
        .context(ERROR_CLIENT_RESPONSE)?;

    // check balance right after funding
    let expected_balance = U64(176_767_177);
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

/// Calls get_block_by_height by setting with_transactions=False on a fixed block on testnet.
async fn probe_getblockbyheight_1() -> Result<&'static str> {
    // create the rest client
    let client: Client = Client::new(TESTNET_NODE_URL.clone());

    // ask for block
    let response = client
        .get_block_by_height(98669388, false)
        .await
        .context(ERROR_CLIENT_RESPONSE)?;

    // check block
    let actual_block = response.inner();
    let expected_block = Block {
        block_height: U64(98669388),
        block_hash: HashValue::from_str(
            "b40362602dccfa1dccebb94adfd4c06d4b0ffe1eadb07659556744db3fe1d0e5",
        )
        .context(ERROR_OTHER)?,
        block_timestamp: U64(1688146946653187),
        first_version: U64(565538916),
        last_version: U64(565538919),
        transactions: None,
    };

    ensure!(
        actual_block.block_height == expected_block.block_height,
        "{} expected {}, got {}",
        FAIL_WRONG_BLOCK_HEIGHT,
        expected_block.block_height,
        actual_block.block_height,
    );
    ensure!(
        actual_block.block_hash == expected_block.block_hash,
        "{} expected {}, got {}",
        FAIL_WRONG_BLOCK_HASH,
        expected_block.block_hash,
        actual_block.block_hash,
    );
    ensure!(
        actual_block.block_timestamp == expected_block.block_timestamp,
        "{} expected {}, got {}",
        FAIL_WRONG_BLOCK_TIMESTAMP,
        expected_block.block_timestamp,
        actual_block.block_timestamp,
    );
    ensure!(
        actual_block.first_version == expected_block.first_version,
        "{} expected {}, got {}",
        FAIL_WRONG_FIRST_VERSION,
        expected_block.first_version,
        actual_block.first_version,
    );
    ensure!(
        actual_block.last_version == expected_block.last_version,
        "{} expected {}, got {}",
        FAIL_WRONG_LAST_VERSION,
        expected_block.last_version,
        actual_block.last_version,
    );
    ensure!(
        actual_block.transactions == expected_block.transactions,
        "{} expected {:?}, got {:?}",
        FAIL_WRONG_TRANSACTIONS,
        expected_block.transactions,
        actual_block.transactions,
    );

    Ok(SUCCESS)
}

/// Calls get_block_by_version by setting with_transactions=False on a fixed block on testnet.
/// The version belongs to a user transaction inside the block.
async fn probe_getblockbyversion_1() -> Result<&'static str> {
    // create the rest client
    let client: Client = Client::new(TESTNET_NODE_URL.clone());

    // ask for block
    let response = client
        .get_block_by_version(565767993, false)
        .await
        .context(ERROR_CLIENT_RESPONSE)?;

    // check block
    let actual_block = response.inner();
    let expected_block = Block {
        block_height: U64(98764388),
        block_hash: HashValue::from_str(
            "f1fcfd799f88a2526d649d5a2cf227c14b77ef374b5daf6fb5d4987c430a91dd",
        )
        .context(ERROR_OTHER)?,
        block_timestamp: U64(1688165648699603),
        first_version: U64(565767992),
        last_version: U64(565767994),
        transactions: None,
    };

    ensure!(
        actual_block.block_height == expected_block.block_height,
        "{} expected {}, got {}",
        FAIL_WRONG_BLOCK_HEIGHT,
        expected_block.block_height,
        actual_block.block_height,
    );
    ensure!(
        actual_block.block_hash == expected_block.block_hash,
        "{} expected {}, got {}",
        FAIL_WRONG_BLOCK_HASH,
        expected_block.block_hash,
        actual_block.block_hash,
    );
    ensure!(
        actual_block.block_timestamp == expected_block.block_timestamp,
        "{} expected {}, got {}",
        FAIL_WRONG_BLOCK_TIMESTAMP,
        expected_block.block_timestamp,
        actual_block.block_timestamp,
    );
    ensure!(
        actual_block.first_version == expected_block.first_version,
        "{} expected {}, got {}",
        FAIL_WRONG_FIRST_VERSION,
        expected_block.first_version,
        actual_block.first_version,
    );
    ensure!(
        actual_block.last_version == expected_block.last_version,
        "{} expected {}, got {}",
        FAIL_WRONG_LAST_VERSION,
        expected_block.last_version,
        actual_block.last_version,
    );
    ensure!(
        actual_block.transactions == expected_block.transactions,
        "{} expected {:?}, got {:?}",
        FAIL_WRONG_TRANSACTIONS,
        expected_block.transactions,
        actual_block.transactions,
    );

    Ok(SUCCESS)
}

#[tokio::main]
async fn main() -> Result<()> {
    match probe_getaccount_1().await {
        Ok(result) => println!("{:?}", result),
        Err(e) => println!("{:?}", e),
    }
    match probe_getaccount_2().await {
        Ok(result) => println!("{:?}", result),
        Err(e) => println!("{:?}", e),
    }
    match probe_getaccountbalance_1().await {
        Ok(result) => println!("{:?}", result),
        Err(e) => println!("{:?}", e),
    }
    match probe_getaccountbalance_2().await {
        Ok(result) => println!("{:?}", result),
        Err(e) => println!("{:?}", e),
    }
    match probe_getaccountbalanceatversion_1().await {
        Ok(result) => println!("{:?}", result),
        Err(e) => println!("{:?}", e),
    }
    match probe_getaccountbalanceatversion_2().await {
        Ok(result) => println!("{:?}", result),
        Err(e) => println!("{:?}", e),
    }
    match probe_getblockbyheight_1().await {
        Ok(result) => println!("{:?}", result),
        Err(e) => println!("{:?}", e),
    }
    match probe_getblockbyversion_1().await {
        Ok(result) => println!("{:?}", result),
        Err(e) => println!("{:?}", e),
    }

    Ok(())
}
