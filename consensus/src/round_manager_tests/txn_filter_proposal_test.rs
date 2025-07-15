// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    network_tests::NetworkPlayground,
    round_manager::round_manager_tests::NodeSetup,
    test_utils::{consensus_runtime, timed_block_on},
};
use aptos_config::config::BlockTransactionFilterConfig;
use aptos_consensus_types::{
    block::{block_test_utils::certificate_for_genesis, Block},
    common::{Payload, ProofWithData},
    proof_of_store::BatchInfo,
};
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519Signature},
    HashValue, PrivateKey, Uniform,
};
use aptos_transaction_filters::{
    block_transaction_filter::{BlockTransactionFilter, BlockTransactionMatcher},
    transaction_filter::TransactionMatcher,
};
use aptos_types::{
    account_address::AccountAddress,
    chain_id::ChainId,
    quorum_store::BatchId,
    transaction::{RawTransaction, Script, SignedTransaction, TransactionPayload},
    PeerId,
};

// Verify that the round manager will not vote if a block
// proposal contains any denied inline transactions.
#[test]
fn test_no_vote_on_denied_inline_transactions() {
    // Test both direct mempool and quorum store payloads
    for use_quorum_store_payloads in [false, true] {
        // Create test transactions
        let transactions = create_test_transactions();

        // Create a block filter config that denies the first transaction sender
        let block_txn_filter = BlockTransactionFilter::empty()
            .add_multiple_matchers_filter(false, vec![BlockTransactionMatcher::Transaction(
                TransactionMatcher::Sender(transactions[0].sender()),
            )])
            .add_all_filter(true);
        let block_txn_filter_config = BlockTransactionFilterConfig::new(true, block_txn_filter);

        // Create a new network playground
        let runtime = consensus_runtime();
        let mut playground = NetworkPlayground::new(runtime.handle().clone());

        // Create a new consensus node. Note: To observe the votes we're
        // going to check proposal processing on the non-proposer node
        // (which will send the votes to the proposer).
        let mut nodes = NodeSetup::create_nodes(
            &mut playground,
            runtime.handle().clone(),
            1,
            None,
            None,
            Some(block_txn_filter_config),
            None,
            None,
            None,
            use_quorum_store_payloads,
        );
        let node = &mut nodes[0];

        // Create a block proposal with inline transactions that will be denied
        let payload = create_payload(transactions, use_quorum_store_payloads);
        let denied_block = Block::new_proposal(
            payload,
            1,
            1,
            certificate_for_genesis(),
            &node.signer,
            Vec::new(),
        )
        .unwrap();

        // Verify that the node does not vote on a block with denied inline transactions
        timed_block_on(&runtime, async {
            assert!(node
                .round_manager
                .process_proposal(denied_block)
                .await
                .is_err());
        });
    }
}

// Verify that the round manager will not invoke
// the filter if the config is disabled.
#[test]
fn test_vote_on_disabled_filter() {
    // Test both direct mempool and quorum store payloads
    for use_quorum_store_payloads in [false, true] {
        // Create a block filter config that denies all transactions, however,
        // the filter is disabled, so it should not be invoked.
        let block_txn_filter = BlockTransactionFilter::empty().add_all_filter(false);
        let block_txn_filter_config = BlockTransactionFilterConfig::new(false, block_txn_filter);

        // Create a new network playground
        let runtime = consensus_runtime();
        let mut playground = NetworkPlayground::new(runtime.handle().clone());

        // Create a new consensus node. Note: To observe the votes we're
        // going to check proposal processing on the non-proposer node
        // (which will send the votes to the proposer).
        let mut nodes = NodeSetup::create_nodes(
            &mut playground,
            runtime.handle().clone(),
            1,
            None,
            None,
            Some(block_txn_filter_config),
            None,
            None,
            None,
            use_quorum_store_payloads,
        );
        let node = &mut nodes[0];

        // Create a block proposal with inline transactions
        let transactions = create_test_transactions();
        let payload = create_payload(transactions, use_quorum_store_payloads);
        let allowed_block = Block::new_proposal(
            payload,
            1,
            1,
            certificate_for_genesis(),
            &node.signer,
            Vec::new(),
        )
        .unwrap();
        let allowed_block_id = allowed_block.id();

        // Verify that the node votes on the block correctly
        timed_block_on(&runtime, async {
            node.round_manager
                .process_proposal(allowed_block)
                .await
                .unwrap();
            let vote_msg = node.next_vote().await;
            assert_eq!(
                vote_msg.vote().vote_data().proposed().id(),
                allowed_block_id
            );
        });
    }
}

// Verify that the round manager will still vote on a block
// if there are no denied inline transactions that match.
#[test]
fn test_vote_on_no_filter_matches() {
    // Test both direct mempool and quorum store payloads
    for use_quorum_store_payloads in [false, true] {
        // Create test transactions
        let transactions = create_test_transactions();

        // Create a block filter config that denies the first transaction sender
        let block_txn_filter = BlockTransactionFilter::empty()
            .add_multiple_matchers_filter(false, vec![BlockTransactionMatcher::Transaction(
                TransactionMatcher::Sender(transactions[0].sender()),
            )])
            .add_all_filter(true);
        let block_txn_filter_config = BlockTransactionFilterConfig::new(true, block_txn_filter);

        // Create a new network playground
        let runtime = consensus_runtime();
        let mut playground = NetworkPlayground::new(runtime.handle().clone());

        // Create a new consensus node. Note: To observe the votes we're
        // going to check proposal processing on the non-proposer node
        // (which will send the votes to the proposer).
        let mut nodes = NodeSetup::create_nodes(
            &mut playground,
            runtime.handle().clone(),
            1,
            None,
            None,
            Some(block_txn_filter_config),
            None,
            None,
            None,
            use_quorum_store_payloads,
        );
        let node = &mut nodes[0];

        // Create a block proposal with inline transactions that don't include the denied sender
        let payload = create_payload(transactions[1..].to_vec(), use_quorum_store_payloads);
        let allowed_block = Block::new_proposal(
            payload,
            1,
            1,
            certificate_for_genesis(),
            &node.signer,
            Vec::new(),
        )
        .unwrap();
        let allowed_block_id = allowed_block.id();

        // Verify that the node votes on the block correctly
        timed_block_on(&runtime, async {
            node.round_manager
                .process_proposal(allowed_block)
                .await
                .unwrap();
            let vote_msg = node.next_vote().await;
            assert_eq!(
                vote_msg.vote().vote_data().proposed().id(),
                allowed_block_id
            );
        });
    }
}

/// Creates and returns a new batch info with the specified number of transactions
fn create_batch_info(num_transactions: usize) -> BatchInfo {
    BatchInfo::new(
        PeerId::ZERO,
        BatchId::new(0),
        1,
        0,
        HashValue::random(),
        num_transactions as u64,
        1,
        0,
    )
}

/// Creates and returns a payload based on the provided transactions and whether to use QS
fn create_payload(
    transactions: Vec<SignedTransaction>,
    use_quorum_store_payloads: bool,
) -> Payload {
    if use_quorum_store_payloads {
        let inline_batch = (create_batch_info(transactions.len()), transactions);
        Payload::QuorumStoreInlineHybrid(vec![inline_batch], ProofWithData::empty(), None)
    } else {
        Payload::DirectMempool(transactions)
    }
}

/// Creates and returns a list of test transactions
fn create_test_transactions() -> Vec<SignedTransaction> {
    let mut transactions = vec![];

    for i in 0..10 {
        // Create the raw transaction
        let transaction_payload = TransactionPayload::Script(Script::new(vec![], vec![], vec![]));
        let raw_transaction = RawTransaction::new(
            AccountAddress::random(),
            i,
            transaction_payload,
            0,
            0,
            0,
            ChainId::new(10),
        );

        // Create a signed transaction
        let private_key = Ed25519PrivateKey::generate_for_testing();
        let public_key = private_key.public_key();
        let signed_transaction = SignedTransaction::new(
            raw_transaction,
            public_key,
            Ed25519Signature::dummy_signature(),
        );

        // Add the signed transaction to the list
        transactions.push(signed_transaction);
    }

    transactions
}
