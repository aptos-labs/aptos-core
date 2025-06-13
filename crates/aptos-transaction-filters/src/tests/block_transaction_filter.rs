// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block_transaction_filter::{BlockMatcher, BlockTransactionFilter, BlockTransactionMatcher},
    tests::utils,
    transaction_filter::TransactionMatcher,
};
use aptos_crypto::HashValue;
use aptos_types::transaction::SignedTransaction;

#[test]
fn test_all_filter() {
    for use_new_txn_payload_format in [false, true] {
        // Create a filter that allows all transactions
        let filter = BlockTransactionFilter::empty().add_all_filter(true);

        // Create a block ID, epoch, and timestamp
        let (block_id, block_epoch, block_timestamp) = utils::get_random_block_info();

        // Verify that all transactions are allowed
        let transactions = utils::create_entry_function_transactions(use_new_txn_payload_format);
        verify_all_transactions_allowed(
            filter,
            block_id,
            block_epoch,
            block_timestamp,
            transactions.clone(),
        );

        // Create a filter that denies all transactions
        let filter = BlockTransactionFilter::empty().add_all_filter(false);

        // Verify that all transactions are denied
        verify_all_transactions_rejected(
            filter,
            block_id,
            block_epoch,
            block_timestamp,
            transactions.clone(),
        );
    }
}

#[test]
fn test_block_id_filter() {
    for use_new_txn_payload_format in [false, true] {
        // Create a block ID, epoch, and timestamp
        let (block_id, block_epoch, block_timestamp) = utils::get_random_block_info();

        // Create a filter that only allows transactions with a specific block ID
        let transactions = utils::create_entry_function_transactions(use_new_txn_payload_format);
        let filter = BlockTransactionFilter::empty()
            .add_block_id_filter(true, block_id)
            .add_all_filter(false);

        // Verify that the filter allows transactions within the specified block ID
        verify_all_transactions_allowed(
            filter.clone(),
            block_id,
            block_epoch,
            block_timestamp,
            transactions.clone(),
        );

        // Verify that the filter denies transactions with a different block ID
        let different_block_id = HashValue::random();
        verify_all_transactions_rejected(
            filter.clone(),
            different_block_id,
            block_epoch,
            block_timestamp,
            transactions.clone(),
        );

        // Create a filter that denies transactions with a specific block ID
        let filter = BlockTransactionFilter::empty().add_block_id_filter(false, block_id);

        // Verify that the filter denies transactions within the specified block ID
        verify_all_transactions_rejected(
            filter.clone(),
            block_id,
            block_epoch,
            block_timestamp,
            transactions.clone(),
        );

        // Verify that the filter allows transactions with a different block ID
        let different_block_id = HashValue::random();
        verify_all_transactions_allowed(
            filter.clone(),
            different_block_id,
            block_epoch,
            block_timestamp,
            transactions.clone(),
        );
    }
}

#[test]
fn test_block_epoch_greater_than_filter() {
    for use_new_txn_payload_format in [false, true] {
        // Create a filter that only allows transactions with a block epoch greater than a specific value
        let filter = BlockTransactionFilter::empty()
            .add_block_epoch_greater_than_filter(true, 1000)
            .add_all_filter(false);

        // Create a block ID and epoch
        let (block_id, _, block_timestamp) = utils::get_random_block_info();

        // Verify that the filter only allows transactions with a block epoch greater than 1000
        let transactions = utils::create_entry_function_transactions(use_new_txn_payload_format);
        for block_epoch in [0, 999, 1000] {
            verify_all_transactions_rejected(
                filter.clone(),
                block_id,
                block_epoch,
                block_timestamp,
                transactions.clone(),
            );
        }
        for block_epoch in [1001, 1002] {
            verify_all_transactions_allowed(
                filter.clone(),
                block_id,
                block_epoch,
                block_timestamp,
                transactions.clone(),
            );
        }

        // Create a filter that denies transactions with a block epoch greater than a specific value
        let filter = BlockTransactionFilter::empty()
            .add_block_epoch_greater_than_filter(false, 1000)
            .add_all_filter(true);

        // Verify that the filter only allows transactions with a block epoch less than or equal to 1000
        for block_epoch in [0, 999, 1000] {
            verify_all_transactions_allowed(
                filter.clone(),
                block_id,
                block_epoch,
                block_timestamp,
                transactions.clone(),
            );
        }
        for block_epoch in [1001, 2000] {
            verify_all_transactions_rejected(
                filter.clone(),
                block_id,
                block_epoch,
                block_timestamp,
                transactions.clone(),
            );
        }
    }
}

#[test]
fn test_block_epoch_less_than_filter() {
    for use_new_txn_payload_format in [false, true] {
        // Create a filter that only allows transactions with a block epoch less than a specific value
        let filter = BlockTransactionFilter::empty()
            .add_block_epoch_less_than_filter(true, 1000)
            .add_all_filter(false);

        // Create a block ID and epoch
        let (block_id, _, block_timestamp) = utils::get_random_block_info();

        // Verify that the filter only allows transactions with a block epoch less than 1000
        let transactions = utils::create_entry_function_transactions(use_new_txn_payload_format);
        for block_epoch in [0, 999] {
            verify_all_transactions_allowed(
                filter.clone(),
                block_id,
                block_epoch,
                block_timestamp,
                transactions.clone(),
            );
        }
        for block_epoch in [1000, 1001] {
            verify_all_transactions_rejected(
                filter.clone(),
                block_id,
                block_epoch,
                block_timestamp,
                transactions.clone(),
            );
        }

        // Create a filter that denies transactions with a block epoch less than a specific value
        let filter = BlockTransactionFilter::empty()
            .add_block_epoch_less_than_filter(false, 1000)
            .add_all_filter(true);

        // Verify that the filter only allows transactions with a block epoch greater than or equal to 1000
        for block_epoch in [0, 999] {
            verify_all_transactions_rejected(
                filter.clone(),
                block_id,
                block_epoch,
                block_timestamp,
                transactions.clone(),
            );
        }
        for block_epoch in [1000, 1001] {
            verify_all_transactions_allowed(
                filter.clone(),
                block_id,
                block_epoch,
                block_timestamp,
                transactions.clone(),
            );
        }
    }
}

#[test]
fn test_block_timestamp_greater_than_filter() {
    for use_new_txn_payload_format in [false, true] {
        // Create a filter that only allows transactions with a block timestamp greater than a specific value
        let filter = BlockTransactionFilter::empty()
            .add_block_timestamp_greater_than_filter(true, 1000)
            .add_all_filter(false);

        // Create a block ID and epoch
        let (block_id, block_epoch, _) = utils::get_random_block_info();

        // Verify that the filter only allows transactions with a block timestamp greater than 1000
        let transactions = utils::create_entry_function_transactions(use_new_txn_payload_format);
        for block_timestamp in [0, 999, 1000] {
            verify_all_transactions_rejected(
                filter.clone(),
                block_id,
                block_epoch,
                block_timestamp,
                transactions.clone(),
            );
        }
        for block_timestamp in [1001, 2000] {
            verify_all_transactions_allowed(
                filter.clone(),
                block_id,
                block_epoch,
                block_timestamp,
                transactions.clone(),
            );
        }

        // Create a filter that denies transactions with a block timestamp greater than a specific value
        let filter = BlockTransactionFilter::empty()
            .add_block_timestamp_greater_than_filter(false, 1000)
            .add_all_filter(true);

        // Verify that the filter only allows transactions with a block timestamp less than or equal to 1000
        for block_timestamp in [0, 999, 1000] {
            verify_all_transactions_allowed(
                filter.clone(),
                block_id,
                block_epoch,
                block_timestamp,
                transactions.clone(),
            );
        }
        for block_timestamp in [1001, 2000] {
            verify_all_transactions_rejected(
                filter.clone(),
                block_id,
                block_epoch,
                block_timestamp,
                transactions.clone(),
            );
        }
    }
}

#[test]
fn test_block_timestamp_less_than_filter() {
    for use_new_txn_payload_format in [false, true] {
        // Create a filter that only allows transactions with a block timestamp less than a specific value
        let filter = BlockTransactionFilter::empty()
            .add_block_timestamp_less_than_filter(true, 1000)
            .add_all_filter(false);

        // Create a block ID and epoch
        let (block_id, block_epoch, _) = utils::get_random_block_info();

        // Verify that the filter only allows transactions with a block timestamp less than 1000
        let transactions = utils::create_entry_function_transactions(use_new_txn_payload_format);
        for block_timestamp in [0, 999] {
            verify_all_transactions_allowed(
                filter.clone(),
                block_id,
                block_epoch,
                block_timestamp,
                transactions.clone(),
            );
        }
        for block_timestamp in [1000, 1001, 2000] {
            verify_all_transactions_rejected(
                filter.clone(),
                block_id,
                block_epoch,
                block_timestamp,
                transactions.clone(),
            );
        }

        // Create a filter that denies transactions with a block timestamp less than a specific value
        let filter = BlockTransactionFilter::empty()
            .add_block_timestamp_less_than_filter(false, 1000)
            .add_all_filter(true);

        // Verify that the filter only allows transactions with a block timestamp greater than or equal to 1000
        for block_timestamp in [0, 999] {
            verify_all_transactions_rejected(
                filter.clone(),
                block_id,
                block_epoch,
                block_timestamp,
                transactions.clone(),
            );
        }
        for block_timestamp in [1000, 1001, 2000] {
            verify_all_transactions_allowed(
                filter.clone(),
                block_id,
                block_epoch,
                block_timestamp,
                transactions.clone(),
            );
        }
    }
}

#[test]
fn test_empty_filter() {
    for use_new_txn_payload_format in [false, true] {
        // Create an empty filter
        let filter = BlockTransactionFilter::empty();

        // Create a block ID, epoch, and timestamp
        let (block_id, block_epoch, block_timestamp) = utils::get_random_block_info();

        // Verify that all transactions are allowed
        let transactions = utils::create_entry_function_transactions(use_new_txn_payload_format);
        verify_all_transactions_allowed(
            filter.clone(),
            block_id,
            block_epoch,
            block_timestamp,
            transactions.clone(),
        );
    }
}

#[test]
fn test_multiple_matchers_filter() {
    for use_new_txn_payload_format in [false, true] {
        // Create a block ID and epoch
        let (block_id, block_epoch, block_timestamp) = utils::get_random_block_info();

        // Create a filter that only allows block transactions with epoch > 1000 and a specific sender (txn 0)
        let transactions = utils::create_entry_function_transactions(use_new_txn_payload_format);
        let block_transaction_matchers = vec![
            BlockTransactionMatcher::Block(BlockMatcher::BlockEpochGreaterThan(1000)),
            BlockTransactionMatcher::Transaction(TransactionMatcher::Sender(
                transactions[0].sender(),
            )),
        ];
        let filter = BlockTransactionFilter::empty()
            .add_multiple_matchers_filter(true, block_transaction_matchers)
            .add_all_filter(false);

        // Verify that the filter returns no transactions with block epoch less than or equal to 1000
        for block_epoch in [0, 999, 1000] {
            verify_all_transactions_rejected(
                filter.clone(),
                block_id,
                block_epoch,
                block_timestamp,
                transactions.clone(),
            );
        }

        // Verify that the filter returns transactions with block epoch greater than 1000 and the specified sender
        for block_epoch in [1001, 2002] {
            let filtered_transactions = filter.filter_block_transactions(
                block_id,
                block_epoch,
                block_timestamp,
                transactions.clone(),
            );
            assert_eq!(filtered_transactions, transactions[0..1].to_vec());
        }

        // Create a filter that denies block transactions with timestamp < 1000 and a specific sender (txn 0 and 1)
        let block_transaction_matchers_0 = vec![
            BlockTransactionMatcher::Block(BlockMatcher::BlockTimeStampLessThan(1000)),
            BlockTransactionMatcher::Transaction(TransactionMatcher::Sender(
                transactions[0].sender(),
            )),
        ];
        let block_transaction_matchers_1 = vec![
            BlockTransactionMatcher::Block(BlockMatcher::BlockTimeStampLessThan(1000)),
            BlockTransactionMatcher::Transaction(TransactionMatcher::Sender(
                transactions[1].sender(),
            )),
        ];
        let filter = BlockTransactionFilter::empty()
            .add_multiple_matchers_filter(false, block_transaction_matchers_0)
            .add_multiple_matchers_filter(false, block_transaction_matchers_1)
            .add_all_filter(true);

        // Verify that it returns all transactions with block timestamp greater than or equal to 1000
        for block_timestamp in [1000, 1001, 2000] {
            verify_all_transactions_allowed(
                filter.clone(),
                block_id,
                block_epoch,
                block_timestamp,
                transactions.clone(),
            );
        }

        // Verify that it returns no transactions with block timestamp less than 1000 and the specified senders
        for block_timestamp in [0, 999] {
            let filtered_transactions = filter.filter_block_transactions(
                block_id,
                block_epoch,
                block_timestamp,
                transactions.clone(),
            );
            assert_eq!(filtered_transactions, transactions[2..].to_vec());
        }
    }
}

/// Verifies that all transactions are allowed by the given filter
fn verify_all_transactions_allowed(
    filter: BlockTransactionFilter,
    block_id: HashValue,
    block_epoch: u64,
    block_timestamp: u64,
    transactions: Vec<SignedTransaction>,
) {
    let filtered_transactions = filter.filter_block_transactions(
        block_id,
        block_epoch,
        block_timestamp,
        transactions.clone(),
    );
    assert_eq!(filtered_transactions, transactions);
}

/// Verifies that all transactions are rejected by the given filter
fn verify_all_transactions_rejected(
    filter: BlockTransactionFilter,
    block_id: HashValue,
    block_epoch: u64,
    block_timestamp: u64,
    transactions: Vec<SignedTransaction>,
) {
    let filtered_transactions =
        filter.filter_block_transactions(block_id, block_epoch, block_timestamp, transactions);
    assert!(filtered_transactions.is_empty());
}
