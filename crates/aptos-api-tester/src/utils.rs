// Copyright © Aptos Foundation

use crate::{
    consts::{
        DEVNET_FAUCET_URL, DEVNET_NODE_URL, FUND_AMOUNT, TESTNET_FAUCET_URL, TESTNET_NODE_URL,
    },
    counters::{test_error, test_fail, test_latency, test_step_latency, test_success},
    fail_message::{ERROR_NO_BALANCE, FAIL_WRONG_BALANCE},
    tests::{coin_transfer, new_account, nft_transfer, publish_module},
    time_fn,
};
use anyhow::Result;
use aptos_api_types::U64;
use aptos_logger::info;
use aptos_rest_client::{error::RestError, Client, FaucetClient};
use aptos_sdk::types::LocalAccount;
use aptos_types::account_address::AccountAddress;
use std::env;

// test failure

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

// test name

#[derive(Clone, Copy)]
pub enum TestName {
    NewAccount,
    CoinTransfer,
    NftTransfer,
    PublishModule,
}

impl TestName {
    pub async fn run(&self, network_name: NetworkName, run_id: &str) {
        let output = match &self {
            TestName::NewAccount => time_fn!(new_account::test, network_name, run_id),
            TestName::CoinTransfer => time_fn!(coin_transfer::test, network_name, run_id),
            TestName::NftTransfer => time_fn!(nft_transfer::test, network_name, run_id),
            TestName::PublishModule => time_fn!(publish_module::test, network_name, run_id),
        };

        emit_test_metrics(output, *self, network_name, run_id);
    }
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

// network name

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

// setup helpers

/// Create a REST client.
pub fn get_client(network_name: NetworkName) -> Client {
    match network_name {
        NetworkName::Testnet => Client::new(TESTNET_NODE_URL.clone()),
        NetworkName::Devnet => Client::new(DEVNET_NODE_URL.clone()),
    }
}

/// Create a faucet client.
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

/// Create an account with zero balance.
pub async fn create_account(faucet_client: &FaucetClient) -> Result<LocalAccount> {
    let account = LocalAccount::generate(&mut rand::rngs::OsRng);
    faucet_client.create_account(account.address()).await?;

    Ok(account)
}

/// Create an account with 100_000_000 balance.
pub async fn create_and_fund_account(faucet_client: &FaucetClient) -> Result<LocalAccount> {
    let account = LocalAccount::generate(&mut rand::rngs::OsRng);
    faucet_client.fund(account.address(), FUND_AMOUNT).await?;

    Ok(account)
}

/// Check account balance.
pub async fn check_balance(
    test_name: TestName,
    client: &Client,
    address: AccountAddress,
    expected: U64,
) -> Result<(), TestFailure> {
    // actual
    let actual = match client.get_account_balance(address).await {
        Ok(response) => response.into_inner().coin.value,
        Err(e) => {
            info!(
                "test: {} part: check_account_data ERROR: {}, with error {:?}",
                &test_name.to_string(),
                ERROR_NO_BALANCE,
                e
            );
            return Err(e.into());
        },
    };

    // compare
    if expected != actual {
        info!(
            "test: {} part: check_account_data FAIL: {}, expected {:?}, got {:?}",
            &test_name.to_string(),
            FAIL_WRONG_BALANCE,
            expected,
            actual
        );
        return Err(TestFailure::Fail(FAIL_WRONG_BALANCE));
    }

    Ok(())
}

// metrics helpers

/// Emit metrics based on test result.
pub fn emit_test_metrics(
    output: (Result<(), TestFailure>, f64),
    test_name: TestName,
    network_name: NetworkName,
    run_id: &str,
) {
    // deconstruct
    let (result, time) = output;

    // emit success rate and get result word
    let result_label = match result {
        Ok(_) => {
            test_success(&test_name.to_string(), &network_name.to_string(), run_id).observe(1_f64);
            test_fail(&test_name.to_string(), &network_name.to_string(), run_id).observe(0_f64);
            test_error(&test_name.to_string(), &network_name.to_string(), run_id).observe(0_f64);

            "success"
        },
        Err(e) => match e {
            TestFailure::Fail(_) => {
                test_success(&test_name.to_string(), &network_name.to_string(), run_id)
                    .observe(0_f64);
                test_fail(&test_name.to_string(), &network_name.to_string(), run_id).observe(1_f64);
                test_error(&test_name.to_string(), &network_name.to_string(), run_id)
                    .observe(0_f64);

                "fail"
            },
            TestFailure::Error(_) => {
                test_success(&test_name.to_string(), &network_name.to_string(), run_id)
                    .observe(0_f64);
                test_fail(&test_name.to_string(), &network_name.to_string(), run_id).observe(0_f64);
                test_error(&test_name.to_string(), &network_name.to_string(), run_id)
                    .observe(1_f64);

                "error"
            },
        },
    };

    // log result
    info!(
        "----- TEST FINISHED test: {} result: {} time: {} -----",
        test_name.to_string(),
        result_label,
        time,
    );

    // emit latency
    test_latency(
        &test_name.to_string(),
        &network_name.to_string(),
        run_id,
        result_label,
    )
    .observe(time);
}

/// Emit metrics based on  result.
pub fn emit_step_metrics<T>(
    output: (Result<T, TestFailure>, f64),
    test_name: TestName,
    step_name: &str,
    network_name: NetworkName,
    run_id: &str,
) -> Result<T, TestFailure> {
    // deconstruct and get result word
    let (result, time) = output;
    let result_label = match &result {
        Ok(_) => "success",
        Err(e) => match e {
            TestFailure::Fail(_) => "fail",
            TestFailure::Error(_) => "error",
        },
    };

    // log result
    info!(
        "STEP FINISHED test: {} step: {} result: {} time: {}",
        test_name.to_string(),
        step_name,
        result_label,
        time,
    );

    // emit latency
    test_step_latency(
        &test_name.to_string(),
        step_name,
        &network_name.to_string(),
        run_id,
        result_label,
    )
    .observe(time);

    result
}
