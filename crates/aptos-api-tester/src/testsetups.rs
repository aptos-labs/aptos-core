// Copyright © Aptos Foundation

use crate::{
    tests::{test_cointransfer, test_newaccount, test_nfttransfer, test_publishmodule},
    utils::{
        create_account, create_and_fund_account, get_client, get_faucet_client, NetworkName,
        TestFailure,
    },
};
use anyhow::Result;
use aptos_sdk::{coin_client::CoinClient, token_client::TokenClient};

pub async fn setup_and_run_newaccount(network_name: NetworkName) -> Result<(), TestFailure> {
    // spin up clients
    let client = get_client(network_name);
    let faucet_client = get_faucet_client(network_name);

    // create and fund account
    let account = create_and_fund_account(&faucet_client).await?;

    // run test
    test_newaccount(&client, &account, 100_000_000).await
}

pub async fn setup_and_run_cointransfer(network_name: NetworkName) -> Result<(), TestFailure> {
    // spin up clients
    let client = get_client(network_name);
    let faucet_client = get_faucet_client(network_name);
    let coin_client = CoinClient::new(&client);

    // create and fund accounts
    let mut account = create_and_fund_account(&faucet_client).await?;
    let receiver = create_account(&faucet_client).await?;

    // run test
    test_cointransfer(
        &client,
        &coin_client,
        &mut account,
        receiver.address(),
        1_000,
    )
    .await
}

pub async fn setup_and_run_nfttransfer(network_name: NetworkName) -> Result<(), TestFailure> {
    // spin up clients
    let client = get_client(network_name);
    let faucet_client = get_faucet_client(network_name);
    let token_client = TokenClient::new(&client);

    // create and fund accounts
    let mut account = create_and_fund_account(&faucet_client).await?;
    let mut receiver = create_and_fund_account(&faucet_client).await?;

    // run test
    test_nfttransfer(&client, &token_client, &mut account, &mut receiver).await
}

pub async fn setup_and_run_publishmodule(network_name: NetworkName) -> Result<(), TestFailure> {
    // spin up clients
    let client = get_client(network_name);
    let faucet_client = get_faucet_client(network_name);

    // create and fund accounts
    let mut account = create_and_fund_account(&faucet_client).await?;

    // run test
    test_publishmodule(&client, &mut account).await
}
