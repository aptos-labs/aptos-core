// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{tests::utils, transaction_filter::TransactionFilter};

#[test]
fn test_transaction_filter_config_allow() {
    for use_new_txn_payload_format in [false, true] {
        // Create a filter that only allows transactions based on multiple criteria
        let transactions = utils::create_entry_function_transactions(use_new_txn_payload_format);
        let transaction_filter_string = format!(
            r#"
            transaction_rules:
                - Allow:
                    - Sender: "{}"
                - Allow:
                    - ModuleAddress: "0000000000000000000000000000000000000000000000000000000000000001"
                - Allow:
                    - PublicKey:
                        Ed25519:
                            - "{}"
                - Allow:
                    - EntryFunction:
                        - "0000000000000000000000000000000000000000000000000000000000000003"
                        - entry
                        - sub
                - Allow:
                    - AccountAddress: "{}"
                - Deny:
                    - All
          "#,
            transactions[0].sender().to_standard_string(),
            utils::get_ed25519_public_key(&transactions[2]),
            utils::get_module_address(&transactions[4]).to_standard_string(),
        );
        let transaction_filter =
            serde_yaml::from_str::<TransactionFilter>(&transaction_filter_string).unwrap();

        // Verify that only the first five transactions are allowed
        let filtered_transactions = transaction_filter.filter_transactions(transactions.clone());
        assert_eq!(filtered_transactions, transactions[0..5].to_vec());
    }
}

#[test]
fn test_transaction_filter_config_deny() {
    for use_new_txn_payload_format in [false, true] {
        // Create a filter that denies transactions based on multiple criteria
        let transactions = utils::create_entry_function_transactions(use_new_txn_payload_format);
        let transaction_filter_string = format!(
            r#"
            transaction_rules:
                - Deny:
                    - ModuleAddress: "0000000000000000000000000000000000000000000000000000000000000000"
                - Deny:
                    - Sender: "{}"
                - Deny:
                    - EntryFunction:
                        - "0000000000000000000000000000000000000000000000000000000000000002"
                        - entry
                        - new
                - Deny:
                    - ModuleAddress: "0000000000000000000000000000000000000000000000000000000000000003"
                - Deny:
                    - AccountAddress: "{}"
                - Allow:
                    - All
          "#,
            transactions[1].sender().to_standard_string(),
            utils::get_module_address(&transactions[4]).to_standard_string(),
        );
        let transaction_filter =
            serde_yaml::from_str::<TransactionFilter>(&transaction_filter_string).unwrap();

        // Verify that the first five transactions are denied
        let filtered_transactions = transaction_filter.filter_transactions(transactions.clone());
        assert_eq!(filtered_transactions, transactions[5..].to_vec());
    }
}

#[test]
fn test_transaction_filter_config_multiple_matchers() {
    for use_new_txn_payload_format in [false, true] {
        // Create a filter that denies transactions based on multiple matching criteria
        let transactions = utils::create_entry_function_transactions(use_new_txn_payload_format);
        let transaction_filter_string = format!(
            r#"
            transaction_rules:
                - Deny:
                    - Sender: "{}"
                    - ModuleAddress: "0000000000000000000000000000000000000000000000000000000000000000"
                - Deny:
                    - Sender: "{}"
                    - ModuleAddress: "0000000000000000000000000000000000000000000000000000000000000001"
                - Deny:
                    - Sender: "{}"
                    - EntryFunction:
                        - "0000000000000000000000000000000000000000000000000000000000000002"
                        - entry
                        - new
                - Deny:
                    - Sender: "{}"
                    - AccountAddress: "{}"
                - Allow:
                    - All
          "#,
            transactions[0].sender().to_standard_string(),
            transactions[1].sender().to_standard_string(),
            transactions[2].sender().to_standard_string(),
            transactions[3].sender().to_standard_string(),
            utils::get_module_address(&transactions[3]).to_standard_string(),
        );
        let transaction_filter =
            serde_yaml::from_str::<TransactionFilter>(&transaction_filter_string).unwrap();

        // Verify that the first four transactions are denied
        let filtered_transactions = transaction_filter.filter_transactions(transactions.clone());
        assert_eq!(filtered_transactions, transactions[4..].to_vec());
    }
}
