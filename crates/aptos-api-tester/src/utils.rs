// Copyright © Aptos Foundation

use crate::counters::{test_error, test_fail, test_latency, test_success};
use anyhow::Result;
use aptos_rest_client::{error::RestError, Client, FaucetClient};
use aptos_sdk::types::LocalAccount;
use once_cell::sync::Lazy;
use std::env;
use url::Url;

// network urls
pub static DEVNET_NODE_URL: Lazy<Url> =
    Lazy::new(|| Url::parse("https://fullnode.devnet.aptoslabs.com").unwrap());
pub static DEVNET_FAUCET_URL: Lazy<Url> =
    Lazy::new(|| Url::parse("https://faucet.devnet.aptoslabs.com").unwrap());
pub static TESTNET_NODE_URL: Lazy<Url> =
    Lazy::new(|| Url::parse("https://fullnode.testnet.aptoslabs.com").unwrap());
pub static TESTNET_FAUCET_URL: Lazy<Url> =
    Lazy::new(|| Url::parse("https://faucet.testnet.aptoslabs.com").unwrap());

#[derive(Debug)]
pub enum TestResult {
    Success,
    Fail(&'static str),
    Error(anyhow::Error),
}

impl From<TestFailure> for TestResult {
    fn from(f: TestFailure) -> TestResult {
        match f {
            TestFailure::Fail(f) => TestResult::Fail(f),
            TestFailure::Error(e) => TestResult::Error(e),
        }
    }
}

#[derive(Debug)]
pub enum TestFailure {
    Fail(&'static str),
    Error(anyhow::Error),
}

impl From<RestError> for TestFailure {
    fn from(e: RestError) -> TestFailure {
        TestFailure::Error(e.into())
    }
}

impl From<anyhow::Error> for TestFailure {
    fn from(e: anyhow::Error) -> TestFailure {
        TestFailure::Error(e)
    }
}

pub enum TestName {
    NewAccount,
    CoinTransfer,
    NftTransfer,
    PublishModule,
}

impl ToString for TestName {
    fn to_string(&self) -> String {
        match &self {
            TestName::NewAccount => "new_account".to_string(),
            TestName::CoinTransfer => "coin_transfer".to_string(),
            TestName::NftTransfer => "nft_transfer".to_string(),
            TestName::PublishModule => "publish_module".to_string(),
        }
    }
}

#[derive(Clone, Copy)]
pub enum NetworkName {
    Testnet,
    Devnet,
}

impl ToString for NetworkName {
    fn to_string(&self) -> String {
        match &self {
            NetworkName::Testnet => "testnet".to_string(),
            NetworkName::Devnet => "devnet".to_string(),
        }
    }
}

// Set metrics based on the result.
pub fn set_metrics(
    output: &TestResult,
    test_name: &str,
    network_name: &str,
    start_time: &str,
    time: f64,
) {
    match output {
        TestResult::Success => {
            test_success(test_name, network_name, start_time).observe(1_f64);
            test_fail(test_name, network_name, start_time).observe(0_f64);
            test_error(test_name, network_name, start_time).observe(0_f64);
            test_latency(test_name, network_name, start_time, "success").observe(time);
        },
        TestResult::Fail(_) => {
            test_success(test_name, network_name, start_time).observe(0_f64);
            test_fail(test_name, network_name, start_time).observe(1_f64);
            test_error(test_name, network_name, start_time).observe(0_f64);
            test_latency(test_name, network_name, start_time, "fail").observe(time);
        },
        TestResult::Error(_) => {
            test_success(test_name, network_name, start_time).observe(0_f64);
            test_fail(test_name, network_name, start_time).observe(0_f64);
            test_error(test_name, network_name, start_time).observe(1_f64);
            test_latency(test_name, network_name, start_time, "error").observe(time);
        },
    }
}

// Create a REST client.
pub fn get_client(network_name: NetworkName) -> Client {
    match network_name {
        NetworkName::Testnet => Client::new(TESTNET_NODE_URL.clone()),
        NetworkName::Devnet => Client::new(DEVNET_NODE_URL.clone()),
    }
}

// Create a faucet client.
pub fn get_faucet_client(network_name: NetworkName) -> FaucetClient {
    match network_name {
        NetworkName::Testnet => {
            let faucet_client =
                FaucetClient::new(TESTNET_FAUCET_URL.clone(), TESTNET_NODE_URL.clone());
            match env::var("TESTNET_FAUCET_CLIENT_TOKEN") {
                Ok(token) => faucet_client.with_auth_token(token),
                Err(_) => faucet_client,
            }
        },
        NetworkName::Devnet => {
            FaucetClient::new(DEVNET_FAUCET_URL.clone(), DEVNET_NODE_URL.clone())
        },
    }
}

// Create an account with zero balance.
pub async fn create_account(faucet_client: &FaucetClient) -> Result<LocalAccount> {
    let account = LocalAccount::generate(&mut rand::rngs::OsRng);
    faucet_client.create_account(account.address()).await?;

    Ok(account)
}

// Create an account with 100_000_000 balance.
pub async fn create_and_fund_account(faucet_client: &FaucetClient) -> Result<LocalAccount> {
    let account = LocalAccount::generate(&mut rand::rngs::OsRng);
    faucet_client.fund(account.address(), 100_000_000).await?;

    Ok(account)
}
