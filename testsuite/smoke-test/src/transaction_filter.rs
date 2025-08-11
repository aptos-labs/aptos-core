// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    smoke_test_environment::SwarmBuilder,
    utils::{create_test_accounts, execute_transactions},
};
use aptos_cached_packages::aptos_stdlib;
use aptos_config::config::{
    BatchTransactionFilterConfig, BlockTransactionFilterConfig, NodeConfig, TransactionFilterConfig,
};
use aptos_crypto::ed25519::{Ed25519PrivateKey, Ed25519PublicKey};
use aptos_forge::{LocalSwarm, NodeExt, Swarm};
use aptos_keygen::KeyGen;
use aptos_sdk::{
    crypto::{PrivateKey, SigningKey},
    types::{
        transaction::{authenticator::AuthenticationKey, SignedTransaction},
        LocalAccount,
    },
};
use aptos_transaction_filters::{
    batch_transaction_filter::{BatchTransactionFilter, BatchTransactionMatcher},
    block_transaction_filter::{BlockTransactionFilter, BlockTransactionMatcher},
    transaction_filter::{TransactionFilter, TransactionMatcher},
};
use aptos_types::on_chain_config::{
    ConsensusAlgorithmConfig, OnChainConsensusConfig, ValidatorTxnConfig, DEFAULT_WINDOW_SIZE,
};
use move_core_types::account_address::AccountAddress;
use std::sync::Arc;

#[tokio::test]
async fn test_consensus_block_filter() {
    // Generate a new key pair and sender address
    let (private_key, sender_address) = create_sender_account();

    // Create a new swarm with an inline consensus filter that denies transactions
    // from the sender, and disable quorum store (to ensure the filter is applied).
    let mut swarm = SwarmBuilder::new_local(3)
        .with_aptos()
        .with_init_config(Arc::new(move |_, config, _| {
            filter_inline_transactions(config, sender_address);
        }))
        .with_init_genesis_config(Arc::new(|genesis_config| {
            genesis_config.consensus_config = OnChainConsensusConfig::V4 {
                alg: ConsensusAlgorithmConfig::default_with_quorum_store_disabled(),
                vtxn: ValidatorTxnConfig::default_for_genesis(),
                window_size: DEFAULT_WINDOW_SIZE,
            };
        }))
        .build()
        .await;

    // Execute a few regular transactions and verify that they are processed correctly
    execute_test_transactions(&mut swarm).await;

    // Prepare a transaction from the sender address
    let transaction = create_transaction_from_sender(private_key, sender_address, &mut swarm).await;

    // Submit the transaction and wait for it to be processed
    let aptos_public_info = swarm.aptos_public_info();
    let response = aptos_public_info
        .client()
        .submit_and_wait(&transaction)
        .await;

    // Verify the transaction was dropped by the consensus filter
    let error = response.unwrap_err();
    assert!(error
        .to_string()
        .contains("Used to be pending and now not found. Transaction expired."));

    // Execute a few more transactions and verify that they are processed correctly
    execute_test_transactions(&mut swarm).await;
}

#[tokio::test]
async fn test_mempool_transaction_filter() {
    // Generate a new key pair and sender address
    let (private_key, sender_address) = create_sender_account();

    // Create a new swarm with a mempool filter that denies transactions from the sender
    let mut swarm = SwarmBuilder::new_local(3)
        .with_aptos()
        .with_init_config(Arc::new(move |_, config, _| {
            filter_mempool_transactions(config, sender_address);
        }))
        .build()
        .await;

    // Execute a few regular transactions and verify that they are processed correctly
    execute_test_transactions(&mut swarm).await;

    // Prepare a transaction from the sender address
    let transaction = create_transaction_from_sender(private_key, sender_address, &mut swarm).await;

    // Submit the transaction and wait for it to be processed
    let aptos_public_info = swarm.aptos_public_info();
    let response = aptos_public_info
        .client()
        .submit_and_wait(&transaction)
        .await;

    // Verify the transaction was rejected by the mempool filter
    let error = response.unwrap_err();
    assert!(error
        .to_string()
        .contains("API error Error(RejectedByFilter)"));

    // Execute a few more transactions and verify that they are processed correctly
    execute_test_transactions(&mut swarm).await;
}

#[tokio::test]
async fn test_quorum_store_batch_filter() {
    // Generate a new key pair and sender address
    let (private_key, sender_address) = create_sender_account();

    // Create a new swarm with a quorum store filter that denies transactions from the sender
    let mut swarm = SwarmBuilder::new_local(3)
        .with_aptos()
        .with_init_config(Arc::new(move |_, config, _| {
            filter_quorum_store_transactions(config, sender_address);
        }))
        .build()
        .await;

    // Execute a few regular transactions and verify that they are processed correctly
    execute_test_transactions(&mut swarm).await;

    // Prepare a transaction from the sender address
    let transaction = create_transaction_from_sender(private_key, sender_address, &mut swarm).await;

    // Submit the transaction and wait for it to be processed
    let aptos_public_info = swarm.aptos_public_info();
    let response = aptos_public_info
        .client()
        .submit_and_wait(&transaction)
        .await;

    // Verify the transaction was dropped by the quorum store filter
    let error = response.unwrap_err();
    assert!(error
        .to_string()
        .contains("Used to be pending and now not found. Transaction expired."));

    // Execute a few more transactions and verify that they are processed correctly
    execute_test_transactions(&mut swarm).await;
}

/// Creates a new on-chain account for the given address and sends some funds to it
async fn create_account_with_funds(
    public_key: &Ed25519PublicKey,
    sender_address: AccountAddress,
    swarm: &mut LocalSwarm,
) {
    let mut aptos_public_info = swarm.aptos_public_info();
    aptos_public_info
        .create_user_account(public_key)
        .await
        .unwrap();
    aptos_public_info
        .mint(sender_address, 10_000_000)
        .await
        .unwrap();
}

/// Creates a new sender account and returns the private key and address
fn create_sender_account() -> (Ed25519PrivateKey, AccountAddress) {
    let private_key = KeyGen::from_os_rng().generate_ed25519_private_key();
    let public_key = private_key.public_key();
    let sender_auth_key = AuthenticationKey::ed25519(&public_key);
    let sender_address = sender_auth_key.account_address();

    (private_key, sender_address)
}

/// Creates a signed transaction from the sender address to the receiver address
async fn create_signed_transaction_from_sender(
    private_key: Ed25519PrivateKey,
    sender_address: AccountAddress,
    receiver: LocalAccount,
    swarm: &mut LocalSwarm,
) -> SignedTransaction {
    // Fetch the sequence number for the sender address
    let aptos_public_info = swarm.aptos_public_info();
    let sequence_number = aptos_public_info
        .client()
        .get_account(sender_address)
        .await
        .unwrap()
        .into_inner()
        .sequence_number;

    // Create the unsigned transaction
    let unsigned_txn = aptos_public_info
        .transaction_factory()
        .payload(aptos_stdlib::aptos_coin_transfer(receiver.address(), 100))
        .sender(sender_address)
        .sequence_number(sequence_number)
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .build();
    assert_eq!(unsigned_txn.sender(), sender_address);

    // Sign the transaction
    let signature = private_key.sign(&unsigned_txn).unwrap();
    SignedTransaction::new(unsigned_txn.clone(), private_key.public_key(), signature)
}

/// Creates a transaction from the sender address to a random receiver address
async fn create_transaction_from_sender(
    private_key: Ed25519PrivateKey,
    sender_address: AccountAddress,
    swarm: &mut LocalSwarm,
) -> SignedTransaction {
    // Create the sender account and mint some coins to it
    create_account_with_funds(&private_key.public_key(), sender_address, swarm).await;

    // Create a receiver account and mint some coins to it
    let mut aptos_public_info = swarm.aptos_public_info();
    let receiver = aptos_public_info.random_account();
    create_account_with_funds(receiver.public_key(), receiver.address(), swarm).await;

    // Create a signed transaction
    create_signed_transaction_from_sender(private_key, sender_address, receiver, swarm).await
}

/// Executes a few test transactions and verifies that they are processed correctly
async fn execute_test_transactions(swarm: &mut LocalSwarm) {
    let validator_peer_id_1 = swarm.validators().next().unwrap().peer_id();
    let validator_client_1 = swarm.validator(validator_peer_id_1).unwrap().rest_client();
    let (mut account_0, account_1) = create_test_accounts(swarm).await;

    execute_transactions(
        swarm,
        &validator_client_1,
        &mut account_0,
        &account_1,
        false,
    )
    .await;
}

/// Adds a filter to the consensus config to ignore transactions from the given sender
fn filter_inline_transactions(node_config: &mut NodeConfig, sender_address: AccountAddress) {
    // Create the block transaction filter
    let block_transaction_filter = BlockTransactionFilter::empty()
        .add_multiple_matchers_filter(false, vec![BlockTransactionMatcher::Transaction(
            TransactionMatcher::Sender(sender_address),
        )])
        .add_all_filter(true);

    // Update the node config with the new filter
    node_config.transaction_filters.consensus_filter =
        BlockTransactionFilterConfig::new(true, block_transaction_filter);
}

/// Adds a filter to the mempool config to ignore transactions from the given sender
fn filter_mempool_transactions(node_config: &mut NodeConfig, sender_address: AccountAddress) {
    // Create the transaction filter
    let transaction_filter = TransactionFilter::empty()
        .add_multiple_matchers_filter(false, vec![TransactionMatcher::Sender(sender_address)])
        .add_all_filter(true);

    // Update the node config with the new filter
    node_config.transaction_filters.mempool_filter =
        TransactionFilterConfig::new(true, transaction_filter);
}

/// Adds a filter to the quorum store config to ignore transactions from the given sender
fn filter_quorum_store_transactions(node_config: &mut NodeConfig, sender_address: AccountAddress) {
    // Create the batch transaction filter
    let batch_transaction_filter = BatchTransactionFilter::empty()
        .add_multiple_matchers_filter(false, vec![BatchTransactionMatcher::Transaction(
            TransactionMatcher::Sender(sender_address),
        )])
        .add_all_filter(true);

    // Update the node config with the new filter
    node_config.transaction_filters.quorum_store_filter =
        BatchTransactionFilterConfig::new(true, batch_transaction_filter);
}
