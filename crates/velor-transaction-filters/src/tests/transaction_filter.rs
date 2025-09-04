// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    tests::utils,
    transaction_filter::{TransactionFilter, TransactionMatcher},
};

#[test]
fn test_account_address_filter_simple() {
    for use_new_txn_payload_format in [false, true] {
        // Create a filter that only allows transactions from specific account addresses.
        // These are: (i) txn 0 sender; (ii) txn 1 sender; and (iii) txn 2 entry function address.
        let transactions = utils::create_entry_function_transactions(use_new_txn_payload_format);
        let filter = TransactionFilter::empty()
            .add_account_address_filter(true, transactions[0].sender())
            .add_account_address_filter(true, transactions[1].sender())
            .add_account_address_filter(true, utils::get_module_address(&transactions[2]))
            .add_all_filter(false);

        // Verify that the filter returns transactions from the specified account addresses
        let filtered_transactions = filter.filter_transactions(transactions.clone());
        assert_eq!(filtered_transactions, transactions[0..3].to_vec());

        // Create a filter that denies transactions from the specified account addresses (as above)
        let filter = TransactionFilter::empty()
            .add_account_address_filter(false, transactions[0].sender())
            .add_account_address_filter(false, transactions[1].sender())
            .add_account_address_filter(false, utils::get_module_address(&transactions[2]))
            .add_all_filter(true);

        // Verify that the filter returns transactions from other account addresses
        let filtered_transactions = filter.filter_transactions(transactions.clone());
        assert_eq!(filtered_transactions, transactions[3..].to_vec());
    }
}

#[test]
fn test_account_address_filter_multisig() {
    for use_new_txn_payload_format in [false, true] {
        // Create a filter that only allows transactions from specific account addresses.
        // These are: (i) txn 0 multisig address; (ii) txn 1 sender; and (iii) txn 2 multisig address.
        let transactions = utils::create_multisig_transactions(use_new_txn_payload_format);
        let filter = TransactionFilter::empty()
            .add_account_address_filter(true, utils::get_multisig_address(&transactions[0]))
            .add_account_address_filter(true, transactions[1].sender())
            .add_account_address_filter(true, utils::get_multisig_address(&transactions[2]))
            .add_all_filter(false);

        // Verify that the filter returns transactions from the specified account addresses
        let filtered_transactions = filter.filter_transactions(transactions.clone());
        assert_eq!(filtered_transactions, transactions[0..3].to_vec());

        // Create a filter that denies transactions from the specified account addresses (as above)
        let filter = TransactionFilter::empty()
            .add_account_address_filter(false, utils::get_multisig_address(&transactions[0]))
            .add_account_address_filter(false, transactions[1].sender())
            .add_account_address_filter(false, utils::get_multisig_address(&transactions[2]))
            .add_all_filter(true);

        // Verify that the filter returns transactions from other account addresses
        let filtered_transactions = filter.filter_transactions(transactions.clone());
        assert_eq!(filtered_transactions, transactions[3..].to_vec());
    }
}

#[test]
fn test_account_address_filter_script_argument() {
    for use_new_txn_payload_format in [false, true] {
        // Create a filter that only allows transactions from specific account addresses.
        // These are: (i) txn 0 script arg address; (ii) txn 1 sender; and (iii) txn 2 script arg address.
        let transactions = utils::create_script_transactions(use_new_txn_payload_format);
        let filter = TransactionFilter::empty()
            .add_account_address_filter(true, utils::get_script_argument_address(&transactions[0]))
            .add_account_address_filter(true, transactions[1].sender())
            .add_account_address_filter(true, utils::get_script_argument_address(&transactions[2]))
            .add_all_filter(false);

        // Verify that the filter returns transactions from the specified account addresses
        let filtered_transactions = filter.filter_transactions(transactions.clone());
        assert_eq!(filtered_transactions, transactions[0..3].to_vec());

        // Create a filter that denies transactions from the specified account addresses (as above)
        let filter = TransactionFilter::empty()
            .add_account_address_filter(false, utils::get_script_argument_address(&transactions[0]))
            .add_account_address_filter(false, transactions[1].sender())
            .add_account_address_filter(false, utils::get_script_argument_address(&transactions[2]))
            .add_all_filter(true);

        // Verify that the filter returns transactions from other account addresses
        let filtered_transactions = filter.filter_transactions(transactions.clone());
        assert_eq!(filtered_transactions, transactions[3..].to_vec());
    }
}

#[test]
fn test_account_address_filter_transaction_authenticator() {
    // Create a filter that only allows transactions from specific account addresses.
    // These are: (i) txn 0 account authenticator; (ii) txn 1 account authenticator; and (iii) txn 2 sender.
    let transactions = utils::create_fee_payer_transactions();
    let filter = TransactionFilter::empty()
        .add_account_address_filter(true, utils::get_fee_payer_address(&transactions[0]))
        .add_account_address_filter(true, utils::get_fee_payer_address(&transactions[1]))
        .add_account_address_filter(true, transactions[2].sender())
        .add_all_filter(false);

    // Verify that the filter returns transactions from the specified account addresses
    let filtered_transactions = filter.filter_transactions(transactions.clone());
    assert_eq!(filtered_transactions, transactions[0..3].to_vec());

    // Create a filter that denies transactions from the specified account addresses (as above)
    let filter = TransactionFilter::empty()
        .add_account_address_filter(false, utils::get_fee_payer_address(&transactions[0]))
        .add_account_address_filter(false, utils::get_fee_payer_address(&transactions[1]))
        .add_account_address_filter(false, transactions[2].sender())
        .add_all_filter(true);

    // Verify that the filter returns transactions from other account addresses
    let filtered_transactions = filter.filter_transactions(transactions.clone());
    assert_eq!(filtered_transactions, transactions[3..].to_vec());
}

#[test]
fn test_all_filter() {
    for use_new_txn_payload_format in [false, true] {
        // Create a filter that allows all transactions
        let filter = TransactionFilter::empty().add_all_filter(true);

        // Verify that all transactions are allowed
        let transactions = utils::create_entry_function_transactions(use_new_txn_payload_format);
        let filtered_transactions = filter.filter_transactions(transactions.clone());
        assert_eq!(filtered_transactions, transactions);

        // Create a filter that denies all transactions
        let filter = TransactionFilter::empty().add_all_filter(false);

        // Verify that all transactions are denied
        let filtered_transactions = filter.filter_transactions(transactions.clone());
        assert!(filtered_transactions.is_empty());
    }
}

#[test]
fn test_empty_filter() {
    for use_new_txn_payload_format in [false, true] {
        // Create an empty filter
        let filter = TransactionFilter::empty();

        // Verify that all transactions are allowed
        let transactions = utils::create_entry_function_transactions(use_new_txn_payload_format);
        let filtered_transactions = filter.filter_transactions(transactions.clone());
        assert_eq!(filtered_transactions, transactions);
    }
}

#[test]
fn test_entry_function_filter() {
    for use_new_txn_payload_format in [false, true] {
        // Create a filter that only allows transactions with specific entry functions (txn 0 and txn 1)
        let transactions = utils::create_entry_function_transactions(use_new_txn_payload_format);
        let filter = TransactionFilter::empty()
            .add_entry_function_filter(
                true,
                utils::get_module_address(&transactions[0]),
                utils::get_module_name(&transactions[0]),
                utils::get_function_name(&transactions[0]),
            )
            .add_entry_function_filter(
                true,
                utils::get_module_address(&transactions[1]),
                utils::get_module_name(&transactions[1]),
                utils::get_function_name(&transactions[1]),
            )
            .add_all_filter(false);

        // Verify that the filter returns only transactions with the specified entry functions
        let filtered_transactions = filter.filter_transactions(transactions.clone());
        assert_eq!(filtered_transactions, transactions[0..2].to_vec());

        // Create a filter that denies transactions with specific entry functions (txn 0)
        let filter = TransactionFilter::empty()
            .add_entry_function_filter(
                false,
                utils::get_module_address(&transactions[0]),
                utils::get_module_name(&transactions[0]),
                utils::get_function_name(&transactions[0]),
            )
            .add_all_filter(true);

        // Verify that the filter returns all transactions except those with the specified entry functions
        let filtered_transactions = filter.filter_transactions(transactions.clone());
        assert_eq!(filtered_transactions, transactions[1..].to_vec());
    }
}

#[test]
fn test_module_address_filter() {
    for use_new_txn_payload_format in [false, true] {
        // Create a filter that only allows transactions from a specific module address (txn 0 and txn 1)
        let transactions = utils::create_entry_function_transactions(use_new_txn_payload_format);
        let filter = TransactionFilter::empty()
            .add_module_address_filter(true, utils::get_module_address(&transactions[0]))
            .add_module_address_filter(true, utils::get_module_address(&transactions[1]))
            .add_all_filter(false);

        // Verify that the filter returns only transactions from the specified module addresses
        let filtered_transactions = filter.filter_transactions(transactions.clone());
        assert_eq!(filtered_transactions, transactions[0..2].to_vec());

        // Create a filter that denies transactions from a specific module address (txn 0 and txn 1)
        let filter = TransactionFilter::empty()
            .add_module_address_filter(false, utils::get_module_address(&transactions[0]))
            .add_module_address_filter(false, utils::get_module_address(&transactions[1]))
            .add_all_filter(true);

        // Verify that the filter returns all transactions except those from the specified module addresses
        let filtered_transactions = filter.filter_transactions(transactions.clone());
        assert_eq!(filtered_transactions, transactions[2..].to_vec());
    }
}

#[test]
fn test_multiple_matchers_filter() {
    for use_new_txn_payload_format in [false, true] {
        // Create a filter that only allows transactions with specific criteria (only txn 1 should match)
        let transactions = utils::create_entry_function_transactions(use_new_txn_payload_format);
        let transaction_matchers = vec![
            TransactionMatcher::Sender(transactions[1].sender()),
            TransactionMatcher::ModuleAddress(utils::get_module_address(&transactions[1])),
            TransactionMatcher::EntryFunction(
                utils::get_module_address(&transactions[1]),
                utils::get_module_name(&transactions[1]),
                utils::get_function_name(&transactions[1]),
            ),
        ];
        let filter = TransactionFilter::empty()
            .add_multiple_matchers_filter(true, transaction_matchers.clone())
            .add_all_filter(false);

        // Verify that the filter returns only transactions that match all specified matchers
        let filtered_transactions = filter.filter_transactions(transactions.clone());
        assert_eq!(filtered_transactions, vec![transactions[1].clone()]);

        // Create a filter that only allows transactions with a specific criteria (none should match)
        let transaction_matchers = vec![
            TransactionMatcher::Sender(transactions[0].sender()),
            TransactionMatcher::ModuleAddress(utils::get_module_address(&transactions[1])),
            TransactionMatcher::ModuleAddress(utils::get_module_address(&transactions[2])),
        ];
        let filter = TransactionFilter::empty()
            .add_multiple_matchers_filter(true, transaction_matchers)
            .add_all_filter(false);

        // Verify that the filter returns no transactions (none should match)
        let filtered_transactions = filter.filter_transactions(transactions.clone());
        assert!(filtered_transactions.is_empty());

        // Create a filter that denies transactions with a specific sender and module address (txn 0)
        let transaction_matchers = vec![
            TransactionMatcher::Sender(transactions[0].sender()),
            TransactionMatcher::ModuleAddress(utils::get_module_address(&transactions[0])),
        ];
        let filter = TransactionFilter::empty()
            .add_multiple_matchers_filter(false, transaction_matchers)
            .add_all_filter(true);

        // Verify that it returns all transactions except those with the specified sender and module address
        let filtered_transactions = filter.filter_transactions(transactions.clone());
        assert_eq!(filtered_transactions, transactions[1..].to_vec());
    }
}

#[test]
fn test_public_key_filter() {
    for use_new_txn_payload_format in [false, true] {
        // Create a filter that only allows transactions from specific public keys.
        // These are: (i) txn 0 authenticator public key; and (ii) txn 1 authenticator public key.
        let transactions = utils::create_entry_function_transactions(use_new_txn_payload_format);
        let filter = TransactionFilter::empty()
            .add_public_key_filter(true, utils::get_auth_public_key(&transactions[0]))
            .add_public_key_filter(true, utils::get_auth_public_key(&transactions[1]))
            .add_all_filter(false);

        // Verify that the filter returns transactions with the specified public keys
        let filtered_transactions = filter.filter_transactions(transactions.clone());
        assert_eq!(filtered_transactions, transactions[0..2].to_vec());

        // Create a filter that denies transactions from the specified public keys (as above)
        let filter = TransactionFilter::empty()
            .add_public_key_filter(false, utils::get_auth_public_key(&transactions[0]))
            .add_public_key_filter(false, utils::get_auth_public_key(&transactions[1]))
            .add_all_filter(true);

        // Verify that it returns transactions from other public keys
        let filtered_transactions = filter.filter_transactions(transactions.clone());
        assert_eq!(filtered_transactions, transactions[2..].to_vec());
    }
}

#[test]
fn test_sender_filter() {
    for use_new_txn_payload_format in [false, true] {
        // Create a filter that only allows transactions from a specific sender (txn 0 and txn 1)
        let transactions = utils::create_entry_function_transactions(use_new_txn_payload_format);
        let filter = TransactionFilter::empty()
            .add_sender_filter(true, transactions[0].sender())
            .add_sender_filter(true, transactions[1].sender())
            .add_all_filter(false);

        // Verify that the filter returns only transactions from the specified senders
        let filtered_transactions = filter.filter_transactions(transactions.clone());
        assert_eq!(filtered_transactions, transactions[0..2].to_vec());

        // Create a filter that denies transactions from a specific sender (txn 0 and txn 1)
        let filter = TransactionFilter::empty()
            .add_sender_filter(false, transactions[0].sender())
            .add_sender_filter(false, transactions[1].sender())
            .add_all_filter(true);

        // Verify that the filter returns all transactions except those from the specified senders
        let filtered_transactions = filter.filter_transactions(transactions.clone());
        assert_eq!(filtered_transactions, transactions[2..].to_vec());
    }
}

#[test]
fn test_transaction_id_filter() {
    for use_new_txn_payload_format in [false, true] {
        // Create a filter that only allows transactions with a specific transaction ID (txn 0)
        let transactions = utils::create_entry_function_transactions(use_new_txn_payload_format);
        let filter = TransactionFilter::empty()
            .add_transaction_id_filter(true, transactions[0].committed_hash())
            .add_all_filter(false);

        // Verify that the filter returns only the transaction with the specified ID
        let filtered_transactions = filter.filter_transactions(transactions.clone());
        assert_eq!(filtered_transactions, vec![transactions[0].clone()]);

        // Create a filter that denies transactions with a specific transaction ID (txn 0)
        let filter = TransactionFilter::empty()
            .add_transaction_id_filter(false, transactions[0].committed_hash())
            .add_all_filter(true);

        // Verify that the filter returns all transactions except the one with the specified ID
        let filtered_transactions = filter.filter_transactions(transactions.clone());
        assert_eq!(filtered_transactions, transactions[1..].to_vec());
    }
}
