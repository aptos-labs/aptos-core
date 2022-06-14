// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{aptos_cli::launch_faucet, smoke_test_environment::new_local_swarm_with_aptos};
use aptos::{account::create::DEFAULT_FUNDED_COINS, test::CliTestFramework};
use aptos_config::config::ApiConfig;
use aptos_rosetta::{client::RosettaClient, types::AccountBalanceResponse, CURRENCY, NUM_DECIMALS};
use aptos_types::{account_address::AccountAddress, chain_id::ChainId};
use forge::{LocalSwarm, Node};
use std::{str::FromStr, time::Duration};

pub async fn setup_test(num_nodes: usize) -> (LocalSwarm, CliTestFramework, RosettaClient) {
    let swarm = new_local_swarm_with_aptos(num_nodes).await;
    let chain_id = swarm.chain_id();
    let validator = swarm.validators().next().unwrap();
    let root_key = swarm.root_key();
    let _faucet = launch_faucet(validator.rest_api_endpoint(), root_key, chain_id);

    // Connect the operator tool to the node's JSON RPC API
    let tool = CliTestFramework::new(
        validator.rest_api_endpoint(),
        "http://localhost:9996".parse().unwrap(),
        2,
    )
    .await;

    // And the client
    let rosetta_socket_addr = "127.0.0.1:9997";
    let rosetta_url = format!("http://{}", rosetta_socket_addr).parse().unwrap();
    let rosetta_client = RosettaClient::new(rosetta_url);
    let rosetta_socket_addr = "127.0.0.1:9997";
    let api_config = ApiConfig {
        enabled: true,
        address: rosetta_socket_addr.parse().unwrap(),
        tls_cert_path: None,
        tls_key_path: None,
        content_length_limit: None,
    };

    // Start the server
    let _ = aptos_rosetta::bootstrap_async(
        swarm.chain_id(),
        api_config,
        aptos_rest_client::Client::new(validator.rest_api_endpoint()),
    )
    .await
    .unwrap();

    (swarm, tool, rosetta_client)
}

#[tokio::test]
#[ignore]
async fn test_account_balance() {
    let (_swarm, cli, rosetta_client) = setup_test(1).await;

    cli.create_account_with_faucet(0).await.unwrap();
    let account = CliTestFramework::account_id(0);

    let response = get_account_balance_once_ready(&rosetta_client, account)
        .await
        .unwrap();
    assert_eq!(1, response.balances.len());
    let balance = response.balances.first().unwrap();
    let currency = &balance.currency;
    assert_eq!(CURRENCY, currency.symbol);
    assert_eq!(NUM_DECIMALS, currency.decimals);
    assert_eq!(DEFAULT_FUNDED_COINS, u64::from_str(&balance.value).unwrap());
}

async fn get_account_balance_once_ready(
    rosetta_client: &RosettaClient,
    account: AccountAddress,
) -> anyhow::Result<AccountBalanceResponse> {
    let mut result = Err(anyhow::Error::msg("Failed to get balance"));
    for _ in 1..10 {
        result = rosetta_client
            .account_balance_simple(account, ChainId::test())
            .await;
        if result.is_ok() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    result
}
