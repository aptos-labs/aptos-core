// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::smoke_test_environment::SwarmBuilder;
use anyhow::anyhow;
use aptos::common::types::{GasOptions, DEFAULT_GAS_UNIT_PRICE, DEFAULT_MAX_GAS};
use aptos::test::INVALID_ACCOUNT;
use aptos::{account::create::DEFAULT_FUNDED_COINS, test::CliTestFramework};
use aptos_config::{config::ApiConfig, utils::get_available_port};
use aptos_crypto::HashValue;
use aptos_rest_client::aptos_api_types::UserTransaction;
use aptos_rest_client::Transaction;
use aptos_rosetta::types::{
    AccountIdentifier, Operation, OperationStatusType, OperationType, TransactionType,
};
use aptos_rosetta::{
    client::RosettaClient,
    common::{native_coin, BLOCKCHAIN, Y2K_MS},
    types::{
        AccountBalanceRequest, AccountBalanceResponse, Block, BlockIdentifier, BlockRequest,
        NetworkIdentifier, NetworkRequest, PartialBlockIdentifier,
    },
    ROSETTA_VERSION,
};
use aptos_types::{account_address::AccountAddress, chain_id::ChainId};
use forge::{LocalSwarm, Node, NodeExt};
use std::str::FromStr;
use std::{future::Future, time::Duration};
use tokio::{task::JoinHandle, time::Instant};

const DEFAULT_MAX_WAIT_MS: u64 = 5000;
const DEFAULT_INTERVAL_MS: u64 = 100;
static DEFAULT_MAX_WAIT_DURATION: Duration = Duration::from_millis(DEFAULT_MAX_WAIT_MS);
static DEFAULT_INTERVAL_DURATION: Duration = Duration::from_millis(DEFAULT_INTERVAL_MS);

pub async fn setup_test(
    num_nodes: usize,
    num_accounts: usize,
) -> (LocalSwarm, CliTestFramework, JoinHandle<()>, RosettaClient) {
    let (swarm, cli, faucet) = SwarmBuilder::new_local(num_nodes)
        .with_aptos()
        .build_with_cli(num_accounts)
        .await;
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
        failpoints_enabled: false,
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

    // Ensure rosetta can take requests
    try_until_ok_default(|| rosetta_client.network_list())
        .await
        .unwrap();

    (swarm, cli, faucet, rosetta_client)
}

#[tokio::test]
async fn test_network() {
    let (swarm, _, _, rosetta_client) = setup_test(1, 1).await;
    let chain_id = swarm.chain_id();

    // We only support one network, this network
    let networks = try_until_ok_default(|| rosetta_client.network_list())
        .await
        .unwrap();
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
    assert!(status.current_block_timestamp > Y2K_MS);
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
    let (swarm, cli, _faucet, rosetta_client) = setup_test(1, 2).await;

    let account_1 = cli.account_id(0);
    let account_2 = cli.account_id(1);
    let chain_id = swarm.chain_id();

    // At time 0, there should be 0 balance
    let response = get_balance(&rosetta_client, chain_id, account_1, Some(0))
        .await
        .unwrap();
    assert_eq!(
        response.block_identifier,
        BlockIdentifier {
            index: 0,
            hash: HashValue::zero().to_hex(),
        }
    );

    // First fund account 1 with lots more gas
    cli.fund_account(0, Some(DEFAULT_FUNDED_COINS * 2))
        .await
        .unwrap();

    let mut account_1_balance = DEFAULT_FUNDED_COINS * 3;
    let mut account_2_balance = DEFAULT_FUNDED_COINS;
    // At some time both accounts should exist with initial amounts
    try_until_ok(Duration::from_secs(5), DEFAULT_INTERVAL_DURATION, || {
        account_has_balance(&rosetta_client, chain_id, account_1, account_1_balance)
    })
    .await
    .unwrap();
    try_until_ok_default(|| {
        account_has_balance(&rosetta_client, chain_id, account_2, account_2_balance)
    })
    .await
    .unwrap();

    // Send money, and expect the gas and fees to show up accordingly
    const TRANSFER_AMOUNT: u64 = 5000;
    let response = cli
        .transfer_coins(
            0,
            1,
            TRANSFER_AMOUNT,
            Some(GasOptions {
                gas_unit_price: DEFAULT_GAS_UNIT_PRICE * 2,
                max_gas: DEFAULT_MAX_GAS,
            }),
        )
        .await
        .unwrap();
    account_1_balance -= TRANSFER_AMOUNT + response.gas_used * response.gas_unit_price;
    account_2_balance += TRANSFER_AMOUNT;
    account_has_balance(&rosetta_client, chain_id, account_1, account_1_balance)
        .await
        .unwrap();
    account_has_balance(&rosetta_client, chain_id, account_2, account_2_balance)
        .await
        .unwrap();

    // Failed transaction spends gas
    let _ = cli
        .transfer_invalid_addr(
            0,
            TRANSFER_AMOUNT,
            Some(GasOptions {
                gas_unit_price: DEFAULT_GAS_UNIT_PRICE * 2,
                max_gas: DEFAULT_MAX_GAS,
            }),
        )
        .await
        .unwrap_err();

    // Make a bad transaction, which will cause gas to be spent but no transfer
    let validator = swarm.validators().next().unwrap();
    let rest_client = validator.rest_client();
    let txns = rest_client
        .get_account_transactions(account_1, None, None)
        .await
        .unwrap()
        .into_inner();
    let failed_txn = txns.last().unwrap();
    if let Transaction::UserTransaction(txn) = failed_txn {
        account_1_balance -= txn.request.gas_unit_price.0 * txn.info.gas_used.0;
        account_has_balance(&rosetta_client, chain_id, account_1, account_1_balance)
            .await
            .unwrap();
    }
}

async fn account_has_balance(
    rosetta_client: &RosettaClient,
    chain_id: ChainId,
    account: AccountAddress,
    expected_balance: u64,
) -> anyhow::Result<u64> {
    let response = get_balance(rosetta_client, chain_id, account, None).await?;
    if response.balances.iter().any(|amount| {
        amount.currency == native_coin() && amount.value == expected_balance.to_string()
    }) {
        Ok(response.block_identifier.index)
    } else {
        Err(anyhow!(
            "Failed to find account with {} {:?}, received {:?}",
            expected_balance,
            native_coin(),
            response
        ))
    }
}

async fn get_balance(
    rosetta_client: &RosettaClient,
    chain_id: ChainId,
    account: AccountAddress,
    index: Option<u64>,
) -> anyhow::Result<AccountBalanceResponse> {
    let request = AccountBalanceRequest {
        network_identifier: chain_id.into(),
        account_identifier: account.into(),
        block_identifier: Some(PartialBlockIdentifier { index, hash: None }),
        currencies: Some(vec![native_coin()]),
    };
    try_until_ok_default(|| rosetta_client.account_balance(&request)).await
}

#[tokio::test]
async fn test_block() {
    let (swarm, _cli, _faucet, rosetta_client) = setup_test(1, 2).await;
    let chain_id = swarm.chain_id();

    // Genesis by version
    let genesis_block = get_block(&rosetta_client, chain_id, 0).await;
    assert_genesis_block(&genesis_block);

    // Get genesis txn by hash
    let genesis_block_by_hash = get_block_by_hash(
        &rosetta_client,
        chain_id,
        genesis_block.block_identifier.hash.clone(),
    )
    .await;

    // Both blocks should be the same
    assert_eq!(
        genesis_block, genesis_block_by_hash,
        "Genesis by hash or by index should be the same"
    );

    // Responses should be idempotent
    let idempotent_block = get_block(&rosetta_client, chain_id, 0).await;
    assert_eq!(
        idempotent_block, genesis_block_by_hash,
        "Blocks should be idempotent"
    );

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
    let latest_block = rosetta_client
        .block(&request_latest)
        .await
        .unwrap()
        .block
        .unwrap();

    // The latest block should always come after genesis
    assert!(latest_block.block_identifier.index >= block_2.block_identifier.index);
    assert!(latest_block.timestamp >= block_2.timestamp);

    // The parent should always be exactly one version before
    assert_eq!(
        latest_block.parent_block_identifier.index,
        latest_block.block_identifier.index - 1
    );

    // There should be at least 1 txn
    assert!(!latest_block.transactions.is_empty());

    // We should be able to query it again by hash or by version and it is the same
    let latest_block_by_version = get_block(
        &rosetta_client,
        chain_id,
        latest_block.block_identifier.index,
    )
    .await;
    let latest_block_by_hash = get_block_by_hash(
        &rosetta_client,
        chain_id,
        latest_block.block_identifier.hash.clone(),
    )
    .await;

    assert_eq!(latest_block, latest_block_by_version);
    assert_eq!(latest_block_by_hash, latest_block_by_version);

    // Wait until we get a new block processed
    let network_request = NetworkRequest {
        network_identifier: NetworkIdentifier::from(chain_id),
    };

    let start = Instant::now();
    let max_wait = Duration::from_secs(1);
    let mut successful = false;
    while start.elapsed() < max_wait {
        if rosetta_client
            .network_status(&network_request)
            .await
            .unwrap()
            .current_block_identifier
            .index
            == latest_block.block_identifier.index
        {
            successful = true;
            break;
        }
        tokio::time::sleep(Duration::from_micros(10)).await
    }

    assert!(successful, "Failed to get the next block");

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

#[tokio::test]
async fn test_block_transactions() {
    let (swarm, cli, _faucet, rosetta_client) = setup_test(1, 2).await;
    let chain_id = swarm.chain_id();

    // Make sure first that there's money to transfer
    assert_eq!(DEFAULT_FUNDED_COINS, cli.account_balance(0).await.unwrap());
    assert_eq!(DEFAULT_FUNDED_COINS, cli.account_balance(1).await.unwrap());

    // Now let's see some transfers
    const TRANSFER_AMOUNT: u64 = 5000;
    let response = cli
        .transfer_coins(
            0,
            1,
            TRANSFER_AMOUNT,
            Some(GasOptions {
                gas_unit_price: DEFAULT_GAS_UNIT_PRICE * 2,
                max_gas: DEFAULT_MAX_GAS,
            }),
        )
        .await
        .unwrap();
    let sender = cli.account_id(0);
    let receiver = cli.account_id(1);

    let transfer_version = response.version;

    let validator = swarm.validators().next().unwrap();
    let rest_client = validator.rest_client();
    let block_info = rest_client
        .get_block_info(transfer_version)
        .await
        .unwrap()
        .into_inner();

    let block_with_transfer = rosetta_client
        .block(&BlockRequest::by_index(chain_id, block_info.block_height))
        .await
        .unwrap();
    let block_with_transfer = block_with_transfer.block.unwrap();

    // Ensure the block is all good
    assert_eq!(
        block_with_transfer.timestamp,
        block_info.block_timestamp.saturating_div(1000)
    );
    assert_eq!(
        block_with_transfer.block_identifier.index,
        block_info.block_height
    );
    assert_eq!(
        block_with_transfer.block_identifier.hash,
        format!("{:x}", block_info.block_hash)
    );
    assert_eq!(
        block_with_transfer.parent_block_identifier.index,
        block_info.block_height.saturating_sub(1)
    );

    // Verify individual txns
    let num_txns = block_info
        .end_version
        .saturating_sub(block_info.start_version) as usize;
    let actual_txns = rest_client
        .get_transactions(Some(block_info.start_version), Some(num_txns as u16))
        .await
        .unwrap()
        .into_inner();
    for i in 0..num_txns {
        let expected_version = block_info.start_version.saturating_add(i as u64);
        let actual_txn = actual_txns.get(i).unwrap();
        let block_txn = block_with_transfer.transactions.get(i).unwrap();

        // Identifiers should match the txn
        let block_txn_metadata = block_txn.metadata.unwrap();
        assert_eq!(block_txn_metadata.version.0, expected_version);
        assert_eq!(
            block_txn.transaction_identifier.hash,
            format!("{:x}", actual_txn.transaction_info().unwrap().hash)
        );

        // first transaction has to be block metadata
        if expected_version == block_info.start_version {
            assert_eq!(
                TransactionType::BlockMetadata,
                block_txn_metadata.transaction_type
            );

            // No operations occur in block metadata txn
            assert!(block_txn.operations.is_empty());
        } else if expected_version == transfer_version {
            if let Transaction::UserTransaction(actual_txn) = actual_txn {
                assert_transfer_transaction(
                    sender,
                    receiver,
                    TRANSFER_AMOUNT,
                    actual_txn,
                    block_txn,
                )
            } else {
                panic!("Must be a user txn");
            }
        } else if let Transaction::StateCheckpointTransaction(actual_txn) = actual_txn {
            // If we have a state checkpoint it should be at the end of the block and have no operations
            assert_eq!(
                TransactionType::StateCheckpoint,
                block_txn_metadata.transaction_type
            );
            assert_eq!(block_txn_metadata.version.0, block_info.end_version);
            assert!(block_txn.operations.is_empty());
            assert_eq!(
                actual_txn.info.hash.to_string(),
                block_txn.transaction_identifier.hash
            );
        }
    }
}

#[tokio::test]
async fn test_invalid_transaction_gas_charged() {
    let (swarm, cli, _faucet, rosetta_client) = setup_test(1, 1).await;
    let chain_id = swarm.chain_id();

    // Make sure first that there's money to transfer
    assert_eq!(DEFAULT_FUNDED_COINS, cli.account_balance(0).await.unwrap());

    // Now let's see some transfers
    const TRANSFER_AMOUNT: u64 = 5000;
    let _ = cli
        .transfer_invalid_addr(
            0,
            TRANSFER_AMOUNT,
            Some(GasOptions {
                gas_unit_price: DEFAULT_GAS_UNIT_PRICE * 2,
                max_gas: DEFAULT_MAX_GAS,
            }),
        )
        .await
        .unwrap_err();

    let sender = cli.account_id(0);

    // Find failed transaction
    let validator = swarm.validators().next().unwrap();
    let rest_client = validator.rest_client();
    let txns = rest_client
        .get_account_transactions(sender, None, None)
        .await
        .unwrap()
        .into_inner();
    let actual_txn = txns.iter().find(|txn| !txn.success()).unwrap();
    let actual_txn = if let Transaction::UserTransaction(txn) = actual_txn {
        txn
    } else {
        panic!("Not a user transaction");
    };
    let txn_version = actual_txn.info.version.0;

    let block_info = rest_client
        .get_block_info(txn_version)
        .await
        .unwrap()
        .into_inner();

    let block_with_transfer = rosetta_client
        .block(&BlockRequest::by_index(chain_id, block_info.block_height))
        .await
        .unwrap();
    let block_with_transfer = block_with_transfer.block.unwrap();
    // Verify failed txn
    let rosetta_txn = block_with_transfer
        .transactions
        .get(txn_version.saturating_sub(block_info.start_version) as usize)
        .unwrap();

    assert_transfer_transaction(
        sender,
        AccountAddress::from_hex_literal(INVALID_ACCOUNT).unwrap(),
        TRANSFER_AMOUNT,
        actual_txn,
        rosetta_txn,
    );
}

fn assert_transfer_transaction(
    sender: AccountAddress,
    receiver: AccountAddress,
    transfer_amount: u64,
    actual_txn: &UserTransaction,
    rosetta_txn: &aptos_rosetta::types::Transaction,
) {
    // Check the transaction
    assert_eq!(
        format!("{:x}", actual_txn.info.hash),
        rosetta_txn.transaction_identifier.hash
    );

    let rosetta_txn_metadata = rosetta_txn.metadata.as_ref().unwrap();
    assert_eq!(TransactionType::User, rosetta_txn_metadata.transaction_type);
    assert_eq!(actual_txn.info.version.0, rosetta_txn_metadata.version.0);
    assert_eq!(rosetta_txn.operations.len(), 3);

    // Check the operations
    let mut seen_deposit = false;
    let mut seen_withdraw = false;
    for (i, operation) in rosetta_txn.operations.iter().enumerate() {
        assert_eq!(i as u64, operation.operation_identifier.index);
        if !seen_deposit && !seen_withdraw {
            match OperationType::from_str(&operation.operation_type).unwrap() {
                OperationType::Deposit => {
                    seen_deposit = true;
                    assert_deposit(
                        operation,
                        transfer_amount,
                        receiver,
                        actual_txn.info.success,
                    );
                }
                OperationType::Withdraw => {
                    seen_withdraw = true;
                    assert_withdraw(operation, transfer_amount, sender, actual_txn.info.success);
                }
                _ => panic!("Shouldn't get any other operations"),
            }
        } else if !seen_deposit {
            seen_deposit = true;
            assert_deposit(
                operation,
                transfer_amount,
                receiver,
                actual_txn.info.success,
            );
        } else if !seen_withdraw {
            seen_withdraw = true;
            assert_withdraw(operation, transfer_amount, sender, actual_txn.info.success);
        } else {
            // Gas is always last
            assert_withdraw(
                operation,
                actual_txn.request.gas_unit_price.0 * actual_txn.info.gas_used.0,
                sender,
                true,
            );
        }
    }
}

fn assert_deposit(
    operation: &Operation,
    expected_amount: u64,
    account: AccountAddress,
    success: bool,
) {
    assert_transfer(
        operation,
        OperationType::Deposit,
        expected_amount.to_string(),
        account,
        success,
    );
}

fn assert_withdraw(
    operation: &Operation,
    expected_amount: u64,
    account: AccountAddress,
    success: bool,
) {
    assert_transfer(
        operation,
        OperationType::Withdraw,
        format!("-{}", expected_amount),
        account,
        success,
    );
}

fn assert_transfer(
    operation: &Operation,
    expected_type: OperationType,
    expected_amount: String,
    account: AccountAddress,
    success: bool,
) {
    assert_eq!(expected_type.to_string(), operation.operation_type);
    let amount = operation.amount.as_ref().unwrap();
    assert_eq!(native_coin(), amount.currency);
    assert_eq!(expected_amount, amount.value);
    assert_eq!(
        &AccountIdentifier::from(account),
        operation.account.as_ref().unwrap()
    );
    let expected_status = if success {
        OperationStatusType::Success
    } else {
        OperationStatusType::Failure
    }
    .to_string();
    assert_eq!(&expected_status, operation.status.as_ref().unwrap());
}

fn assert_genesis_block(block: &Block) {
    assert_eq!(
        block.block_identifier, block.parent_block_identifier,
        "The genesis block is also it's own parent"
    );
    assert_eq!(
        HashValue::zero().to_hex(),
        block.block_identifier.hash,
        "The genesis block hash is always 0s"
    );
    assert_eq!(
        0, block.block_identifier.index,
        "The genesis block index is always 0"
    );

    assert_eq!(
        Y2K_MS, block.timestamp,
        "The genesis timestamp should be Y2K seconds"
    );
    assert_eq!(
        1,
        block.transactions.len(),
        "The genesis block should be exactly 1 transaction"
    );

    let genesis_txn = block.transactions.first().unwrap();
    assert_eq!(
        0,
        genesis_txn.metadata.unwrap().version.0,
        "Genesis version should be 0"
    );
    assert_ne!(
        HashValue::zero().to_hex(),
        genesis_txn.transaction_identifier.hash,
        "Genesis should have a txn hash"
    );

    assert!(
        !genesis_txn.operations.is_empty(),
        "There should be at least one operation in genesis"
    );
}

async fn get_block(rosetta_client: &RosettaClient, chain_id: ChainId, index: u64) -> Block {
    let rosetta_client = (*rosetta_client).clone();
    let request = BlockRequest::by_index(chain_id, index);
    try_until_ok_default(|| rosetta_client.block(&request))
        .await
        .unwrap()
        .block
        .unwrap()
}

async fn get_block_by_hash(
    rosetta_client: &RosettaClient,
    chain_id: ChainId,
    hash: String,
) -> Block {
    let rosetta_client = (*rosetta_client).clone();
    let request = BlockRequest::by_hash(chain_id, hash);
    try_until_ok_default(|| rosetta_client.block(&request))
        .await
        .unwrap()
        .block
        .unwrap()
}

/// Try for 2 seconds to get a response.  This handles the fact that it's starting async
async fn try_until_ok_default<F, Fut, T>(function: F) -> anyhow::Result<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = anyhow::Result<T>>,
{
    try_until_ok(
        DEFAULT_MAX_WAIT_DURATION,
        DEFAULT_INTERVAL_DURATION,
        function,
    )
    .await
}

async fn try_until_ok<F, Fut, T>(
    total_wait: Duration,
    interval: Duration,
    function: F,
) -> anyhow::Result<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = anyhow::Result<T>>,
{
    let mut result = Err(anyhow::Error::msg("Failed to get response"));
    let start = Instant::now();
    while start.elapsed() < total_wait {
        result = function().await;
        if result.is_ok() {
            break;
        }
        tokio::time::sleep(interval).await;
    }

    result
}
