// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::aptos_cli::setup_cli_test;
use aptos::{account::create::DEFAULT_FUNDED_COINS, test::CliTestFramework};
use aptos_config::{config::ApiConfig, utils::get_available_port};
use aptos_crypto::HashValue;
use aptos_rosetta::common::{BLOCKCHAIN, Y2K_SECS};
use aptos_rosetta::types::{
    AccountBalanceResponse, Block, BlockIdentifier, NetworkIdentifier, NetworkRequest,
    PartialBlockIdentifier,
};
use aptos_rosetta::{
    client::RosettaClient,
    common::native_coin,
    types::{AccountBalanceRequest, BlockRequest},
    ROSETTA_VERSION,
};
use aptos_types::account_address::AccountAddress;
use aptos_types::chain_id::ChainId;
use forge::{LocalSwarm, Node};
use std::{future::Future, time::Duration};
use tokio::task::JoinHandle;

pub async fn setup_test(
    num_nodes: usize,
    num_accounts: usize,
) -> (LocalSwarm, CliTestFramework, JoinHandle<()>, RosettaClient) {
    let (swarm, cli, faucet) = setup_cli_test(num_nodes).await;
    let validator = swarm.validators().next().unwrap();

    // And the client
    let rosetta_port = get_available_port();
    let rosetta_socket_addr = format!("127.0.0.1:{}", rosetta_port);
    let rosetta_url = format!("http://{}", rosetta_socket_addr.clone())
        .parse()
        .unwrap();
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
        Some(aptos_rest_client::Client::new(
            validator.rest_api_endpoint(),
        )),
    )
    .await
    .unwrap();

    // Create accounts
    for i in 0..num_accounts {
        cli.create_account_with_faucet(i).await.unwrap();
    }
    (swarm, cli, faucet, rosetta_client)
}

#[tokio::test]
async fn test_network() {
    let (swarm, _, _, rosetta_client) = setup_test(1, 1).await;
    let chain_id = swarm.chain_id();

    // We only support one network, this network
    let networks = rosetta_client.network_list().await.unwrap();
    assert_eq!(1, networks.network_identifiers.len());
    let network_id = networks.network_identifiers.first().unwrap();
    assert_eq!(BLOCKCHAIN, network_id.blockchain);
    assert_eq!(chain_id.to_string(), network_id.network);

    let request = NetworkRequest {
        network_identifier: NetworkIdentifier::from(chain_id),
    };
    let options = rosetta_client.network_options(&request).await.unwrap();
    assert_eq!(ROSETTA_VERSION, options.version.rosetta_version);

    // TODO: Check other options

    let request = NetworkRequest {
        network_identifier: NetworkIdentifier::from(chain_id),
    };
    let status = rosetta_client.network_status(&request).await.unwrap();
    assert!(status.current_block_identifier.index > 0);
    assert!(status.current_block_timestamp > Y2K_SECS);
    assert_eq!(
        BlockIdentifier {
            index: 0,
            hash: HashValue::zero().to_hex()
        },
        status.genesis_block_identifier
    );
    assert_eq!(
        Some(status.genesis_block_identifier),
        status.oldest_block_identifier,
    );
}

#[tokio::test]
async fn test_account_balance() {
    let (swarm, cli, _faucet, rosetta_client) = setup_test(1, 1).await;

    cli.create_account_with_faucet(0).await.unwrap();
    let account = CliTestFramework::account_id(0);
    let chain_id = swarm.chain_id();

    // At time 0, there should be 0 balance
    let response = get_balance(&rosetta_client, chain_id, account, 0)
        .await
        .unwrap();
    assert_eq!(
        response.block_identifier,
        BlockIdentifier {
            index: 0,
            hash: HashValue::zero().to_hex(),
        }
    );

    // At some time before version 100, the account should exist
    let mut successful_version = None;
    for i in 1..100 {
        let response = get_balance(&rosetta_client, chain_id, account, i)
            .await
            .unwrap();
        let amount = response.balances.first().unwrap();
        if amount.value == DEFAULT_FUNDED_COINS.to_string() {
            successful_version = Some(i);
            break;
        }
    }

    if successful_version.is_none() {
        panic!("Failed to find account balance increase")
    }

    // TODO: Send money
    // TODO: Fail request due to bad transaction
    // TODO: Receive money
    // TODO: Recieve money by faucet
}

async fn get_balance(
    rosetta_client: &RosettaClient,
    chain_id: ChainId,
    account: AccountAddress,
    index: u64,
) -> anyhow::Result<AccountBalanceResponse> {
    let request = AccountBalanceRequest {
        network_identifier: chain_id.into(),
        account_identifier: account.into(),
        block_identifier: Some(PartialBlockIdentifier {
            index: Some(index),
            hash: None,
        }),
        currencies: Some(vec![native_coin()]),
    };
    try_until_ok(|| rosetta_client.account_balance(&request)).await
}

#[tokio::test]
async fn test_block() {
    let (swarm, _cli, _faucet, rosetta_client) = setup_test(1, 0).await;
    let chain_id = swarm.chain_id();

    let request_genesis = BlockRequest::by_index(chain_id, 0);
    let by_version_response = try_until_ok(|| rosetta_client.block(&request_genesis))
        .await
        .unwrap();
    let genesis_block = by_version_response.block.unwrap();

    // Genesis txn should always have parent be same as block
    assert_eq!(
        genesis_block.block_identifier,
        genesis_block.parent_block_identifier
    );
    assert_eq!(
        HashValue::zero().to_hex(),
        genesis_block.block_identifier.hash,
    );
    assert_eq!(0, genesis_block.block_identifier.index);

    // Genesis timestamp is always Y2K
    assert_eq!(Y2K_SECS, genesis_block.timestamp);

    // There should only be the genesis transaction
    assert_eq!(1, genesis_block.transactions.len());
    let genesis_txn = genesis_block.transactions.first().unwrap();

    // Version should match as 0
    assert_eq!(
        0,
        genesis_txn.metadata.unwrap().version.0,
        "Genesis version"
    );

    // There should be at least one transfer in genesis
    assert!(!genesis_txn.operations.is_empty());

    // Get genesis txn by hash
    let request_genesis_by_hash =
        BlockRequest::by_hash(chain_id, genesis_block.block_identifier.hash.clone());
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

    // Block 1 is always a reconfig with exactly 1 txn
    let block_1 = get_block(&rosetta_client, chain_id, 1).await;
    assert_eq!(1, block_1.transactions.len());
    // Block metadata won't have operations
    assert!(block_1.transactions.first().unwrap().operations.is_empty());
    assert!(block_1.timestamp > genesis_block.timestamp);

    // Block 2 is always a standard block with 2 or more txns
    let block_2 = get_block(&rosetta_client, chain_id, 2).await;
    assert!(block_2.transactions.len() >= 2);
    // Block metadata won't have operations
    assert!(block_2.transactions.first().unwrap().operations.is_empty());
    // StateCheckpoint won't have operations
    assert!(block_2.transactions.last().unwrap().operations.is_empty());
    assert!(block_2.timestamp >= block_1.timestamp);

    // No input should give the latest version, not the genesis txn
    let request_latest = BlockRequest::latest(chain_id);
    let response = rosetta_client.block(&request_latest).await.unwrap();
    let latest_block = response.block.unwrap();

    // The latest block should always come after genesis
    assert!(latest_block.block_identifier.index >= block_2.block_identifier.index);
    assert!(latest_block.timestamp >= block_2.timestamp);

    // The parent should always be exactly one version before
    assert_eq!(
        latest_block.parent_block_identifier.index,
        latest_block.block_identifier.index - 1
    );

    // There should be at least 1 txn
    assert!(latest_block.transactions.len() > 1);

    // We should be able to query it again by hash or by version and it is the same
    let request_latest_by_version =
        BlockRequest::by_index(chain_id, latest_block.block_identifier.index);
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

    // Wait until we get a new block processed
    let network_request = NetworkRequest {
        network_identifier: NetworkIdentifier::from(chain_id),
    };
    while rosetta_client
        .network_status(&network_request)
        .await
        .unwrap()
        .current_block_identifier
        .index
        == latest_block.block_identifier.index
    {
        tokio::time::sleep(Duration::from_micros(10)).await
    }

    // And querying latest again should get yet another transaction in the future
    let newer_block = rosetta_client
        .block(&request_latest)
        .await
        .unwrap()
        .block
        .unwrap();
    assert!(newer_block.block_identifier.index >= latest_block.block_identifier.index);
    assert!(newer_block.timestamp >= latest_block.timestamp);
}

async fn get_block(rosetta_client: &RosettaClient, chain_id: ChainId, index: u64) -> Block {
    let rosetta_client = (*rosetta_client).clone();
    let request = BlockRequest::by_index(chain_id, index);
    try_until_ok(|| rosetta_client.block(&request))
        .await
        .unwrap()
        .block
        .unwrap()
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
