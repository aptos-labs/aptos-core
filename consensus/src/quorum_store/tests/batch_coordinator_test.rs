// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    network::NetworkSender,
    network_interface::ConsensusNetworkClient,
    quorum_store::{
        batch_coordinator::BatchCoordinator, batch_generator::BatchGeneratorCommand,
        batch_store::BatchStore, proof_manager::ProofManagerCommand,
        quorum_store_db::MockQuorumStoreDB, types::Batch,
    },
};
use aptos_config::config::TransactionFilterConfig;
use aptos_consensus_types::{common::Author, proof_of_store::BatchId};
use aptos_crypto::{ed25519::Ed25519PrivateKey, PrivateKey, SigningKey, Uniform};
use aptos_network::application::{interface::NetworkClient, storage::PeersAndMetadata};
use aptos_transactions_filter::transaction_matcher::Filter;
use aptos_types::{
    chain_id::ChainId,
    transaction::{RawTransaction, Script, SignedTransaction, TransactionPayload},
    validator_signer::ValidatorSigner,
    validator_verifier::ValidatorVerifier,
    PeerId,
};
use futures::FutureExt;
use move_core_types::account_address::AccountAddress;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{
    sync::mpsc::{channel, Sender},
    time::timeout,
};

#[tokio::test(flavor = "multi_thread")]
async fn test_handle_batches_msg_filter_disabled() {
    // Create the message channels
    let (sender_to_proof_manager, _receiver_for_proof_manager) = channel(100);
    let (sender_to_batch_generator, mut receiver_for_batch_generator) = channel(100);

    // Create a filtering config with filtering disabled
    let transaction_filter = Filter::empty().add_all_filter(false);
    let transaction_filter_config = TransactionFilterConfig {
        enable_quorum_store_filter: false,
        transaction_filter,
        ..TransactionFilterConfig::default()
    };

    // Create a batch coordinator
    let mut batch_coordinator = create_batch_coordinator(
        sender_to_proof_manager,
        sender_to_batch_generator,
        transaction_filter_config,
    );

    // Create a single batch with some transactions
    let transactions = create_signed_transactions(10);
    let account_address = AccountAddress::random();
    let batch = Batch::new(
        BatchId::new_for_test(100),
        transactions.clone(),
        1,
        1,
        account_address,
        0,
    );

    // Handle a batches message
    batch_coordinator
        .handle_batches_msg(account_address, vec![batch.clone()])
        .await;

    // Verify that the receiver for the batch generator received the batch
    let received_message = timeout(Duration::from_secs(10), receiver_for_batch_generator.recv())
        .await
        .unwrap()
        .unwrap();
    if let BatchGeneratorCommand::RemoteBatch(remote_batch) = received_message {
        assert_eq!(remote_batch.batch_info(), batch.batch_info());
    } else {
        panic!(
            "Expected a RemoteBatch command! Received: {:?}",
            received_message
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_handle_batches_msg_filter_enabled() {
    // Create the message channels
    let (sender_to_proof_manager, _receiver_for_proof_manager) = channel(100);
    let (sender_to_batch_generator, mut receiver_for_batch_generator) = channel(100);

    // Create a filtering config with filtering enabled (the first transaction sender is rejected)
    let transactions = create_signed_transactions(10);
    let transaction_filter = Filter::empty().add_sender_filter(false, transactions[0].sender());
    let transaction_filter_config = TransactionFilterConfig {
        enable_quorum_store_filter: true,
        transaction_filter,
        ..TransactionFilterConfig::default()
    };

    // Create a batch coordinator
    let mut batch_coordinator = create_batch_coordinator(
        sender_to_proof_manager,
        sender_to_batch_generator,
        transaction_filter_config,
    );

    // Create a single batch
    let account_address = AccountAddress::random();
    let batch = Batch::new(
        BatchId::new_for_test(109),
        transactions.clone(),
        1,
        1,
        account_address,
        0,
    );

    // Handle a batches message
    batch_coordinator
        .handle_batches_msg(account_address, vec![batch])
        .await;

    // Verify that the receiver for the batch generator does not receive the batch
    assert!(receiver_for_batch_generator.recv().now_or_never().is_none());
}

/// Creates and returns a new batch coordinator with the specified parameters
fn create_batch_coordinator(
    sender_to_proof_manager: Sender<ProofManagerCommand>,
    sender_to_batch_generator: Sender<BatchGeneratorCommand>,
    transaction_filter_config: TransactionFilterConfig,
) -> BatchCoordinator {
    // Create the consensus network sender and batch store
    let consensus_network_sender = create_consensus_network_sender();
    let batch_store = create_batch_store();

    // Create the batch coordinator
    BatchCoordinator::new(
        PeerId::random(),
        consensus_network_sender,
        sender_to_proof_manager,
        sender_to_batch_generator,
        Arc::new(batch_store),
        10_000,
        10_000,
        10_000,
        10_000,
        10_000,
        transaction_filter_config,
    )
}

/// Creates and returns a mock batch store
fn create_batch_store() -> BatchStore {
    let qs_storage = Arc::new(MockQuorumStoreDB::new());
    let validator_signer = ValidatorSigner::random(None);
    BatchStore::new(0, false, 0, qs_storage, 0, 0, 0, validator_signer, 0)
}

/// Creates and returns a mock consensus network sender
fn create_consensus_network_sender() -> NetworkSender {
    // Create the consensus network client
    let peers_and_metadata = PeersAndMetadata::new(&[]);
    let network_client =
        NetworkClient::new(vec![], vec![], HashMap::new(), peers_and_metadata.clone());
    let consensus_network_client = ConsensusNetworkClient::new(network_client.clone());

    // Create the self sender and validator verifier
    let (self_sender, _self_receiver) = aptos_channels::new_unbounded_test();
    let validator_verifier = Arc::new(ValidatorVerifier::new(vec![]));

    // Create a network sender
    NetworkSender::new(
        Author::random(),
        consensus_network_client,
        self_sender,
        validator_verifier,
    )
}

/// Creates and returns a raw transaction
fn create_raw_transaction() -> RawTransaction {
    RawTransaction::new(
        AccountAddress::random(),
        0,
        TransactionPayload::Script(Script::new(vec![], vec![], vec![])),
        0,
        0,
        0,
        ChainId::new(10),
    )
}

/// Creates and returns the specified number of signed transactions
fn create_signed_transactions(num_transactions: u64) -> Vec<SignedTransaction> {
    let mut signed_transactions = Vec::new();

    for _ in 0..num_transactions {
        let raw_transaction = create_raw_transaction();
        let private_key_1 = Ed25519PrivateKey::generate_for_testing();
        let signature = private_key_1.sign(&raw_transaction).unwrap();

        let signed_transaction = SignedTransaction::new(
            raw_transaction.clone(),
            private_key_1.public_key(),
            signature.clone(),
        );
        signed_transactions.push(signed_transaction);
    }

    signed_transactions
}
