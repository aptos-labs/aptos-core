// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    consts::{
        DEVNET_FAUCET_URL, DEVNET_NODE_URL, FUND_AMOUNT, TESTNET_FAUCET_URL, TESTNET_NODE_URL,
    },
    counters::{test_error, test_fail, test_latency, test_step_latency, test_success},
    strings::{ERROR_NO_BALANCE, FAIL_WRONG_BALANCE},
    tests::{coin_transfer, new_account, publish_module, tokenv1_transfer, view_function},
    time_fn,
};
use anyhow::{anyhow, Error, Result};
use aptos_api_types::U64;
use aptos_logger::{error, info};
use aptos_rest_client::{error::RestError, Client, FaucetClient};
use aptos_sdk::types::LocalAccount;
use aptos_types::account_address::AccountAddress;
use std::{env, fmt::Display, num::ParseIntError, str::FromStr};

// Test failure

#[derive(Debug)]
#[allow(dead_code)]
pub enum TestFailure {
    // Variant for failed checks, e.g. wrong balance
    Fail(&'static str),
    // Variant for test failures, e.g. client returns an error
    Error(anyhow::Error),
}

impl From<anyhow::Error> for TestFailure {
    fn from(e: anyhow::Error) -> TestFailure {
        TestFailure::Error(e)
    }
}

impl From<RestError> for TestFailure {
    fn from(e: RestError) -> TestFailure {
        TestFailure::Error(e.into())
    }
}

impl From<ParseIntError> for TestFailure {
    fn from(e: ParseIntError) -> TestFailure {
        TestFailure::Error(e.into())
    }
}

// Test name

#[derive(Clone, Copy)]
pub enum TestName {
    NewAccount,
    CoinTransfer,
    TokenV1Transfer,
    PublishModule,
    ViewFunction,
}

impl TestName {
    pub async fn run(&self, network_name: NetworkName, run_id: &str) {
        let output = match &self {
            TestName::NewAccount => time_fn!(new_account::test, network_name, run_id),
            TestName::CoinTransfer => time_fn!(coin_transfer::test, network_name, run_id),
            TestName::TokenV1Transfer => time_fn!(tokenv1_transfer::test, network_name, run_id),
            TestName::PublishModule => time_fn!(publish_module::test, network_name, run_id),
            TestName::ViewFunction => time_fn!(view_function::test, network_name, run_id),
        };

        emit_test_metrics(output, *self, network_name, run_id);
    }
}

impl Display for TestName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            TestName::NewAccount => write!(f, "new_account"),
            TestName::CoinTransfer => write!(f, "coin_transfer"),
            TestName::TokenV1Transfer => write!(f, "tokenv1_transfer"),
            TestName::PublishModule => write!(f, "publish_module"),
            TestName::ViewFunction => write!(f, "view_function"),
        }
    }
}

// Network name

#[derive(Clone, Copy)]
pub enum NetworkName {
    Testnet,
    Devnet,
}

impl Display for NetworkName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            NetworkName::Testnet => write!(f, "testnet"),
            NetworkName::Devnet => write!(f, "devnet"),
        }
    }
}

impl FromStr for NetworkName {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "testnet" => Ok(NetworkName::Testnet),
            "devnet" => Ok(NetworkName::Devnet),
            _ => Err(anyhow!("invalid network name")),
        }
    }
}

impl NetworkName {
    /// Create a REST client.
    pub fn get_client(&self) -> Client {
        match self {
            NetworkName::Testnet => Client::new(TESTNET_NODE_URL.clone()),
            NetworkName::Devnet => Client::new(DEVNET_NODE_URL.clone()),
        }
    }

    /// Create a faucet client.
    pub fn get_faucet_client(&self) -> FaucetClient {
        match self {
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
}

// Setup helpers

/// Create an account with zero balance.
pub async fn create_account(
    faucet_client: &FaucetClient,
    test_name: TestName,
) -> Result<LocalAccount> {
    let account = LocalAccount::generate(&mut rand::rngs::OsRng);
    faucet_client.create_account(account.address()).await?;

    info!(
        "CREATED ACCOUNT {} for test: {}",
        account.address(),
        test_name.to_string()
    );
    Ok(account)
}

/// Create an account with 100_000_000 balance.
pub async fn create_and_fund_account(
    faucet_client: &FaucetClient,
    test_name: TestName,
) -> Result<LocalAccount> {
    let account = LocalAccount::generate(&mut rand::rngs::OsRng);
    faucet_client.fund(account.address(), FUND_AMOUNT).await?;

    info!(
        "CREATED ACCOUNT {} for test: {}",
        account.address(),
        test_name.to_string()
    );
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
    let actual = match client.view_apt_account_balance(address).await {
        Ok(response) => response.into_inner(),
        Err(e) => {
            error!(
                "test: {} part: check_account_data ERROR: {}, with error {:?}",
                &test_name.to_string(),
                ERROR_NO_BALANCE,
                e
            );
            return Err(e.into());
        },
    };

    // compare
    if expected.0 != actual {
        error!(
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

// Metrics helpers

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

/// Emit metrics based on result.
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
