// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{aptos_cli::launch_faucet, smoke_test_environment::new_local_swarm_with_aptos};
use aptos::{account::create::DEFAULT_FUNDED_COINS, test::CliTestFramework};
use aptos_config::config::ApiConfig;
use aptos_rosetta::{
    client::RosettaClient,
    types::{AccountBalanceRequest, BlockRequest, Currency},
};
use forge::{LocalSwarm, Node};
use std::{future::Future, str::FromStr, time::Duration};

pub async fn setup_test(
    num_nodes: usize,
    num_accounts: usize,
) -> (LocalSwarm, CliTestFramework, RosettaClient) {
    let swarm = new_local_swarm_with_aptos(num_nodes).await;
    let chain_id = swarm.chain_id();
    let validator = swarm.validators().next().unwrap();
    let root_key = swarm.root_key();
    let _faucet = launch_faucet(validator.rest_api_endpoint(), root_key, chain_id);

    // Connect the operator tool to the node's JSON RPC API
    let tool = CliTestFramework::new(
        validator.rest_api_endpoint(),
        "http://localhost:9997".parse().unwrap(),
        2,
    )
    .await;

    // And the client
    let rosetta_socket_addr = "127.0.0.1:9998";
    let rosetta_url = format!("http://{}", rosetta_socket_addr).parse().unwrap();
    let rosetta_client = RosettaClient::new(rosetta_url);
    let api_config = ApiConfig {
        enabled: true,
        address: rosetta_socket_addr.parse().unwrap(),
        tls_cert_path: None,
        tls_key_path: None,
        content_length_limit: None,
    };

    // Start the server
    let _rosetta = aptos_rosetta::bootstrap_async(
        swarm.chain_id(),
        api_config,
        aptos_rest_client::Client::new(validator.rest_api_endpoint()),
    )
    .await
    .unwrap();

    // Create accounts
    for i in 0..num_accounts {
        tool.create_account_with_faucet(i).await.unwrap();
    }
    (swarm, tool, rosetta_client)
}

#[tokio::test]
#[ignore]
async fn test_account_balance() {
    let (swarm, _cli, rosetta_client) = setup_test(1, 1).await;
    let account = CliTestFramework::account_id(0);
    let chain_id = swarm.chain_id();
    let request = AccountBalanceRequest {
        network_identifier: chain_id.into(),
        account_identifier: account.into(),
        block_identifier: None,
        currencies: None,
    };

    let response = try_until_ok(|| rosetta_client.account_balance(&request))
        .await
        .unwrap();
    assert_eq!(1, response.balances.len());
    let balance = response.balances.first().unwrap();
    assert_eq!(DEFAULT_FUNDED_COINS, u64::from_str(&balance.value).unwrap());
    assert_eq!(&Currency::test_coin(), &balance.currency);
}

#[tokio::test]
#[ignore]
// TODO: Fix test so it doesn't conflict with other tests
async fn test_block() {
    let (swarm, _cli, rosetta_client) = setup_test(1, 0).await;
    let chain_id = swarm.chain_id();

    let request_genesis = BlockRequest::by_version(chain_id, 0);
    let by_version_response = try_until_ok(|| rosetta_client.block(&request_genesis))
        .await
        .unwrap();
    let genesis_block = by_version_response.block.unwrap();

    // Genesis txn should always have parent be same as block
    assert_eq!(
        genesis_block.block_identifier,
        genesis_block.parent_block_identifier
    );

    // Genesis timestamp is always 0
    assert_eq!(0, genesis_block.timestamp);

    // There should only be the genesis transaction
    assert_eq!(1, genesis_block.transactions.len());
    let genesis_txn = genesis_block.transactions.first().unwrap();
    // TODO: Verify operations

    // Get genesis txn by hash
    let request_genesis_by_hash =
        BlockRequest::by_hash(chain_id, genesis_txn.transaction_identifier.hash.clone());
    let by_hash_response = rosetta_client
        .block(&request_genesis_by_hash)
        .await
        .unwrap();
    let genesis_block_by_hash = by_hash_response.block.unwrap();

    // Both blocks should be the same
    assert_eq!(genesis_block, genesis_block_by_hash);

    // Responses should be idempotent
    let response = rosetta_client.block(&request_genesis).await.unwrap();
    assert_eq!(response.block.unwrap(), genesis_block_by_hash);
    let response = rosetta_client
        .block(&request_genesis_by_hash)
        .await
        .unwrap();
    assert_eq!(response.block.unwrap(), genesis_block_by_hash);

    // No input should give the latest version, not the genesis txn
    let request_latest = BlockRequest::latest(chain_id);
    let response = rosetta_client.block(&request_latest).await.unwrap();
    let latest_block = response.block.unwrap();

    // The latest block should always come after genesis
    assert!(latest_block.block_identifier.index > genesis_block.block_identifier.index);
    assert!(latest_block.timestamp > genesis_block.timestamp);

    // The parent should always be exactly one version before
    assert_eq!(
        latest_block.parent_block_identifier.index,
        latest_block.block_identifier.index - 1
    );

    // There should be exactly one txn
    assert_eq!(1, latest_block.transactions.len());

    // We should be able to query it again by hash or by version and it is the same
    let request_latest_by_version =
        BlockRequest::by_version(chain_id, latest_block.block_identifier.index);
    let latest_block_by_version = rosetta_client
        .block(&request_latest_by_version)
        .await
        .unwrap()
        .block
        .unwrap();
    let request_latest_by_hash =
        BlockRequest::by_hash(chain_id, latest_block.block_identifier.hash.clone());
    let latest_block_by_hash = rosetta_client
        .block(&request_latest_by_hash)
        .await
        .unwrap()
        .block
        .unwrap();

    assert_eq!(latest_block, latest_block_by_version);
    assert_eq!(latest_block_by_hash, latest_block_by_version);

    // And querying latest again should get yet another transaction in the future
    let newer_block = rosetta_client
        .block(&request_latest)
        .await
        .unwrap()
        .block
        .unwrap();
    assert!(newer_block.block_identifier.index > latest_block.block_identifier.index);
    assert!(newer_block.timestamp > latest_block.timestamp);
}

/// Try for 2 seconds to get a response.  This handles the fact that it's starting async
async fn try_until_ok<F, Fut, T>(function: F) -> anyhow::Result<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = anyhow::Result<T>>,
{
    let mut result = Err(anyhow::Error::msg("Failed to get response"));
    for _ in 1..10 {
        result = function().await;
        if result.is_ok() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    result
}
