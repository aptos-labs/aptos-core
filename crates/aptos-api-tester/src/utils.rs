// Copyright © Aptos Foundation

use crate::counters::{test_error, test_fail, test_latency, test_success};
use aptos_rest_client::error::RestError;
use once_cell::sync::Lazy;
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

// Helper function to set metrics based on the result.
pub fn set_metrics(output: &TestResult, test_name: &str, network_name: &str, time: f64) {
    match output {
        TestResult::Success => {
            test_success(test_name, network_name).observe(1_f64);
            test_fail(test_name, network_name).observe(0_f64);
            test_error(test_name, network_name).observe(0_f64);
            test_latency(test_name, network_name, "success").observe(time);
        },
        TestResult::Fail(_) => {
            test_success(test_name, network_name).observe(0_f64);
            test_fail(test_name, network_name).observe(1_f64);
            test_error(test_name, network_name).observe(0_f64);
            test_latency(test_name, network_name, "fail").observe(time);
        },
        TestResult::Error(_) => {
            test_success(test_name, network_name).observe(0_f64);
            test_fail(test_name, network_name).observe(0_f64);
            test_error(test_name, network_name).observe(1_f64);
            test_latency(test_name, network_name, "error").observe(time);
        },
    }
}
