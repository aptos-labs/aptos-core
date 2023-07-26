// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

mod counters;
mod tests;
mod utils;

use crate::{
    tests::{test_cointransfer, test_mintnft, test_module, test_newaccount},
    utils::{
        set_metrics, NetworkName, TestFailure, TestName, TestResult, DEVNET_FAUCET_URL,
        DEVNET_NODE_URL, TESTNET_FAUCET_URL, TESTNET_NODE_URL,
    },
};
use anyhow::{anyhow, Result};
use aptos_logger::{info, Level, Logger};
use aptos_push_metrics::MetricsPusher;
use aptos_rest_client::{Client, FaucetClient};
use aptos_sdk::{coin_client::CoinClient, token_client::TokenClient, types::LocalAccount};
use std::{future::Future, time::Instant};

// Processes a test result.
async fn handle_result<Fut: Future<Output = Result<(), TestFailure>>>(
    test_name: TestName,
    network_type: NetworkName,
    fut: Fut,
) -> Result<TestResult> {
    // start timer
    let start = Instant::now();

    // call the flow
    let result = fut.await;

    // end timer
    let time = (Instant::now() - start).as_micros() as f64;

    // process the result
    let output = match result {
        Ok(_) => TestResult::Success,
        Err(failure) => TestResult::from(failure),
    };

    // set metrics and log
    set_metrics(
        &output,
        &test_name.to_string(),
        &network_type.to_string(),
        time,
    );
    info!(
        "{} {} result:{:?} in time:{:?}",
        network_type.to_string(),
        test_name.to_string(),
        output,
        time,
    );

    Ok(output)
}

async fn test_flows(
    network_type: NetworkName,
    client: Client,
    faucet_client: FaucetClient,
) -> Result<()> {
    info!("testing {}", network_type.to_string());

    // create clients
    let coin_client = CoinClient::new(&client);
    let token_client = TokenClient::new(&client);

    // create and fund account for tests
    let mut giray = LocalAccount::generate(&mut rand::rngs::OsRng);
    faucet_client.fund(giray.address(), 100_000_000).await?;
    info!("{:?}", giray.address());

    let mut giray2 = LocalAccount::generate(&mut rand::rngs::OsRng);
    faucet_client.fund(giray2.address(), 100_000_000).await?;
    info!("{:?}", giray2.address());

    // Test new account creation and funding
    // this test is critical to pass for the next tests
    match handle_result(
        TestName::NewAccount,
        network_type,
        test_newaccount(&client, &giray, 100_000_000),
    )
    .await?
    {
        TestResult::Success => {},
        _ => return Err(anyhow!("returning early because new account test failed")),
    }

    // Flow 1: Coin transfer
    let _ = handle_result(
        TestName::CoinTransfer,
        network_type,
        test_cointransfer(&client, &coin_client, &mut giray, giray2.address(), 1_000),
    )
    .await;

    // Flow 2: NFT transfer
    let _ = handle_result(
        TestName::NftTransfer,
        network_type,
        test_mintnft(&client, &token_client, &mut giray, &mut giray2),
    )
    .await;

    // Flow 3: Publishing module
    let _ = handle_result(
        TestName::PublishModule,
        network_type,
        test_module(&client, &mut giray),
    )
    .await;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // log metrics
    Logger::builder().level(Level::Info).build();
    let _mp = MetricsPusher::start_for_local_run("api-tester");

    // test flows on testnet
    let _ = test_flows(
        NetworkName::Testnet,
        Client::new(TESTNET_NODE_URL.clone()),
        FaucetClient::new(TESTNET_FAUCET_URL.clone(), TESTNET_NODE_URL.clone()),
    )
    .await;

    // test flows on devnet
    let _ = test_flows(
        NetworkName::Devnet,
        Client::new(DEVNET_NODE_URL.clone()),
        FaucetClient::new(DEVNET_FAUCET_URL.clone(), DEVNET_NODE_URL.clone()),
    )
    .await;

    Ok(())
}
