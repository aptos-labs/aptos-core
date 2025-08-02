// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::smoke_test_environment::SwarmBuilder;
use anyhow::anyhow;
use aptos::{
    account::create::DEFAULT_FUNDED_COINS,
    common::types::GasOptions,
    test::{CliTestFramework, INVALID_ACCOUNT},
};
use aptos_cached_packages::aptos_stdlib;
use aptos_config::{config::ApiConfig, utils::get_available_port};
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519Signature},
    HashValue, PrivateKey,
};
use aptos_forge::{AptosPublicInfo, LocalSwarm, Node, NodeExt, Swarm};
use aptos_gas_schedule::{AptosGasParameters, FromOnChainGasSchedule};
use aptos_genesis::builder::InitConfigFn;
use aptos_global_constants::GAS_UNIT_PRICE;
use aptos_rest_client::{
    aptos_api_types::{TransactionOnChainData, UserTransaction},
    Response, Transaction,
};
use aptos_rosetta::{
    client::RosettaClient,
    common::{native_coin, BlockHash, BLOCKCHAIN, Y2K_MS},
    types::{
        AccountBalanceRequest, AccountBalanceResponse, AccountIdentifier, BlockIdentifier,
        BlockRequest, BlockResponse, NetworkIdentifier, NetworkRequest, Operation,
        OperationStatusType, OperationType, PartialBlockIdentifier, TransactionIdentifier,
        TransactionType, STAKING_CONTRACT_MODULE, SWITCH_OPERATOR_WITH_SAME_COMMISSION_FUNCTION,
    },
    ROSETTA_VERSION,
};
use aptos_sdk::{transaction_builder::TransactionFactory, types::LocalAccount};
use aptos_types::{
    account_address::AccountAddress,
    account_config::CORE_CODE_ADDRESS,
    chain_id::ChainId,
    on_chain_config::{GasScheduleV2, OnChainRandomnessConfig},
    transaction::SignedTransaction,
};
use serde_json::json;
use std::{
    collections::{BTreeMap, HashSet},
    future::Future,
    str::FromStr,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{task::JoinHandle, time::Instant};

const EPOCH_DURATION_S: u64 = 5;
const DEFAULT_TRANSFER_AMOUNT: u64 = 20;
const DEFAULT_MAX_WAIT_S: u64 = 5;
const DEFAULT_INTERVAL_MS: u64 = 100;
static DEFAULT_MAX_WAIT_DURATION: Duration = Duration::from_secs(DEFAULT_MAX_WAIT_S);
static DEFAULT_INTERVAL_DURATION: Duration = Duration::from_millis(DEFAULT_INTERVAL_MS);

async fn setup_simple_test(
    num_accounts: usize,
) -> (
    LocalSwarm,
    CliTestFramework,
    JoinHandle<anyhow::Result<()>>,
    RosettaClient,
) {
    setup_test(
        num_accounts,
        Arc::new(|_, config, _| {
            config.indexer_db_config.enable_transaction = true;
        }),
    )
    .await
}

async fn setup_test(
    num_accounts: usize,
    config_fn: InitConfigFn,
) -> (
    LocalSwarm,
    CliTestFramework,
    JoinHandle<anyhow::Result<()>>,
    RosettaClient,
) {
    // Start the validator
    let (swarm, cli, faucet) = SwarmBuilder::new_local(1)
        .with_init_genesis_config(Arc::new(|genesis_config| {
            genesis_config.epoch_duration_secs = EPOCH_DURATION_S;
            genesis_config.randomness_config_override = Some(OnChainRandomnessConfig::Off);
        }))
        .with_init_config(config_fn)
        .with_aptos()
        .build_with_cli(num_accounts)
        .await;
    let validator = swarm.validators().next().unwrap();

    // And the rosetta server
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
        ..Default::default()
    };

    // Start the server
    let _rosetta = aptos_rosetta::bootstrap_async(
        swarm.chain_id(),
        api_config,
        Some(aptos_rest_client::Client::new(
            validator.rest_api_endpoint(),
        )),
        HashSet::new(),
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
async fn test_block_transactions() {
    const NUM_TXNS_PER_PAGE: u16 = 2;

    let (swarm, cli, _faucet, rosetta_client) = setup_test(
        2,
        Arc::new(|_, config, _| config.api.max_transactions_page_size = NUM_TXNS_PER_PAGE),
    )
    .await;

    let account_1 = cli.account_id(0);
    let chain_id = swarm.chain_id();

    // At time 0, there should be 0 balance
    let response = get_balance(
        &rosetta_client,
        chain_id,
        AccountIdentifier::base_account(account_1),
        Some(0),
    )
    .await
    .unwrap();
    assert_eq!(response.block_identifier, BlockIdentifier {
        index: 0,
        hash: BlockHash::new(chain_id, 0).to_string(),
    });

    // First fund account 1 with lots more gas
    cli.fund_account(0, Some(DEFAULT_FUNDED_COINS * 10))
        .await
        .unwrap();
    let response = cli.transfer_coins(0, 1, 100, None).await.unwrap();

    let validator = swarm.validators().next().unwrap();
    let rest_client = validator.rest_client();
    let height = rest_client
        .get_block_by_version_bcs(response.version, false)
        .await
        .unwrap()
        .into_inner()
        .block_height;

    let response = rosetta_client
        .block(&BlockRequest::by_index(swarm.chain_id(), height))
        .await
        .unwrap();

    // There is only one user transaction, so the other one should be dropped
    assert_eq!(1, response.block.transactions.len());
}

#[tokio::test]
async fn test_network() {
    let (swarm, _, _, rosetta_client) = setup_simple_test(1).await;
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
    let status = try_until_ok_default(|| rosetta_client.network_status(&request))
        .await
        .unwrap();
    assert!(status.current_block_timestamp >= Y2K_MS);
    assert_eq!(
        BlockIdentifier {
            index: 0,
            hash: BlockHash::new(chain_id, 0).to_string(),
        },
        status.genesis_block_identifier
    );
    assert_eq!(
        status.genesis_block_identifier,
        status.oldest_block_identifier,
    );

    // Wrong blockchain should fail
    let request = NetworkRequest {
        network_identifier: NetworkIdentifier {
            blockchain: "eth".to_string(),
            network: chain_id.to_string(),
        },
    };
    rosetta_client
        .network_status(&request)
        .await
        .expect_err("Should not work with wrong blockchain name");

    // Wrong network should fail
    let request = NetworkRequest {
        network_identifier: NetworkIdentifier::from(ChainId::new(22)),
    };
    rosetta_client
        .network_status(&request)
        .await
        .expect_err("Should not work with wrong network chain id");
}

#[tokio::test]
async fn test_account_balance() {
    let (swarm, cli, _faucet, rosetta_client) = setup_simple_test(3).await;

    let account_1 = cli.account_id(0);
    let account_2 = cli.account_id(1);
    let account_3 = cli.account_id(2);
    let chain_id = swarm.chain_id();
    swarm
        .aptos_public_info()
        .sync_root_account_sequence_number()
        .await;

    let mut account_4 = swarm
        .aptos_public_info()
        .create_and_fund_user_account(10_000_000_000)
        .await
        .unwrap();

    // At time 0, there should be no balance
    let response = get_balance(
        &rosetta_client,
        chain_id,
        AccountIdentifier::base_account(account_1),
        Some(0),
    )
    .await
    .unwrap();
    assert_eq!(response.block_identifier, BlockIdentifier {
        index: 0,
        hash: BlockHash::new(chain_id, 0).to_string(),
    });

    // First fund account 1 with lots more gas
    cli.fund_account(0, Some(DEFAULT_FUNDED_COINS * 2))
        .await
        .unwrap();

    let mut account_1_balance = DEFAULT_FUNDED_COINS * 3;
    let mut account_2_balance = DEFAULT_FUNDED_COINS;
    // At some time both accounts should exist with initial amounts
    try_until_ok(DEFAULT_MAX_WAIT_DURATION, DEFAULT_INTERVAL_DURATION, || {
        account_has_balance(
            &rosetta_client,
            chain_id,
            AccountIdentifier::base_account(account_1),
            account_1_balance,
            0,
        )
    })
    .await
    .unwrap();
    try_until_ok_default(|| {
        account_has_balance(
            &rosetta_client,
            chain_id,
            AccountIdentifier::base_account(account_2),
            account_2_balance,
            0,
        )
    })
    .await
    .unwrap();

    // Send money, and expect the gas and fees to show up accordingly
    const TRANSFER_AMOUNT: u64 = 5000;
    let response = cli
        .transfer_coins(0, 1, TRANSFER_AMOUNT, None)
        .await
        .unwrap();
    account_1_balance -= TRANSFER_AMOUNT + response.gas_used * response.gas_unit_price;
    account_2_balance += TRANSFER_AMOUNT;
    account_has_balance(
        &rosetta_client,
        chain_id,
        AccountIdentifier::base_account(account_1),
        account_1_balance,
        1,
    )
    .await
    .unwrap();
    account_has_balance(
        &rosetta_client,
        chain_id,
        AccountIdentifier::base_account(account_2),
        account_2_balance,
        0,
    )
    .await
    .unwrap();

    // Failed transaction spends gas
    let _ = cli
        .transfer_invalid_addr(
            0,
            u64::MAX,
            Some(GasOptions {
                gas_unit_price: None,
                max_gas: Some(1000),
                expiration_secs: 30,
            }),
        )
        .await
        .unwrap_err();

    // Make a bad transaction, which will cause gas to be spent but no transfer
    let validator = swarm.validators().next().unwrap();
    let rest_client = validator.rest_client();
    let txns = rest_client
        .get_account_ordered_transactions(account_1, None, None)
        .await
        .unwrap()
        .into_inner();
    let failed_txn = txns.last().unwrap();
    if let Transaction::UserTransaction(txn) = failed_txn {
        account_1_balance -= txn.request.gas_unit_price.0 * txn.info.gas_used.0;
        account_has_balance(
            &rosetta_client,
            chain_id,
            AccountIdentifier::base_account(account_1),
            account_1_balance,
            2,
        )
        .await
        .unwrap();
    }

    // Check that the balance hasn't changed (and should be 0) in the invalid account
    account_has_balance(
        &rosetta_client,
        chain_id,
        AccountIdentifier::base_account(AccountAddress::from_hex_literal(INVALID_ACCOUNT).unwrap()),
        0,
        0,
    )
    .await
    .unwrap();

    // Let's now check the staking balance with the original staking contract, it should not be supported
    cli.fund_account(2, Some(10_000_000)).await.unwrap();
    cli.initialize_stake_owner(2, 1_000_000, None, None)
        .await
        .unwrap();
    account_has_balance(
        &rosetta_client,
        chain_id,
        AccountIdentifier::total_stake_account(account_3),
        1_000_000,
        1,
    )
    .await
    .expect_err("Original staking contract is not supported");

    create_staking_contract(
        &swarm.aptos_public_info(),
        &mut account_4,
        account_1,
        account_2,
        1_000_000,
        10,
        1,
    )
    .await;

    account_has_balance(
        &rosetta_client,
        chain_id,
        AccountIdentifier::total_stake_account(account_4.address()),
        1_000_000,
        1,
    )
    .await
    .unwrap();

    account_has_balance(
        &rosetta_client,
        chain_id,
        AccountIdentifier::active_stake_account(account_4.address()),
        1_000_000,
        1,
    )
    .await
    .unwrap();

    unlock_stake(
        &swarm.aptos_public_info(),
        &mut account_4,
        account_1,
        1_000,
        2,
    )
    .await;

    // Since unlock_stake was initiated, 1000 APT should be in pending inactive state until lockup ends
    account_has_balance(
        &rosetta_client,
        chain_id,
        AccountIdentifier::pending_inactive_stake_account(account_4.address()),
        1_000,
        2,
    )
    .await
    .unwrap();

    account_has_balance(
        &rosetta_client,
        chain_id,
        AccountIdentifier::inactive_stake_account(account_4.address()),
        0,
        2,
    )
    .await
    .unwrap();

    /* TODO: Support operator stake account in the future
    account_has_balance(
        &rosetta_client,
        chain_id,
        AccountIdentifier::operator_stake_account(account_4.address(), account_1),
        1_000_000,
        1,
    )
    .await
    .unwrap();*/
}

async fn create_staking_contract(
    info: &AptosPublicInfo,
    account: &mut LocalAccount,
    operator: AccountAddress,
    voter: AccountAddress,
    amount: u64,
    commission_percentage: u64,
    sequence_number: u64,
) -> Response<Transaction> {
    let staking_contract_creation = info
        .transaction_factory()
        .payload(aptos_stdlib::staking_contract_create_staking_contract(
            operator,
            voter,
            amount,
            commission_percentage,
            vec![],
        ))
        .sequence_number(sequence_number);

    let txn = account.sign_with_transaction_builder(staking_contract_creation);
    info.client().submit_and_wait(&txn).await.unwrap()
}

async fn unlock_stake(
    info: &AptosPublicInfo,
    account: &mut LocalAccount,
    operator: AccountAddress,
    amount: u64,
    sequence_number: u64,
) -> Response<Transaction> {
    let unlock_stake = info
        .transaction_factory()
        .payload(aptos_stdlib::staking_contract_unlock_stake(
            operator, amount,
        ))
        .sequence_number(sequence_number);

    let txn = account.sign_with_transaction_builder(unlock_stake);
    info.client().submit_and_wait(&txn).await.unwrap()
}

async fn create_delegation_pool(
    info: &AptosPublicInfo,
    account: &mut LocalAccount,
    commission_percentage: u64,
    sequence_number: u64,
) -> Response<Transaction> {
    let delegation_pool_creation = info
        .transaction_factory()
        .payload(aptos_stdlib::delegation_pool_initialize_delegation_pool(
            commission_percentage,
            vec![],
        ))
        .sequence_number(sequence_number);

    let txn = account.sign_with_transaction_builder(delegation_pool_creation);
    info.client().submit_and_wait(&txn).await.unwrap()
}

async fn account_has_balance(
    rosetta_client: &RosettaClient,
    chain_id: ChainId,
    account_identifier: AccountIdentifier,
    expected_balance: u64,
    expected_sequence_number: u64,
) -> anyhow::Result<u64> {
    let response = get_balance(rosetta_client, chain_id, account_identifier.clone(), None).await?;
    assert_eq!(
        expected_sequence_number,
        response.metadata.sequence_number.0
    );

    if response.balances.iter().any(|amount| {
        amount.currency == native_coin() && amount.value == expected_balance.to_string()
    }) {
        Ok(response.block_identifier.index)
    } else {
        Err(anyhow!(
            "Failed to find account {:?} with {} {:?}, received {:?}",
            account_identifier,
            expected_balance,
            native_coin(),
            response
        ))
    }
}

async fn get_balance(
    rosetta_client: &RosettaClient,
    chain_id: ChainId,
    account_identifier: AccountIdentifier,
    index: Option<u64>,
) -> anyhow::Result<AccountBalanceResponse> {
    let request = AccountBalanceRequest {
        network_identifier: chain_id.into(),
        account_identifier,
        block_identifier: Some(PartialBlockIdentifier { index, hash: None }),
        currencies: Some(vec![native_coin()]),
    };
    try_until_ok_default(|| rosetta_client.account_balance(&request)).await
}

async fn wait_for_rosetta_block(node_clients: &NodeClients<'_>, block_height: u64) {
    // Wait until the Rosetta service is ready
    let request = NetworkRequest {
        network_identifier: node_clients.network.clone(),
    };

    loop {
        let status = try_until_ok_default(|| node_clients.rosetta_client.network_status(&request))
            .await
            .unwrap();
        if status.current_block_identifier.index >= block_height {
            break;
        }
    }
}

#[tokio::test]
async fn test_transfer() {
    let (swarm, cli, _faucet, rosetta_client) = setup_simple_test(1).await;
    let chain_id = swarm.chain_id();
    let client = swarm.aptos_public_info().client().clone();
    let sender = cli.account_id(0);
    let receiver = AccountAddress::from_hex_literal("0xBEEF").unwrap();
    let sender_private_key = cli.private_key(0);
    let sender_balance = client
        .view_apt_account_balance(sender)
        .await
        .unwrap()
        .into_inner();
    let network = NetworkIdentifier::from(chain_id);
    let node_clients = NodeClients {
        rosetta_client: &rosetta_client,
        rest_client: &client,
        network: &network,
    };
    wait_for_rosetta_block(&node_clients, 2).await;

    // Attempt to transfer more than balance to another user (should fail)
    simple_transfer_and_wait(
        &node_clients,
        sender_private_key,
        receiver,
        sender_balance + 200,
    )
    .await
    .expect_err("Should fail simulation since we can't transfer more than balance coins");

    // Attempt to transfer more than balance to another user (should fail)
    let transaction_factory = TransactionFactory::new(chain_id)
        // We purposely don't set gas unit price here so the builder uses the default.
        // Note that the default is different in tests. See here:
        // config/global-constants/src/lib.rs
        .with_gas_unit_price(GAS_UNIT_PRICE)
        .with_max_gas_amount(1000);
    let txn_payload = aptos_stdlib::aptos_account_transfer(receiver, 100);
    let unsigned_transaction = transaction_factory
        .payload(txn_payload)
        .sender(sender)
        .sequence_number(0)
        .build();
    let signed_transaction = SignedTransaction::new(
        unsigned_transaction,
        sender_private_key.public_key(),
        Ed25519Signature::dummy_signature(),
    );

    let simulation_txn = client
        .simulate_bcs(&signed_transaction)
        .await
        .expect("Should succeed getting gas estimate")
        .into_inner();
    let gas_usage = simulation_txn.info.gas_used() * GAS_UNIT_PRICE;
    let max_sent = sender_balance - gas_usage;

    // Attempt to transfer more than balance - gas to another user (should fail)
    simple_transfer_and_wait(&node_clients, sender_private_key, receiver, max_sent + 1)
        .await
        .expect_err("Should fail simulation since we can't transfer more than balance + gas coins");

    // Attempt to transfer more than balance - gas to another user (should fail)
    let node_clients = NodeClients {
        rosetta_client: &rosetta_client,
        rest_client: &client,
        network: &network,
    };
    simple_transfer_and_wait(&node_clients, sender_private_key, receiver, max_sent)
        .await
        .expect("Should succeed transfer");

    // Sender balance should be 0
    assert_eq!(
        client
            .view_apt_account_balance(sender)
            .await
            .unwrap()
            .into_inner(),
        0
    );
    // Receiver should be sent coins
    assert_eq!(
        client
            .view_apt_account_balance(receiver)
            .await
            .unwrap()
            .into_inner(),
        max_sent
    );
}

/// This test tests all of Rosetta's functionality from the read side in one go.  Since
/// it's block based and it needs time to run, we do all the checks in a single test.
#[tokio::test]
async fn test_block() {
    let (swarm, cli, _faucet, rosetta_client) = setup_simple_test(5).await;
    let chain_id = swarm.chain_id();
    let validator = swarm.validators().next().unwrap();
    let rest_client = validator.rest_client();

    // Mapping of account to block and balance mappings
    let mut balances = BTreeMap::<AccountAddress, BTreeMap<u64, i128>>::new();

    // Wait until the Rosetta service is ready
    let network = NetworkIdentifier::from(chain_id);
    let node_clients = NodeClients {
        rosetta_client: &rosetta_client,
        rest_client: &rest_client,
        network: &network,
    };
    wait_for_rosetta_block(&node_clients, 2).await;

    // Do some transfers
    let account_id_0 = cli.account_id(0);
    let account_id_1 = cli.account_id(1);
    let account_id_2 = cli.account_id(2);
    let account_id_3 = cli.account_id(3);

    // TODO(greg): revisit after fixing gas estimation
    cli.fund_account(0, Some(100000000)).await.unwrap();
    cli.fund_account(1, Some(6500000)).await.unwrap();
    cli.fund_account(2, Some(500000)).await.unwrap();
    cli.fund_account(3, Some(200000)).await.unwrap();

    // Get minimum gas price
    let gas_schedule: GasScheduleV2 = rest_client
        .get_account_resource_bcs(CORE_CODE_ADDRESS, "0x1::gas_schedule::GasScheduleV2")
        .await
        .unwrap()
        .into_inner();
    let feature_version = gas_schedule.feature_version;
    let gas_params = AptosGasParameters::from_on_chain_gas_schedule(
        &gas_schedule.into_btree_map(),
        feature_version,
    )
    .unwrap();
    let min_gas_price = u64::from(gas_params.vm.txn.min_price_per_gas_unit);

    let private_key_0 = cli.private_key(0);
    let private_key_1 = cli.private_key(1);
    let private_key_2 = cli.private_key(2);
    let private_key_3 = cli.private_key(3);

    let seq_no_0 = simple_transfer_and_wait(
        &node_clients,
        private_key_0,
        account_id_1,
        DEFAULT_TRANSFER_AMOUNT,
    )
    .await
    .unwrap()
    .request
    .sequence_number
    .0;
    simple_transfer_and_wait(
        &node_clients,
        private_key_1,
        account_id_0,
        DEFAULT_TRANSFER_AMOUNT,
    )
    .await
    .unwrap();
    transfer_and_wait(
        &node_clients,
        private_key_0,
        account_id_0,
        DEFAULT_TRANSFER_AMOUNT,
        None,
        Some(seq_no_0 + 1),
        None,
        None,
    )
    .await
    .unwrap();
    // Create a new account via transfer
    simple_transfer_and_wait(
        &node_clients,
        private_key_2,
        AccountAddress::from_hex_literal(INVALID_ACCOUNT).unwrap(),
        DEFAULT_TRANSFER_AMOUNT,
    )
    .await
    .unwrap();
    let seq_no_3 = transfer_and_wait(
        &node_clients,
        private_key_3,
        account_id_0,
        DEFAULT_TRANSFER_AMOUNT,
        None,
        None,
        Some(2000000),
        Some(min_gas_price),
    )
    .await
    .unwrap()
    .request
    .sequence_number
    .0;

    // Create another account via command
    create_account_and_wait(
        &node_clients,
        private_key_3,
        AccountAddress::from_hex_literal("0x99").unwrap(),
        None,
        Some(seq_no_3 + 1),
        None,
        None,
    )
    .await
    .unwrap();

    transfer_and_wait(
        &node_clients,
        private_key_1,
        account_id_3,
        DEFAULT_TRANSFER_AMOUNT,
        None,
        // Test the default behavior
        None,
        None,
        Some(min_gas_price + 1),
    )
    .await
    .unwrap();

    // This one will fail because expiration is in the past
    transfer_and_wait(
        &node_clients,
        private_key_3,
        AccountAddress::ONE,
        DEFAULT_TRANSFER_AMOUNT,
        Some(Duration::from_secs(0)),
        None,
        None,
        None,
    )
    .await
    .unwrap_err();

    // This one will fail because gas is too low
    transfer_and_wait(
        &node_clients,
        private_key_3,
        AccountAddress::ONE,
        DEFAULT_TRANSFER_AMOUNT,
        None,
        None,
        Some(1),
        None,
    )
    .await
    .unwrap_err();

    // Add a ton of coins, and set an operator
    cli.fund_account(3, Some(10_000_000)).await.unwrap();
    cli.create_stake_pool(3, 3, 1, 1_000_000, 0).await.unwrap();

    // Set the operator
    set_operator_and_wait(
        &node_clients,
        private_key_3,
        Some(account_id_3),
        account_id_1,
    )
    .await
    .expect("Set operator should work!");

    // Also fail to set an operator (since the operator already changed)
    set_operator_and_wait(
        &node_clients,
        private_key_3,
        Some(account_id_3),
        account_id_1,
    )
    .await
    .unwrap_err();

    // Test native stake pool and reset lockup support
    const MIL_APT: u64 = 100000000000000;
    cli.fund_account(2, Some(10 * MIL_APT)).await.unwrap();
    create_stake_pool_and_wait(
        &node_clients,
        private_key_2,
        Some(account_id_3),
        Some(account_id_2),
        Some(MIL_APT),
        Some(5),
    )
    .await
    .expect("Should successfully create stake pool");

    // TODO: Verify lockup time changes

    // Reset lockup
    reset_lockup_and_wait(&node_clients, private_key_2, Some(account_id_3))
        .await
        .expect("Should successfully reset lockup");

    // Update commission
    update_commission_and_wait(&node_clients, private_key_2, Some(account_id_3), Some(50))
        .await
        .expect("Should successfully update commission");

    // Successfully, and fail setting a voter
    set_voter_and_wait(
        &node_clients,
        private_key_3,
        Some(account_id_3),
        account_id_1,
    )
    .await
    .expect_err("Set voter shouldn't work with the wrong operator!");
    set_voter_and_wait(
        &node_clients,
        private_key_3,
        Some(account_id_1),
        account_id_1,
    )
    .await
    .expect("Set voter should work!");

    // Unlock stake
    unlock_stake_and_wait(&node_clients, private_key_2, Some(account_id_3), Some(10))
        .await
        .expect("Should successfully unlock stake");

    // Failed distribution with wrong staker
    distribute_staking_rewards_and_wait(&node_clients, private_key_3, account_id_2, account_id_3)
        .await
        .expect_err("Staker has no staking contracts.");

    let final_txn = distribute_staking_rewards_and_wait(
        &node_clients,
        private_key_3,
        account_id_3,
        account_id_2,
    )
    .await
    .expect("Distribute staking rewards should work!");

    let final_block_to_check = rest_client
        .get_block_by_version(final_txn.info.version.0, false)
        .await
        .expect("Should be able to get block info for completed txns");

    // Check a couple blocks past the final transaction to check more txns
    let final_block_height = final_block_to_check.into_inner().block_height.0 + 2;

    // TODO: Track total supply?
    // TODO: Check account balance block hashes?
    // TODO: Handle multiple coin types

    // Wait until the Rosetta service is ready
    wait_for_rosetta_block(&node_clients, final_block_height).await;

    // Now we have to watch all the changes
    let mut current_version = 0;
    let mut previous_block_index = 0;
    let mut block_hashes = HashSet::new();
    for block_height in 0..final_block_height {
        let request = BlockRequest::by_index(chain_id, block_height);
        let response: BlockResponse = rosetta_client
            .block(&request)
            .await
            .expect("Should be able to get blocks that are already known");
        let block = response.block;
        let actual_block = rest_client
            .get_block_by_height_bcs(block_height, true)
            .await
            .expect("Should be able to get block for a known block")
            .into_inner();

        assert_eq!(
            block.block_identifier.index, block_height,
            "The block should match the requested block"
        );
        assert_eq!(
            block.block_identifier.hash,
            BlockHash::new(chain_id, block_height).to_string(),
            "Block hash should match chain_id-block_height"
        );
        assert_eq!(
            block.parent_block_identifier.index, previous_block_index,
            "Parent block index should be previous block"
        );
        assert_eq!(
            block.parent_block_identifier.hash,
            BlockHash::new(chain_id, previous_block_index).to_string(),
            "Parent block hash should be previous block chain_id-block_height"
        );
        assert!(
            block_hashes.insert(block.block_identifier.hash.clone()),
            "Block hash was repeated {}",
            block.block_identifier.hash
        );

        // It's only greater or equal because microseconds are cut off
        let expected_timestamp = if block_height == 0 {
            Y2K_MS
        } else {
            actual_block.block_timestamp.saturating_div(1000)
        };
        assert_eq!(
            expected_timestamp, block.timestamp,
            "Block timestamp should match actual timestamp but in ms"
        );

        // TODO: double check that all transactions do show with the flag, and that all expected txns
        // are shown without the flag

        let actual_txns = actual_block
            .transactions
            .as_ref()
            .expect("Every actual block should have transactions");
        parse_block_transactions(&block, &mut balances, actual_txns, &mut current_version).await;

        // Keep track of the previous
        previous_block_index = block_height;
    }

    // Reconcile and ensure all balances are calculated correctly
    check_balances(&rosetta_client, chain_id, balances).await;
}

/// Parse the transactions in each block
async fn parse_block_transactions(
    block: &aptos_rosetta::types::Block,
    balances: &mut BTreeMap<AccountAddress, BTreeMap<u64, i128>>,
    actual_txns: &[TransactionOnChainData],
    current_version: &mut u64,
) {
    let mut txn_hashes = HashSet::new();
    for transaction in block.transactions.iter() {
        let txn_metadata = &transaction.metadata;
        let txn_version = txn_metadata.version.0;
        let cur_version = *current_version;
        assert!(
            txn_version >= cur_version,
            "Transaction version {} must be greater than previous {}",
            txn_version,
            cur_version
        );

        let actual_txn = actual_txns
            .iter()
            .find(|txn| txn.version == txn_version)
            .expect("There should be the transaction in the actual block");
        let actual_txn_info = &actual_txn.info;

        // Ensure transaction identifier is correct
        let txn_hash = transaction.transaction_identifier.hash.clone();
        assert_eq!(
            format!("{:x}", actual_txn_info.transaction_hash()),
            txn_hash,
            "Transaction hash should match the actual hash"
        );

        assert!(
            txn_hashes.insert(txn_hash.clone()),
            "Transaction hash was repeated {}",
            txn_hash
        );

        // Ensure the status is correct
        assert_eq!(txn_metadata.failed, !actual_txn_info.status().is_success());
        assert_eq!(
            txn_metadata.vm_status,
            format!("{:?}", actual_txn_info.status())
        );

        // Type specific checks
        match txn_metadata.transaction_type {
            TransactionType::Genesis => {
                // For this test, there should only be one genesis
                assert_eq!(0, cur_version);
                assert!(matches!(
                    actual_txn.transaction,
                    aptos_types::transaction::Transaction::GenesisTransaction(_)
                ));
            },
            TransactionType::User => {
                assert!(matches!(
                    actual_txn.transaction,
                    aptos_types::transaction::Transaction::UserTransaction(_)
                ));
                // Must have a gas fee
                assert!(!transaction.operations.is_empty());
            },
            TransactionType::BlockMetadata => {
                assert!(matches!(
                    actual_txn.transaction,
                    aptos_types::transaction::Transaction::BlockMetadata(_)
                ));
                assert!(transaction.operations.is_empty());
            },
            TransactionType::BlockMetadataExt => {
                assert!(matches!(
                    actual_txn.transaction,
                    aptos_types::transaction::Transaction::BlockMetadataExt(_)
                ));
                assert!(transaction.operations.is_empty());
            },
            TransactionType::StateCheckpoint => {
                assert!(matches!(
                    actual_txn.transaction,
                    aptos_types::transaction::Transaction::StateCheckpoint(_)
                ));
                assert!(transaction.operations.is_empty());
            },
            TransactionType::BlockEpilogue => {
                assert!(matches!(
                    actual_txn.transaction,
                    aptos_types::transaction::Transaction::BlockEpilogue(_)
                ));
                assert!(transaction.operations.is_empty());
            },
            TransactionType::Validator => {
                assert!(matches!(
                    actual_txn.transaction,
                    aptos_types::transaction::Transaction::ValidatorTransaction(_)
                ));
                assert!(transaction.operations.is_empty());
            },
            TransactionType::ScheduledTransaction => {
                assert!(matches!(
                    actual_txn.transaction,
                    aptos_types::transaction::Transaction::ScheduledTransaction(_)
                ));
                assert!(transaction.operations.is_empty());
            },
        }

        parse_operations(
            block.block_identifier.index,
            balances,
            transaction,
            actual_txn,
        )
        .await;

        for (_, account_balance) in balances.iter() {
            if let Some(amount) = account_balance.get(&cur_version) {
                assert!(*amount >= 0, "Amount shouldn't be negative!")
            }
        }

        // Increment to next version
        *current_version = txn_version + 1;
    }
}

/// Parse the individual operations in a transaction
async fn parse_operations(
    block_height: u64,
    balances: &mut BTreeMap<AccountAddress, BTreeMap<u64, i128>>,
    transaction: &aptos_rosetta::types::Transaction,
    actual_txn: &TransactionOnChainData,
) {
    // If there are no operations, then there is no gas operation
    let mut has_gas_op = false;
    for (expected_index, operation) in transaction.operations.iter().enumerate() {
        assert_eq!(expected_index as u64, operation.operation_identifier.index);

        // Gas transaction is always last
        let status = operation.status().expect("Should have an operation status");
        let operation_type = operation
            .operation_type()
            .expect("Operation type should be known");
        let actual_successful = actual_txn.info.status().is_success();

        // Iterate through every operation, keeping track of balances
        match operation_type {
            OperationType::CreateAccount => {
                // Initialize state for a new account
                let account = operation
                    .account()
                    .expect("Account address should be parsable");

                if actual_successful {
                    assert_eq!(OperationStatusType::Success, status);
                    let account_balances = balances.entry(account).or_default();

                    if account_balances.is_empty() {
                        account_balances.insert(block_height, 0i128);
                    } else {
                        panic!("Account already has a balance when being created!");
                    }
                } else {
                    assert_eq!(
                        OperationStatusType::Failure,
                        status,
                        "Failed transaction should have failed create account operation"
                    );
                }
            },
            OperationType::Deposit => {
                let account = operation
                    .account()
                    .expect("Account address should be parsable");

                if actual_successful {
                    assert_eq!(OperationStatusType::Success, status);
                    let account_balances = balances.entry(account).or_insert_with(|| {
                        let mut map = BTreeMap::new();
                        map.insert(block_height, 0);
                        map
                    });
                    let (_, latest_balance) = account_balances.iter().last().unwrap();
                    assert_eq!(
                        operation.currency().unwrap(),
                        &native_coin(),
                        "Balance should be the native coin"
                    );
                    let delta = operation.amount().unwrap();

                    // Add with panic on overflow in case of too high of a balance
                    let new_balance = *latest_balance + delta;
                    account_balances.insert(block_height, new_balance);
                } else {
                    assert_eq!(
                        OperationStatusType::Failure,
                        status,
                        "Failed transaction should have failed deposit operation"
                    );
                }
            },
            OperationType::Withdraw => {
                // Gas is always successful
                if actual_successful {
                    assert_eq!(OperationStatusType::Success, status);
                    let account = operation
                        .account()
                        .expect("Account address should be parsable");

                    let account_balances = balances.entry(account).or_insert_with(|| {
                        let mut map = BTreeMap::new();
                        map.insert(block_height, 0);
                        map
                    });
                    let (_, latest_balance) = account_balances.iter().last().unwrap();
                    assert_eq!(
                        operation.currency().unwrap(),
                        &native_coin(),
                        "Balance should be the native coin"
                    );
                    let delta = operation.amount().unwrap();
                    let new_balance = *latest_balance + delta;
                    account_balances.insert(block_height, new_balance);
                } else {
                    assert_eq!(
                        OperationStatusType::Failure,
                        status,
                        "Failed transaction should have failed withdraw operation"
                    );
                }
            },
            OperationType::StakingReward => {
                let account = operation
                    .account()
                    .expect("Account address should be parsable");

                if actual_successful {
                    assert_eq!(OperationStatusType::Success, status);
                    let account_balances = balances.entry(account).or_insert_with(|| {
                        let mut map = BTreeMap::new();
                        map.insert(block_height, 0);
                        map
                    });
                    let (_, latest_balance) = account_balances.iter().last().unwrap();
                    assert_eq!(
                        operation.currency().unwrap(),
                        &native_coin(),
                        "Balance should be the native coin"
                    );

                    // Add with panic on overflow in case of too high of a balance
                    let delta = operation.amount().unwrap();
                    let new_balance = *latest_balance + delta;
                    account_balances.insert(block_height, new_balance);
                } else {
                    assert_eq!(
                        OperationStatusType::Failure,
                        status,
                        "Failed transaction should have failed stake reward operation"
                    );
                }
            },
            OperationType::SetOperator => {
                if actual_successful {
                    assert_eq!(
                        OperationStatusType::Success,
                        status,
                        "Successful transaction should have successful set operator operation"
                    );
                } else {
                    assert_eq!(
                        OperationStatusType::Failure,
                        status,
                        "Failed transaction should have failed set operator operation"
                    );
                }

                // Check that operator was set the same
                if let aptos_types::transaction::Transaction::UserTransaction(ref txn) =
                    actual_txn.transaction
                {
                    if let aptos_types::transaction::TransactionPayload::EntryFunction(
                        ref payload,
                    ) = txn.payload()
                    {
                        let actual_operator_address: AccountAddress = match (
                            *payload.module().address(),
                            payload.module().name().as_str(),
                            payload.function().as_str(),
                        ) {
                            (
                                AccountAddress::ONE,
                                STAKING_CONTRACT_MODULE,
                                SWITCH_OPERATOR_WITH_SAME_COMMISSION_FUNCTION,
                            ) => bcs::from_bytes(payload.args().last().unwrap()).unwrap(),
                            (
                                AccountAddress::ONE,
                                STAKING_CONTRACT_MODULE,
                                "create_staking_contract",
                            ) => bcs::from_bytes(payload.args().first().unwrap()).unwrap(),
                            _ => panic!("Unsupported entry function for set operator! {:?}", txn),
                        };

                        let operator = operation.new_operator().unwrap();
                        assert_eq!(actual_operator_address, operator)
                    } else {
                        panic!("Not an entry function");
                    }
                } else {
                    panic!("Not a user transaction");
                }
            },
            OperationType::SetVoter => {
                if actual_successful {
                    assert_eq!(
                        OperationStatusType::Success,
                        status,
                        "Successful transaction should have successful set voter operation"
                    );
                } else {
                    assert_eq!(
                        OperationStatusType::Failure,
                        status,
                        "Failed transaction should have failed set voter operation"
                    );
                }

                // Check that voter was set the same
                if let aptos_types::transaction::Transaction::UserTransaction(ref txn) =
                    actual_txn.transaction
                {
                    if let aptos_types::transaction::TransactionPayload::EntryFunction(
                        ref payload,
                    ) = txn.payload()
                    {
                        let actual_voter_address: AccountAddress =
                            bcs::from_bytes(payload.args().first().unwrap()).unwrap();
                        let voter = operation.new_voter().unwrap();
                        assert_eq!(actual_voter_address, voter)
                    } else {
                        panic!("Not an entry function");
                    }
                } else {
                    panic!("Not a user transaction");
                }
            },
            OperationType::Fee => {
                has_gas_op = true;
                assert_eq!(OperationStatusType::Success, status);
                let account = operation
                    .account()
                    .expect("There should be an account in a fee operation");

                let account_balances = balances.entry(account).or_insert_with(|| {
                    let mut map = BTreeMap::new();
                    map.insert(block_height, 0);
                    map
                });
                let (_, latest_balance) = account_balances.iter().last().unwrap();
                assert_eq!(
                    operation.currency().unwrap(),
                    &native_coin(),
                    "Balance should be the native coin"
                );
                let delta = operation.amount().unwrap();

                // Subtract with panic on overflow in case of a negative balance
                let new_balance = *latest_balance + delta;
                account_balances.insert(block_height, new_balance);
                match actual_txn.transaction {
                    aptos_types::transaction::Transaction::UserTransaction(ref txn) => {
                        assert_eq!(
                            actual_txn
                                .info
                                .gas_used()
                                .saturating_mul(txn.gas_unit_price()),
                            delta.unsigned_abs() as u64,
                            "Gas operation should always match gas used * gas unit price"
                        )
                    },
                    _ => {
                        panic!("Gas transactions should be user transactions!")
                    },
                };
            },
            OperationType::ResetLockup => {
                if actual_successful {
                    assert_eq!(
                        OperationStatusType::Success,
                        status,
                        "Successful transaction should have successful reset lockup operation"
                    );
                } else {
                    assert_eq!(
                        OperationStatusType::Failure,
                        status,
                        "Failed transaction should have failed reset lockup operation"
                    );
                }

                // Check that reset lockup was set the same
                if let aptos_types::transaction::Transaction::UserTransaction(ref txn) =
                    actual_txn.transaction
                {
                    if let aptos_types::transaction::TransactionPayload::EntryFunction(
                        ref payload,
                    ) = txn.payload()
                    {
                        let actual_operator_address: AccountAddress =
                            bcs::from_bytes(payload.args().first().unwrap()).unwrap();
                        let operator = operation.operator().unwrap();
                        assert_eq!(actual_operator_address, operator)
                    } else {
                        panic!("Not an entry function");
                    }
                } else {
                    panic!("Not a user transaction");
                }
            },
            OperationType::UpdateCommission => {
                if actual_successful {
                    assert_eq!(
                        OperationStatusType::Success,
                        status,
                        "Successful transaction should have successful update commission operation"
                    );
                } else {
                    assert_eq!(
                        OperationStatusType::Failure,
                        status,
                        "Failed transaction should have failed update commission operation"
                    );
                }

                // Check that update commmission was set the same
                if let aptos_types::transaction::Transaction::UserTransaction(ref txn) =
                    actual_txn.transaction
                {
                    if let aptos_types::transaction::TransactionPayload::EntryFunction(
                        ref payload,
                    ) = txn.payload()
                    {
                        let actual_operator_address: AccountAddress =
                            bcs::from_bytes(payload.args().first().unwrap()).unwrap();
                        let operator = operation
                            .metadata
                            .as_ref()
                            .unwrap()
                            .operator
                            .as_ref()
                            .unwrap()
                            .account_address()
                            .unwrap();
                        assert_eq!(actual_operator_address, operator);

                        let new_commission = operation
                            .metadata
                            .as_ref()
                            .unwrap()
                            .commission_percentage
                            .as_ref()
                            .unwrap()
                            .0;
                        let actual_new_commission: u64 =
                            bcs::from_bytes(payload.args().get(1).unwrap()).unwrap();
                        assert_eq!(actual_new_commission, new_commission);
                    } else {
                        panic!("Not an entry function");
                    }
                } else {
                    panic!("Not a user transaction");
                }
            },
            OperationType::InitializeStakePool => {
                if actual_successful {
                    assert_eq!(
                        OperationStatusType::Success,
                        status,
                        "Successful transaction should have successful initialize stake pool operation"
                    );
                } else {
                    assert_eq!(
                        OperationStatusType::Failure,
                        status,
                        "Failed transaction should have failed initialize stake pool operation"
                    );
                }

                // Check that reset lockup was set the same
                if let aptos_types::transaction::Transaction::UserTransaction(ref txn) =
                    actual_txn.transaction
                {
                    if let aptos_types::transaction::TransactionPayload::EntryFunction(
                        ref payload,
                    ) = txn.payload()
                    {
                        let actual_operator_address: AccountAddress =
                            bcs::from_bytes(payload.args().first().unwrap()).unwrap();
                        let operator = operation.new_operator().unwrap();
                        assert_eq!(actual_operator_address, operator);

                        let actual_voter_address: AccountAddress =
                            bcs::from_bytes(payload.args().get(1).unwrap()).unwrap();
                        let voter = operation.new_voter().unwrap();
                        assert_eq!(actual_voter_address, voter);

                        let actual_stake_amount: u64 =
                            bcs::from_bytes(payload.args().get(2).unwrap()).unwrap();
                        let stake = operation.staked_balance().unwrap();
                        assert_eq!(actual_stake_amount, stake);

                        let commission_percentage = operation.commission_percentage().unwrap();
                        let actual_commission: u64 =
                            bcs::from_bytes(payload.args().get(3).unwrap()).unwrap();
                        assert_eq!(actual_commission, commission_percentage);
                    } else {
                        panic!("Not an entry function");
                    }
                } else {
                    panic!("Not a user transaction");
                }
            },
            OperationType::UnlockStake => {
                if actual_successful {
                    assert_eq!(
                        OperationStatusType::Success,
                        status,
                        "Successful transaction should have successful unlock stake operation"
                    );
                } else {
                    assert_eq!(
                        OperationStatusType::Failure,
                        status,
                        "Failed transaction should have failed unlock stake operation"
                    );
                }

                // Check that unlock stake was set the same
                if let aptos_types::transaction::Transaction::UserTransaction(ref txn) =
                    actual_txn.transaction
                {
                    if let aptos_types::transaction::TransactionPayload::EntryFunction(
                        ref payload,
                    ) = txn.payload()
                    {
                        let actual_operator_address: AccountAddress =
                            bcs::from_bytes(payload.args().first().unwrap()).unwrap();
                        let operator = operation.operator().unwrap();
                        assert_eq!(actual_operator_address, operator);

                        let actual_amount: u64 =
                            bcs::from_bytes(payload.args().get(1).unwrap()).unwrap();
                        let amount = operation.metadata_amount().unwrap();
                        assert_eq!(actual_amount, amount);
                    } else {
                        panic!("Not an entry function");
                    }
                } else {
                    panic!("Not a user transaction");
                }
            },
            OperationType::DistributeStakingRewards => {
                if actual_successful {
                    assert_eq!(
                        OperationStatusType::Success,
                        status,
                        "Successful transaction should have successful distribute operation"
                    );
                } else {
                    assert_eq!(
                        OperationStatusType::Failure,
                        status,
                        "Failed transaction should have failed distribute operation"
                    );
                }

                // Check that distribute was set the same
                if let aptos_types::transaction::Transaction::UserTransaction(ref txn) =
                    actual_txn.transaction
                {
                    if let aptos_types::transaction::TransactionPayload::EntryFunction(
                        ref payload,
                    ) = txn.payload()
                    {
                        let actual_staker_address: AccountAddress =
                            bcs::from_bytes(payload.args().first().unwrap()).unwrap();
                        let staker = operation.staker().unwrap();
                        assert_eq!(actual_staker_address, staker);

                        let actual_operator_address: AccountAddress =
                            bcs::from_bytes(payload.args().get(1).unwrap()).unwrap();
                        let operator = operation.operator().unwrap();
                        assert_eq!(actual_operator_address, operator);
                    } else {
                        panic!("Not an entry function");
                    }
                } else {
                    panic!("Not a user transaction");
                }
            },
            OperationType::AddDelegatedStake => {
                if actual_successful {
                    assert_eq!(
                        OperationStatusType::Success,
                        status,
                        "Successful transaction should have successful add_delegated_stake operation"
                    );
                } else {
                    assert_eq!(
                        OperationStatusType::Failure,
                        status,
                        "Failed transaction should have failed add_delegated_stake operation"
                    );
                }

                // Check that add stake was set the same
                if let aptos_types::transaction::Transaction::UserTransaction(ref txn) =
                    actual_txn.transaction
                {
                    if let aptos_types::transaction::TransactionPayload::EntryFunction(
                        ref payload,
                    ) = txn.payload()
                    {
                        let actual_pool_address: AccountAddress =
                            bcs::from_bytes(payload.args().first().unwrap()).unwrap();

                        let pool_address = operation
                            .metadata
                            .as_ref()
                            .unwrap()
                            .pool_address
                            .as_ref()
                            .unwrap()
                            .account_address()
                            .unwrap();

                        assert_eq!(actual_pool_address, pool_address);

                        let actual_amount: u64 =
                            bcs::from_bytes(payload.args().get(1).unwrap()).unwrap();

                        let amount = operation
                            .metadata
                            .as_ref()
                            .unwrap()
                            .amount
                            .as_ref()
                            .unwrap()
                            .0;

                        assert_eq!(actual_amount, amount);
                    } else {
                        panic!("Not an entry function");
                    }
                } else {
                    panic!("Not a user transaction");
                }
            },
            OperationType::UnlockDelegatedStake => {
                if actual_successful {
                    assert_eq!(
                        OperationStatusType::Success,
                        status,
                        "Successful transaction should have successful unlock_delegated_stake operation"
                    );
                } else {
                    assert_eq!(
                        OperationStatusType::Failure,
                        status,
                        "Failed transaction should have failed unlock_delegated_stake operation"
                    );
                }

                // Check that unlock stake was set the same
                if let aptos_types::transaction::Transaction::UserTransaction(ref txn) =
                    actual_txn.transaction
                {
                    if let aptos_types::transaction::TransactionPayload::EntryFunction(
                        ref payload,
                    ) = txn.payload()
                    {
                        let actual_pool_address: AccountAddress =
                            bcs::from_bytes(payload.args().first().unwrap()).unwrap();

                        let pool_address = operation
                            .metadata
                            .as_ref()
                            .unwrap()
                            .pool_address
                            .as_ref()
                            .unwrap()
                            .account_address()
                            .unwrap();

                        assert_eq!(actual_pool_address, pool_address);

                        let actual_amount: u64 =
                            bcs::from_bytes(payload.args().get(1).unwrap()).unwrap();

                        let amount = operation
                            .metadata
                            .as_ref()
                            .unwrap()
                            .amount
                            .as_ref()
                            .unwrap()
                            .0;

                        assert_eq!(actual_amount, amount);
                    } else {
                        panic!("Not an entry function");
                    }
                } else {
                    panic!("Not a user transaction");
                }
            },
            OperationType::WithdrawUndelegatedFunds => {
                if actual_successful {
                    assert_eq!(
                        OperationStatusType::Success,
                        status,
                        "Successful transaction should have successful distribute operation"
                    );
                } else {
                    assert_eq!(
                        OperationStatusType::Failure,
                        status,
                        "Failed transaction should have failed distribute operation"
                    );
                }
                if let aptos_types::transaction::Transaction::UserTransaction(ref txn) =
                    actual_txn.transaction
                {
                    if let aptos_types::transaction::TransactionPayload::EntryFunction(
                        ref payload,
                    ) = txn.payload()
                    {
                        let actual_pool_address: AccountAddress =
                            bcs::from_bytes(payload.args().first().unwrap()).unwrap();

                        let pool_address = operation
                            .metadata
                            .as_ref()
                            .unwrap()
                            .pool_address
                            .as_ref()
                            .unwrap()
                            .account_address()
                            .unwrap();

                        assert_eq!(actual_pool_address, pool_address);

                        let actual_amount: u64 =
                            bcs::from_bytes(payload.args().get(1).unwrap()).unwrap();

                        let amount = operation
                            .metadata
                            .as_ref()
                            .unwrap()
                            .amount
                            .as_ref()
                            .unwrap()
                            .0;

                        assert_eq!(actual_amount, amount);
                    } else {
                        panic!("Not an entry function");
                    }
                }
            },
        }
    }

    assert!(
        has_gas_op
            || transaction.metadata.transaction_type == TransactionType::Genesis
            || transaction.operations.is_empty(),
        "Must have a gas operation at least in a transaction except for Genesis",
    );
}

/// Check that all balances are correct with the account balance command from the blocks
async fn check_balances(
    rosetta_client: &RosettaClient,
    chain_id: ChainId,
    balances: BTreeMap<AccountAddress, BTreeMap<u64, i128>>,
) {
    // TODO: Check some random times that arent on changes?
    for (account, account_balances) in balances {
        for (block_height, expected_balance) in account_balances {
            // Block should match it's calculated balance
            let response = rosetta_client
                .account_balance(&AccountBalanceRequest {
                    network_identifier: NetworkIdentifier::from(chain_id),
                    account_identifier: AccountIdentifier::base_account(account),
                    block_identifier: Some(PartialBlockIdentifier {
                        index: Some(block_height),
                        hash: None,
                    }),
                    currencies: Some(vec![native_coin()]),
                })
                .await
                .unwrap();
            assert_eq!(
                block_height, response.block_identifier.index,
                "Block should be the one expected"
            );

            let balance = response.balances.first().unwrap();
            assert_eq!(
                balance.currency,
                native_coin(),
                "Balance should be the native coin"
            );
            assert_eq!(
                expected_balance,
                balance
                    .value
                    .parse::<u64>()
                    .expect("Should have a balance from account balance") as i128,
                "Expected {} to have a balance of {}, but was {} at block {}",
                account,
                expected_balance,
                balance.value,
                block_height
            );
        }
    }
}

#[tokio::test]
async fn test_invalid_transaction_gas_charged() {
    let (swarm, cli, _faucet, rosetta_client) = setup_simple_test(1).await;
    let chain_id = swarm.chain_id();

    // Make sure first that there's money to transfer
    cli.assert_account_balance_now(0, DEFAULT_FUNDED_COINS)
        .await;

    // Now let's see some transfers
    let _ = cli
        .transfer_invalid_addr(
            0,
            DEFAULT_FUNDED_COINS + 1,
            Some(GasOptions {
                gas_unit_price: None,
                max_gas: Some(1000),
                expiration_secs: 30,
            }),
        )
        .await
        .unwrap_err();

    let sender = cli.account_id(0);

    // Find failed transaction
    let validator = swarm.validators().next().unwrap();
    let rest_client = validator.rest_client();
    let txns = rest_client
        .get_account_ordered_transactions(sender, None, None)
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
        .get_block_by_version(txn_version, false)
        .await
        .unwrap()
        .into_inner();

    let block_with_transfer = rosetta_client
        .block(&BlockRequest::by_index(chain_id, block_info.block_height.0))
        .await
        .unwrap();
    let block_with_transfer = block_with_transfer.block;
    // Verify failed txn
    let rosetta_txn = block_with_transfer
        .transactions
        .iter()
        .find(|txn| txn.metadata.version.0 == txn_version)
        .unwrap();

    assert_failed_transfer_transaction(
        sender,
        AccountAddress::from_hex_literal(INVALID_ACCOUNT).unwrap(),
        DEFAULT_FUNDED_COINS + 1,
        actual_txn,
        rosetta_txn,
    );
}

fn assert_failed_transfer_transaction(
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

    let rosetta_txn_metadata = &rosetta_txn.metadata;
    assert_eq!(TransactionType::User, rosetta_txn_metadata.transaction_type);
    assert_eq!(actual_txn.info.version.0, rosetta_txn_metadata.version.0);
    // This should have 3, the deposit, withdraw, and fee
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
                },
                OperationType::Withdraw => {
                    seen_withdraw = true;
                    assert_withdraw(operation, transfer_amount, sender, actual_txn.info.success);
                },
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
            assert_gas(
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

fn assert_gas(operation: &Operation, expected_amount: u64, account: AccountAddress, success: bool) {
    assert_transfer(
        operation,
        OperationType::Fee,
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
        &AccountIdentifier::base_account(account),
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

struct NodeClients<'a> {
    pub rosetta_client: &'a RosettaClient,
    pub rest_client: &'a aptos_rest_client::Client,
    pub network: &'a NetworkIdentifier,
}

async fn create_account_and_wait(
    node_clients: &NodeClients<'_>,
    sender_key: &Ed25519PrivateKey,
    new_account: AccountAddress,
    txn_expiry_duration: Option<Duration>,
    sequence_number: Option<u64>,
    max_gas: Option<u64>,
    gas_unit_price: Option<u64>,
) -> Result<UserTransaction, ErrorWrapper> {
    submit_transaction(
        node_clients.rest_client,
        txn_expiry_duration.unwrap_or(DEFAULT_MAX_WAIT_DURATION),
        |expiry_time| {
            node_clients.rosetta_client.create_account(
                node_clients.network,
                sender_key,
                new_account,
                expiry_time,
                sequence_number,
                max_gas,
                gas_unit_price,
            )
        },
    )
    .await
}

async fn simple_transfer_and_wait(
    node_clients: &NodeClients<'_>,
    sender_key: &Ed25519PrivateKey,
    receiver: AccountAddress,
    amount: u64,
) -> Result<UserTransaction, ErrorWrapper> {
    transfer_and_wait(
        node_clients,
        sender_key,
        receiver,
        amount,
        None,
        None,
        None,
        None,
    )
    .await
}

async fn transfer_and_wait(
    node_clients: &NodeClients<'_>,
    sender_key: &Ed25519PrivateKey,
    receiver: AccountAddress,
    amount: u64,
    txn_expiry_duration: Option<Duration>,
    sequence_number: Option<u64>,
    max_gas: Option<u64>,
    gas_unit_price: Option<u64>,
) -> Result<UserTransaction, ErrorWrapper> {
    submit_transaction(
        node_clients.rest_client,
        txn_expiry_duration.unwrap_or(DEFAULT_MAX_WAIT_DURATION),
        |expiry_time| {
            node_clients.rosetta_client.transfer(
                node_clients.network,
                sender_key,
                receiver,
                amount,
                expiry_time,
                sequence_number,
                max_gas,
                gas_unit_price,
                native_coin(),
            )
        },
    )
    .await
}

async fn set_operator_and_wait(
    node_clients: &NodeClients<'_>,
    sender_key: &Ed25519PrivateKey,
    old_operator: Option<AccountAddress>,
    new_operator: AccountAddress,
) -> Result<UserTransaction, ErrorWrapper> {
    submit_transaction(
        node_clients.rest_client,
        DEFAULT_MAX_WAIT_DURATION,
        |expiry_time| {
            node_clients.rosetta_client.set_operator(
                node_clients.network,
                sender_key,
                old_operator,
                new_operator,
                expiry_time,
                None,
                None,
                None,
            )
        },
    )
    .await
}

async fn set_voter_and_wait(
    node_clients: &NodeClients<'_>,
    sender_key: &Ed25519PrivateKey,
    operator: Option<AccountAddress>,
    new_voter: AccountAddress,
) -> Result<UserTransaction, ErrorWrapper> {
    submit_transaction(
        node_clients.rest_client,
        DEFAULT_MAX_WAIT_DURATION,
        |expiry_time| {
            node_clients.rosetta_client.set_voter(
                node_clients.network,
                sender_key,
                operator,
                new_voter,
                expiry_time,
                None,
                None,
                None,
            )
        },
    )
    .await
}

async fn create_stake_pool_and_wait(
    node_clients: &NodeClients<'_>,
    sender_key: &Ed25519PrivateKey,
    operator: Option<AccountAddress>,
    voter: Option<AccountAddress>,
    stake_amount: Option<u64>,
    commission_percentage: Option<u64>,
) -> Result<UserTransaction, ErrorWrapper> {
    submit_transaction(
        node_clients.rest_client,
        DEFAULT_MAX_WAIT_DURATION,
        |expiry_time| {
            node_clients.rosetta_client.create_stake_pool(
                node_clients.network,
                sender_key,
                operator,
                voter,
                stake_amount,
                commission_percentage,
                expiry_time,
                None,
                None,
                None,
            )
        },
    )
    .await
}

async fn reset_lockup_and_wait(
    node_clients: &NodeClients<'_>,
    sender_key: &Ed25519PrivateKey,
    operator: Option<AccountAddress>,
) -> Result<UserTransaction, ErrorWrapper> {
    submit_transaction(
        node_clients.rest_client,
        DEFAULT_MAX_WAIT_DURATION,
        |expiry_time| {
            node_clients.rosetta_client.reset_lockup(
                node_clients.network,
                sender_key,
                operator,
                expiry_time,
                None,
                None,
                None,
            )
        },
    )
    .await
}

async fn update_commission_and_wait(
    node_clients: &NodeClients<'_>,
    sender_key: &Ed25519PrivateKey,
    operator: Option<AccountAddress>,
    new_commission_percentage: Option<u64>,
) -> Result<UserTransaction, ErrorWrapper> {
    submit_transaction(
        node_clients.rest_client,
        DEFAULT_MAX_WAIT_DURATION,
        |expiry_time| {
            node_clients.rosetta_client.update_commission(
                node_clients.network,
                sender_key,
                operator,
                new_commission_percentage,
                expiry_time,
                None,
                None,
                None,
            )
        },
    )
    .await
}

async fn unlock_stake_and_wait(
    node_clients: &NodeClients<'_>,
    sender_key: &Ed25519PrivateKey,
    operator: Option<AccountAddress>,
    amount: Option<u64>,
) -> Result<UserTransaction, ErrorWrapper> {
    submit_transaction(
        node_clients.rest_client,
        DEFAULT_MAX_WAIT_DURATION,
        |expiry_time| {
            node_clients.rosetta_client.unlock_stake(
                node_clients.network,
                sender_key,
                operator,
                amount,
                expiry_time,
                None,
                None,
                None,
            )
        },
    )
    .await
}

async fn distribute_staking_rewards_and_wait(
    node_clients: &NodeClients<'_>,
    sender_key: &Ed25519PrivateKey,
    operator: AccountAddress,
    staker: AccountAddress,
) -> Result<UserTransaction, ErrorWrapper> {
    submit_transaction(
        node_clients.rest_client,
        DEFAULT_MAX_WAIT_DURATION,
        |expiry_time| {
            node_clients.rosetta_client.distribute_staking_rewards(
                node_clients.network,
                sender_key,
                operator,
                staker,
                expiry_time,
                None,
                None,
                None,
            )
        },
    )
    .await
}

async fn submit_transaction<
    Fut: Future<Output = anyhow::Result<TransactionIdentifier>>,
    F: FnOnce(u64) -> Fut,
>(
    rest_client: &aptos_rest_client::Client,
    txn_expiry_duration: Duration,
    transaction_builder: F,
) -> Result<UserTransaction, ErrorWrapper> {
    let expiry_time = expiry_time(txn_expiry_duration);

    let txn_hash = transaction_builder(expiry_time.as_secs())
        .await
        .map_err(ErrorWrapper::BeforeSubmission)?
        .hash;
    wait_for_transaction(rest_client, expiry_time, txn_hash)
        .await
        .map_err(ErrorWrapper::AfterSubmission)
}

async fn wait_for_transaction(
    rest_client: &aptos_rest_client::Client,
    expiry_time: Duration,
    txn_hash: String,
) -> Result<UserTransaction, UserTransaction> {
    let hash_value = HashValue::from_str(&txn_hash).unwrap();
    let response = rest_client
        .wait_for_transaction_by_hash(
            hash_value,
            expiry_time.as_secs(),
            Some(DEFAULT_MAX_WAIT_DURATION),
            None,
        )
        .await;
    match response {
        Ok(response) => {
            if let Transaction::UserTransaction(txn) = response.into_inner() {
                Ok(txn)
            } else {
                panic!("Transaction is supposed to be a UserTransaction!")
            }
        },
        Err(_) => {
            if let Transaction::UserTransaction(txn) = rest_client
                .get_transaction_by_hash(hash_value)
                .await
                .unwrap()
                .into_inner()
            {
                Err(txn)
            } else {
                panic!("Failed transaction is supposed to be a UserTransaction!");
            }
        },
    }
}

fn expiry_time(txn_expiry_duration: Duration) -> Duration {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .saturating_add(txn_expiry_duration)
}

async fn add_delegated_stake_and_wait(
    rosetta_client: &RosettaClient,
    rest_client: &aptos_rest_client::Client,
    network_identifier: &NetworkIdentifier,
    sender_key: &Ed25519PrivateKey,
    pool_address: AccountAddress,
    amount: Option<u64>,
    txn_expiry_duration: Duration,
    sequence_number: Option<u64>,
    max_gas: Option<u64>,
    gas_unit_price: Option<u64>,
) -> Result<UserTransaction, ErrorWrapper> {
    let expiry_time = expiry_time(txn_expiry_duration);
    let txn_hash = rosetta_client
        .add_delegated_stake(
            network_identifier,
            sender_key,
            pool_address,
            amount,
            expiry_time.as_secs(),
            sequence_number,
            max_gas,
            gas_unit_price,
        )
        .await
        .map_err(ErrorWrapper::BeforeSubmission)?
        .hash;

    wait_for_transaction(rest_client, expiry_time, txn_hash)
        .await
        .map_err(ErrorWrapper::AfterSubmission)
}

async fn unlock_delegated_stake_and_wait(
    rosetta_client: &RosettaClient,
    rest_client: &aptos_rest_client::Client,
    network_identifier: &NetworkIdentifier,
    sender_key: &Ed25519PrivateKey,
    pool_address: AccountAddress,
    amount: Option<u64>,
    txn_expiry_duration: Duration,
    sequence_number: Option<u64>,
    max_gas: Option<u64>,
    gas_unit_price: Option<u64>,
) -> Result<UserTransaction, ErrorWrapper> {
    let expiry_time = expiry_time(txn_expiry_duration);
    let txn_hash = rosetta_client
        .unlock_delegated_stake(
            network_identifier,
            sender_key,
            pool_address,
            amount,
            expiry_time.as_secs(),
            sequence_number,
            max_gas,
            gas_unit_price,
        )
        .await
        .map_err(ErrorWrapper::BeforeSubmission)?
        .hash;
    wait_for_transaction(rest_client, expiry_time, txn_hash)
        .await
        .map_err(ErrorWrapper::AfterSubmission)
}

async fn withdraw_undelegated_stake_and_wait(
    rosetta_client: &RosettaClient,
    rest_client: &aptos_rest_client::Client,
    network_identifier: &NetworkIdentifier,
    sender_key: &Ed25519PrivateKey,
    pool_address: AccountAddress,
    amount: Option<u64>,
    txn_expiry_duration: Duration,
    sequence_number: Option<u64>,
    max_gas: Option<u64>,
    gas_unit_price: Option<u64>,
) -> Result<UserTransaction, ErrorWrapper> {
    let expiry_time = expiry_time(txn_expiry_duration);
    let txn_hash = rosetta_client
        .withdraw_undelegated_stake(
            network_identifier,
            sender_key,
            pool_address,
            amount,
            expiry_time.as_secs(),
            sequence_number,
            max_gas,
            gas_unit_price,
        )
        .await
        .map_err(ErrorWrapper::BeforeSubmission)?
        .hash;

    wait_for_transaction(rest_client, expiry_time, txn_hash)
        .await
        .map_err(ErrorWrapper::AfterSubmission)
}

#[tokio::test]
async fn test_delegation_pool_operations() {
    const NUM_TXNS_PER_PAGE: u16 = 2;

    let (swarm, cli, _, rosetta_client) = setup_test(
        2,
        Arc::new(|_, config, _| config.api.max_transactions_page_size = NUM_TXNS_PER_PAGE),
    )
    .await;

    // 20 APT
    let delegate_initial_balance = 20 * u64::pow(10, 8);
    cli.fund_account(0, Some(delegate_initial_balance))
        .await
        .unwrap();
    let delegate_account_private_key = cli.private_key(0);

    let chain_id = swarm.chain_id();
    let validator = swarm.validators().next().unwrap();
    let rest_client = validator.rest_client();
    let _request = NetworkRequest {
        network_identifier: NetworkIdentifier::from(chain_id),
    };
    let network_identifier = chain_id.into();

    let root_address = swarm.aptos_public_info().root_account().address();
    let root_sequence_number = swarm
        .aptos_public_info()
        .client()
        .get_account_bcs(root_address)
        .await
        .unwrap()
        .into_inner()
        .sequence_number();

    swarm
        .aptos_public_info()
        .root_account()
        .set_sequence_number(root_sequence_number);

    let mut delegation_pool_creator_account = swarm
        .aptos_public_info()
        .create_and_fund_user_account(1_000_000_000_000_000)
        .await
        .unwrap();

    let res = create_delegation_pool(
        &swarm.aptos_public_info(),
        &mut delegation_pool_creator_account,
        10,
        1,
    )
    .await;

    let (tx, _) = res.into_parts();
    let tx_serialized = json!(tx);
    let mut pool_address_str = "";

    if let Some(changes) = tx_serialized["changes"].as_array() {
        for change in changes {
            if change["data"]["type"] == "0x1::delegation_pool::DelegationPool" {
                pool_address_str = change["address"].as_str().unwrap();
                break;
            }
        }
    }
    let pool_address = AccountAddress::from_hex_literal(pool_address_str).unwrap();

    // Must stake at least 11 APT
    let staked_amount = 11 * u64::pow(10, 8);

    add_delegated_stake_and_wait(
        &rosetta_client,
        &rest_client,
        &network_identifier,
        delegate_account_private_key,
        pool_address,
        Some(staked_amount),
        Duration::from_secs(5),
        None,
        None,
        None,
    )
    .await
    .expect("Should successfully add delegated stake");

    unlock_delegated_stake_and_wait(
        &rosetta_client,
        &rest_client,
        &network_identifier,
        delegate_account_private_key,
        pool_address,
        Some(staked_amount),
        Duration::from_secs(5),
        None,
        None,
        None,
    )
    .await
    .expect("Should successfully unlock delegated stake");

    let final_txn = withdraw_undelegated_stake_and_wait(
        &rosetta_client,
        &rest_client,
        &network_identifier,
        delegate_account_private_key,
        pool_address,
        Some(staked_amount),
        Duration::from_secs(5),
        None,
        None,
        None,
    )
    .await
    .expect("Should successfully withdraw undelegated");

    let final_block_to_check = rest_client
        .get_block_by_version(final_txn.info.version.0, false)
        .await
        .expect("Should be able to get block info for completed txns");

    // Check a couple blocks past the final transaction to check more txns
    let final_block_height = final_block_to_check.into_inner().block_height.0 + 2;

    // TODO: Track total supply?
    // TODO: Check account balance block hashes?
    // TODO: Handle multiple coin types

    // Wait until the Rosetta service is ready
    let request = NetworkRequest {
        network_identifier: NetworkIdentifier::from(chain_id),
    };

    loop {
        let status = try_until_ok_default(|| rosetta_client.network_status(&request))
            .await
            .unwrap();
        if status.current_block_identifier.index >= final_block_height {
            break;
        }
    }

    // Now we have to watch all the changes
    let mut current_version = 0;
    let mut balances = BTreeMap::<AccountAddress, BTreeMap<u64, i128>>::new();
    let mut previous_block_index = 0;
    let mut block_hashes = HashSet::new();
    for block_height in 0..final_block_height {
        let request = BlockRequest::by_index(chain_id, block_height);
        let response: BlockResponse = rosetta_client
            .block(&request)
            .await
            .expect("Should be able to get blocks that are already known");
        let block = response.block;
        let actual_block = rest_client
            .get_block_by_height_bcs(block_height, true)
            .await
            .expect("Should be able to get block for a known block")
            .into_inner();

        assert_eq!(
            block.block_identifier.index, block_height,
            "The block should match the requested block"
        );
        assert_eq!(
            block.block_identifier.hash,
            BlockHash::new(chain_id, block_height).to_string(),
            "Block hash should match chain_id-block_height"
        );
        assert_eq!(
            block.parent_block_identifier.index, previous_block_index,
            "Parent block index should be previous block"
        );
        assert_eq!(
            block.parent_block_identifier.hash,
            BlockHash::new(chain_id, previous_block_index).to_string(),
            "Parent block hash should be previous block chain_id-block_height"
        );
        assert!(
            block_hashes.insert(block.block_identifier.hash.clone()),
            "Block hash was repeated {}",
            block.block_identifier.hash
        );

        // It's only greater or equal because microseconds are cut off
        let expected_timestamp = if block_height == 0 {
            Y2K_MS
        } else {
            actual_block.block_timestamp.saturating_div(1000)
        };
        assert_eq!(
            expected_timestamp, block.timestamp,
            "Block timestamp should match actual timestamp but in ms"
        );

        // TODO: double check that all transactions do show with the flag, and that all expected txns
        // are shown without the flag

        let actual_txns = actual_block
            .transactions
            .as_ref()
            .expect("Every actual block should have transactions");

        parse_block_transactions(&block, &mut balances, actual_txns, &mut current_version).await;

        // Keep track of the previous
        previous_block_index = block_height;
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum ErrorWrapper {
    BeforeSubmission(anyhow::Error),
    AfterSubmission(UserTransaction),
}
