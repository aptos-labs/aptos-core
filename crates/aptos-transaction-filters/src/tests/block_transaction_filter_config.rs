// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{block_transaction_filter::BlockTransactionFilter, tests::utils};
use aptos_crypto::HashValue;
use move_core_types::account_address::AccountAddress;

#[test]
fn test_block_transaction_filter_config_allow() {
    for use_new_txn_payload_format in [false, true] {
        // Create a filter that only allows transactions based on multiple criteria
        let transactions = utils::create_entry_function_transactions(use_new_txn_payload_format);
        let block_transaction_filter_string = format!(
            r#"
            block_transaction_rules:
                - Allow:
                    - Transaction:
                        Sender: "{}"
                - Allow:
                    - Transaction:
                        ModuleAddress: "0000000000000000000000000000000000000000000000000000000000000001"
                - Allow:
                    - Transaction:
                        PublicKey:
                            Ed25519:
                                - "{}"
                - Allow:
                    - Transaction:
                        EntryFunction:
                            - "0000000000000000000000000000000000000000000000000000000000000003"
                            - entry
                            - sub
                - Allow:
                    - Transaction:
                        AccountAddress: "{}"
                - Deny:
                    - Block:
                        All
          "#,
            transactions[0].sender().to_standard_string(),
            utils::get_ed25519_public_key(&transactions[2]),
            utils::get_module_address(&transactions[4]).to_standard_string(),
        );
        let block_transaction_filter =
            serde_yaml::from_str::<BlockTransactionFilter>(&block_transaction_filter_string)
                .unwrap();

        // Create a block ID, author, epoch, and timestamp
        let (block_id, block_author, block_epoch, block_timestamp) = utils::get_random_block_info();

        // Verify that only the first five transactions are allowed
        let filtered_transactions = block_transaction_filter.filter_block_transactions(
            block_id,
            Some(block_author),
            block_epoch,
            block_timestamp,
            transactions.clone(),
        );
        assert_eq!(filtered_transactions, transactions[0..5].to_vec());
    }
}

#[test]
fn test_block_transaction_filter_config_deny() {
    for use_new_txn_payload_format in [false, true] {
        // Create a filter that denies transactions based on multiple criteria
        let transactions = utils::create_entry_function_transactions(use_new_txn_payload_format);
        let block_transaction_filter_string = format!(
            r#"
            block_transaction_rules:
                - Deny:
                    - Transaction:
                        ModuleAddress: "0000000000000000000000000000000000000000000000000000000000000000"
                - Deny:
                    - Transaction:
                        Sender: "{}"
                - Deny:
                    - Transaction:
                        EntryFunction:
                            - "0000000000000000000000000000000000000000000000000000000000000002"
                            - entry
                            - new
                - Deny:
                    - Transaction:
                        ModuleAddress: "0000000000000000000000000000000000000000000000000000000000000003"
                - Deny:
                    - Transaction:
                        AccountAddress: "{}"
                - Allow:
                    - Transaction:
                        All
          "#,
            transactions[1].sender().to_standard_string(),
            utils::get_module_address(&transactions[4]).to_standard_string(),
        );
        let block_transaction_filter =
            serde_yaml::from_str::<BlockTransactionFilter>(&block_transaction_filter_string)
                .unwrap();

        // Create a block ID, author, epoch, and timestamp
        let (block_id, block_author, block_epoch, block_timestamp) = utils::get_random_block_info();

        // Verify that the first five transactions are denied
        let filtered_transactions = block_transaction_filter.filter_block_transactions(
            block_id,
            Some(block_author),
            block_epoch,
            block_timestamp,
            transactions.clone(),
        );
        assert_eq!(filtered_transactions, transactions[5..].to_vec());
    }
}

#[test]
fn test_block_transaction_filter_config_multiple_matchers() {
    for use_new_txn_payload_format in [false, true] {
        // Create a block ID, author, epoch, and timestamp
        let (block_id, block_author, block_epoch, block_timestamp) = utils::get_random_block_info();

        // Create a malicious block author (where blocks are not allowed)
        let malicious_block_author = AccountAddress::random();

        // Create a filter that denies transactions based on multiple criteria
        let transactions = utils::create_entry_function_transactions(use_new_txn_payload_format);
        let block_transaction_filter_string = format!(
            r#"
            block_transaction_rules:
                - Deny:
                    - Transaction:
                        Sender: "{}"
                    - Transaction:
                        ModuleAddress: "0000000000000000000000000000000000000000000000000000000000000000"
                    - Block:
                        BlockId: "{}"
                - Deny:
                    - Transaction:
                        Sender: "{}"
                    - Transaction:
                        EntryFunction:
                            - "0000000000000000000000000000000000000000000000000000000000000001"
                            - entry
                            - check
                    - Block:
                        BlockId: "{}"
                - Deny:
                    - Transaction:
                        Sender: "{}"
                    - Block:
                        BlockId: "0000000000000000000000000000000000000000000000000000000000000000"
                - Deny:
                    - Block:
                        Author: "{}"
                - Deny:
                    - Block:
                        BlockEpochLessThan: {}
                - Allow:
                    - Transaction:
                        All
          "#,
            transactions[0].sender().to_standard_string(),
            block_id.to_hex(),
            transactions[1].sender().to_standard_string(),
            block_id.to_hex(),
            transactions[2].sender().to_standard_string(),
            malicious_block_author,
            block_epoch,
        );
        let block_transaction_filter =
            serde_yaml::from_str::<BlockTransactionFilter>(&block_transaction_filter_string)
                .unwrap();

        // Verify that the first two transactions are denied in the current block
        let filtered_transactions = block_transaction_filter.filter_block_transactions(
            block_id,
            Some(block_author),
            block_epoch,
            block_timestamp,
            transactions.clone(),
        );
        assert_eq!(filtered_transactions, transactions[2..].to_vec());

        // Verify that all transactions are denied in the previous block
        let filtered_transactions = block_transaction_filter.filter_block_transactions(
            HashValue::random(),
            Some(block_author),
            block_epoch - 1,
            block_timestamp - 1,
            transactions.clone(),
        );
        assert!(filtered_transactions.is_empty());

        // Verify that all transactions are allowed in a completely different block
        let filtered_transactions = block_transaction_filter.filter_block_transactions(
            HashValue::random(),
            Some(block_author),
            block_epoch + 1,
            block_timestamp + 1,
            transactions.clone(),
        );
        assert_eq!(filtered_transactions, transactions);

        // Verify that all transactions are denied in a block with a malicious author
        let filtered_transactions = block_transaction_filter.filter_block_transactions(
            HashValue::random(),
            Some(malicious_block_author),
            block_epoch + 1,
            block_timestamp + 1,
            transactions.clone(),
        );
        assert!(filtered_transactions.is_empty());
    }
}
