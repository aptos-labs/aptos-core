// Copyright © Aptos Foundation

use aptos_rest_client::error::RestError;

#[derive(Debug)]
pub struct TestLog {
    pub result: TestResult,
    pub time: f64,
}

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
