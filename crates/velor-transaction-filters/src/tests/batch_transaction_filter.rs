// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    batch_transaction_filter::{BatchMatcher, BatchTransactionFilter, BatchTransactionMatcher},
    tests::utils,
    transaction_filter::TransactionMatcher,
};
use velor_crypto::HashValue;
use velor_types::{quorum_store::BatchId, transaction::SignedTransaction, PeerId};

#[test]
fn test_all_filter() {
    for use_new_txn_payload_format in [false, true] {
        // Create a filter that allows all transactions
        let filter = BatchTransactionFilter::empty().add_all_filter(true);

        // Create a batch ID, author, and digest
        let (batch_id, batch_author, batch_digest) = utils::get_random_batch_info();

        // Verify that all transactions are allowed
        let transactions = utils::create_entry_function_transactions(use_new_txn_payload_format);
        verify_all_transactions_allowed(
            filter,
            batch_id,
            batch_author,
            batch_digest,
            transactions.clone(),
        );

        // Create a filter that denies all transactions
        let filter = BatchTransactionFilter::empty().add_all_filter(false);

        // Verify that all transactions are denied
        verify_all_transactions_rejected(
            filter,
            batch_id,
            batch_author,
            batch_digest,
            transactions.clone(),
        );
    }
}

#[test]
fn test_batch_id_filter() {
    for use_new_txn_payload_format in [false, true] {
        // Create a batch ID, author, and digest
        let (batch_id, batch_author, batch_digest) = utils::get_random_batch_info();

        // Create a filter that only allows transactions with a specific batch ID
        let transactions = utils::create_entry_function_transactions(use_new_txn_payload_format);
        let filter = BatchTransactionFilter::empty()
            .add_batch_id_filter(true, batch_id)
            .add_all_filter(false);

        // Verify that the filter allows transactions within the specified batch ID
        verify_all_transactions_allowed(
            filter.clone(),
            batch_id,
            batch_author,
            batch_digest,
            transactions.clone(),
        );

        // Verify that the filter denies transactions with a different batch ID
        let different_batch_id = BatchId::new_for_test(1000);
        verify_all_transactions_rejected(
            filter.clone(),
            different_batch_id,
            batch_author,
            batch_digest,
            transactions.clone(),
        );

        // Create a filter that denies transactions with a specific batch ID
        let filter = BatchTransactionFilter::empty().add_batch_id_filter(false, batch_id);

        // Verify that the filter denies transactions within the specified batch ID
        verify_all_transactions_rejected(
            filter.clone(),
            batch_id,
            batch_author,
            batch_digest,
            transactions.clone(),
        );

        // Verify that the filter allows transactions with a different batch ID
        let different_batch_id = BatchId::new_for_test(200);
        verify_all_transactions_allowed(
            filter.clone(),
            different_batch_id,
            batch_author,
            batch_digest,
            transactions.clone(),
        );
    }
}

#[test]
fn test_batch_author_filter() {
    for use_new_txn_payload_format in [false, true] {
        // Create a batch ID, author, and digest
        let (batch_id, batch_author, batch_digest) = utils::get_random_batch_info();

        // Create a filter that only allows transactions with a specific batch author
        let transactions = utils::create_entry_function_transactions(use_new_txn_payload_format);
        let filter = BatchTransactionFilter::empty()
            .add_batch_author_filter(true, batch_author)
            .add_all_filter(false);

        // Verify that the filter allows transactions with the specified batch author
        verify_all_transactions_allowed(
            filter.clone(),
            batch_id,
            batch_author,
            batch_digest,
            transactions.clone(),
        );

        // Verify that the filter denies transactions with a different batch author
        let different_batch_author = PeerId::random();
        verify_all_transactions_rejected(
            filter.clone(),
            batch_id,
            different_batch_author,
            batch_digest,
            transactions.clone(),
        );

        // Create a filter that denies transactions with a specific batch author
        let filter = BatchTransactionFilter::empty().add_batch_author_filter(false, batch_author);

        // Verify that the filter denies transactions with the specified batch author
        verify_all_transactions_rejected(
            filter.clone(),
            batch_id,
            batch_author,
            batch_digest,
            transactions.clone(),
        );

        // Verify that the filter allows transactions with a different batch author
        verify_all_transactions_allowed(
            filter.clone(),
            batch_id,
            different_batch_author,
            batch_digest,
            transactions.clone(),
        );
    }
}

#[test]
fn test_batch_digest_filter() {
    for use_new_txn_payload_format in [false, true] {
        // Create a batch ID, author, and digest
        let (batch_id, batch_author, batch_digest) = utils::get_random_batch_info();

        // Create a filter that only allows transactions with a specific batch digest
        let transactions = utils::create_entry_function_transactions(use_new_txn_payload_format);
        let filter = BatchTransactionFilter::empty()
            .add_batch_digest_filter(true, batch_digest)
            .add_all_filter(false);

        // Verify that the filter allows transactions with the specified batch digest
        verify_all_transactions_allowed(
            filter.clone(),
            batch_id,
            batch_author,
            batch_digest,
            transactions.clone(),
        );

        // Verify that the filter denies transactions with a different batch digest
        let different_batch_digest = HashValue::random();
        verify_all_transactions_rejected(
            filter.clone(),
            batch_id,
            batch_author,
            different_batch_digest,
            transactions.clone(),
        );

        // Create a filter that denies transactions with a specific batch digest
        let filter = BatchTransactionFilter::empty().add_batch_digest_filter(false, batch_digest);

        // Verify that the filter denies transactions with the specified batch digest
        verify_all_transactions_rejected(
            filter.clone(),
            batch_id,
            batch_author,
            batch_digest,
            transactions.clone(),
        );

        // Verify that the filter allows transactions with a different batch digest
        verify_all_transactions_allowed(
            filter.clone(),
            batch_id,
            batch_author,
            different_batch_digest,
            transactions.clone(),
        );
    }
}

#[test]
fn test_empty_filter() {
    for use_new_txn_payload_format in [false, true] {
        // Create an empty filter
        let filter = BatchTransactionFilter::empty();

        // Create a batch ID, author, and digest
        let (batch_id, batch_author, batch_digest) = utils::get_random_batch_info();

        // Verify that all transactions are allowed
        let transactions = utils::create_entry_function_transactions(use_new_txn_payload_format);
        verify_all_transactions_allowed(
            filter.clone(),
            batch_id,
            batch_author,
            batch_digest,
            transactions.clone(),
        );
    }
}

#[test]
fn test_multiple_matchers_filter() {
    for use_new_txn_payload_format in [false, true] {
        // Create a batch ID, author, and digest
        let (batch_id, batch_author, batch_digest) = utils::get_random_batch_info();

        // Create a filter that only allows batch transactions with a specific author and sender (txn 0)
        let transactions = utils::create_entry_function_transactions(use_new_txn_payload_format);
        let batch_transaction_matchers = vec![
            BatchTransactionMatcher::Batch(BatchMatcher::BatchAuthor(batch_author)),
            BatchTransactionMatcher::Transaction(TransactionMatcher::Sender(
                transactions[0].sender(),
            )),
        ];
        let filter = BatchTransactionFilter::empty()
            .add_multiple_matchers_filter(true, batch_transaction_matchers)
            .add_all_filter(false);

        // Verify that the filter returns no transactions with a different batch author
        verify_all_transactions_rejected(
            filter.clone(),
            batch_id,
            PeerId::random(), // Use a different author
            batch_digest,
            transactions.clone(),
        );

        // Verify that the filter returns transactions with the specified batch author and sender
        let filtered_transactions = filter.filter_batch_transactions(
            batch_id,
            batch_author,
            batch_digest,
            transactions.clone(),
        );
        assert_eq!(filtered_transactions, transactions[0..1].to_vec());

        // Create a filter that denies batch transactions with a specific author and sender (txn 0 and 1)
        let batch_transaction_matchers_0 = vec![
            BatchTransactionMatcher::Batch(BatchMatcher::BatchAuthor(batch_author)),
            BatchTransactionMatcher::Transaction(TransactionMatcher::Sender(
                transactions[0].sender(),
            )),
        ];
        let batch_transaction_matchers_1 = vec![
            BatchTransactionMatcher::Batch(BatchMatcher::BatchAuthor(batch_author)),
            BatchTransactionMatcher::Transaction(TransactionMatcher::Sender(
                transactions[1].sender(),
            )),
        ];
        let filter = BatchTransactionFilter::empty()
            .add_multiple_matchers_filter(false, batch_transaction_matchers_0)
            .add_multiple_matchers_filter(false, batch_transaction_matchers_1)
            .add_all_filter(true);

        // Verify that the filter returns all transaction with a different batch author
        verify_all_transactions_allowed(
            filter.clone(),
            batch_id,
            PeerId::random(), // Use a different author
            batch_digest,
            transactions.clone(),
        );

        // Verify that the filter rejects transactions with the specified batch author and senders
        let filtered_transactions = filter.filter_batch_transactions(
            batch_id,
            batch_author,
            batch_digest,
            transactions.clone(),
        );
        assert_eq!(filtered_transactions, transactions[2..].to_vec());
    }
}

/// Verifies that all transactions are allowed by the given filter
fn verify_all_transactions_allowed(
    filter: BatchTransactionFilter,
    batch_id: BatchId,
    batch_author: PeerId,
    batch_digest: HashValue,
    transactions: Vec<SignedTransaction>,
) {
    let filtered_transactions = filter.filter_batch_transactions(
        batch_id,
        batch_author,
        batch_digest,
        transactions.clone(),
    );
    assert_eq!(filtered_transactions, transactions);
}

/// Verifies that all transactions are rejected by the given filter
fn verify_all_transactions_rejected(
    filter: BatchTransactionFilter,
    batch_id: BatchId,
    batch_author: PeerId,
    batch_digest: HashValue,
    transactions: Vec<SignedTransaction>,
) {
    let filtered_transactions = filter.filter_batch_transactions(
        batch_id,
        batch_author,
        batch_digest,
        transactions.clone(),
    );
    assert!(filtered_transactions.is_empty());
}
