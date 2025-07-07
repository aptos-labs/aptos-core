// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{batch_transaction_filter::BatchTransactionFilter, tests::utils};
use aptos_types::{quorum_store::BatchId, PeerId};

#[test]
fn test_batch_transaction_filter_config_allow() {
    for use_new_txn_payload_format in [false, true] {
        // Create a filter that only allows transactions based on multiple criteria
        let transactions = utils::create_entry_function_transactions(use_new_txn_payload_format);
        let batch_transaction_filter_string = format!(
            r#"
            batch_transaction_rules:
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
                    - Batch:
                        All
          "#,
            transactions[0].sender().to_standard_string(),
            utils::get_ed25519_public_key(&transactions[2]),
            utils::get_module_address(&transactions[4]).to_standard_string(),
        );
        let batch_transaction_filter =
            serde_yaml::from_str::<BatchTransactionFilter>(&batch_transaction_filter_string)
                .unwrap();

        // Create a batch ID, author and digest
        let (batch_id, batch_author, batch_digest) = utils::get_random_batch_info();

        // Verify that only the first five transactions are allowed
        let filtered_transactions = batch_transaction_filter.filter_batch_transactions(
            batch_id,
            batch_author,
            batch_digest,
            transactions.clone(),
        );
        assert_eq!(filtered_transactions, transactions[0..5].to_vec());
    }
}

#[test]
fn test_batch_transaction_filter_config_deny() {
    for use_new_txn_payload_format in [false, true] {
        // Create a filter that denies transactions based on multiple criteria
        let transactions = utils::create_entry_function_transactions(use_new_txn_payload_format);
        let batch_transaction_filter_string = format!(
            r#"
            batch_transaction_rules:
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
        let batch_transaction_filter =
            serde_yaml::from_str::<BatchTransactionFilter>(&batch_transaction_filter_string)
                .unwrap();

        // Create a batch ID, author and digest
        let (batch_id, batch_author, batch_digest) = utils::get_random_batch_info();

        // Verify that the first five transactions are denied
        let filtered_transactions = batch_transaction_filter.filter_batch_transactions(
            batch_id,
            batch_author,
            batch_digest,
            transactions.clone(),
        );
        assert_eq!(filtered_transactions, transactions[5..].to_vec());
    }
}

#[test]
fn test_batch_transaction_filter_config_multiple_matchers() {
    for use_new_txn_payload_format in [false, true] {
        // Create a batch ID, author and digest
        let (batch_id, batch_author, batch_digest) = utils::get_random_batch_info();

        // Create a filter that denies transactions based on multiple criteria
        let transactions = utils::create_entry_function_transactions(use_new_txn_payload_format);
        let batch_transaction_filter_string = format!(
            r#"
            batch_transaction_rules:
                - Deny:
                    - Transaction:
                        Sender: "{}"
                    - Transaction:
                        ModuleAddress: "0000000000000000000000000000000000000000000000000000000000000000"
                    - Batch:
                        BatchId:
                            id: {}
                            nonce: 0
                - Deny:
                    - Transaction:
                        Sender: "{}"
                    - Transaction:
                        EntryFunction:
                            - "0000000000000000000000000000000000000000000000000000000000000001"
                            - entry
                            - check
                    - Batch:
                        BatchId:
                            id: {}
                            nonce: 0
                - Deny:
                    - Transaction:
                        Sender: "{}"
                    - Batch:
                        BatchId:
                            id: 999
                            nonce: 0
                - Deny:
                    - Batch:
                        BatchAuthor: {}
                - Allow:
                    - Transaction:
                        All
          "#,
            transactions[0].sender().to_standard_string(),
            batch_id.id,
            transactions[1].sender().to_standard_string(),
            batch_id.id,
            transactions[2].sender().to_standard_string(),
            batch_author.to_standard_string(),
        );
        let batch_transaction_filter =
            serde_yaml::from_str::<BatchTransactionFilter>(&batch_transaction_filter_string)
                .unwrap();

        // Verify that the first two transactions are denied when the batch has a different author
        let filtered_transactions = batch_transaction_filter.filter_batch_transactions(
            batch_id,
            PeerId::random(), // Use a different author
            batch_digest,
            transactions.clone(),
        );
        assert_eq!(filtered_transactions, transactions[2..].to_vec());

        // Verify that all transactions are denied when the batch has the rejected author
        let filtered_transactions = batch_transaction_filter.filter_batch_transactions(
            batch_id,
            batch_author,
            batch_digest,
            transactions.clone(),
        );
        assert!(filtered_transactions.is_empty());

        // Verify that all transactions are allowed in a different batch (different author and ID)
        let filtered_transactions = batch_transaction_filter.filter_batch_transactions(
            BatchId::new_for_test(0), // Use a different batch ID
            PeerId::random(),         // Use a different author
            batch_digest,
            transactions.clone(),
        );
        assert_eq!(filtered_transactions, transactions);
    }
}
