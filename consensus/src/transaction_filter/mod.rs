// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_config::config::transaction_filter_type::Filter;
use aptos_crypto::HashValue;
use aptos_types::transaction::SignedTransaction;

pub struct TransactionFilter {
    filter: Filter,
}

impl TransactionFilter {
    pub(crate) fn new(filter: Filter) -> Self {
        Self { filter }
    }

    pub fn filter(
        &self,
        block_id: HashValue,
        block_epoch: u64,
        block_timestamp: u64,
        txns: Vec<SignedTransaction>,
    ) -> Vec<SignedTransaction> {
        // Special case for no filter to avoid unnecessary iteration through all transactions in the default case
        if self.filter.is_empty() {
            return txns;
        }

        txns.into_iter()
            .filter(|txn| {
                self.filter
                    .allows(block_id, block_epoch, block_timestamp, txn)
            })
            .collect()
    }
}

#[cfg(test)]
mod test {
    use crate::transaction_filter::TransactionFilter;
    use aptos_config::config::transaction_filter_type::{Filter, Matcher};
    use aptos_crypto::{ed25519::Ed25519PrivateKey, HashValue, PrivateKey, SigningKey, Uniform};
    use aptos_types::{
        chain_id::ChainId,
        move_utils::MemberId,
        transaction::{
            EntryFunction, RawTransaction, SignedTransaction, TransactionExecutableRef,
            TransactionPayload,
        },
    };
    use move_core_types::account_address::AccountAddress;

    fn create_signed_transaction(function: MemberId) -> SignedTransaction {
        let private_key = Ed25519PrivateKey::generate_for_testing();
        let public_key = private_key.public_key();
        let sender = AccountAddress::random();
        let sequence_number = 0;
        let MemberId {
            module_id,
            member_id: function_id,
        } = function;

        // TODO[Orderless]: Test with payload v2 format as well.
        let payload = TransactionPayload::EntryFunction(EntryFunction::new(
            module_id,
            function_id,
            vec![],
            vec![],
        ));
        let raw_transaction =
            RawTransaction::new(sender, sequence_number, payload, 0, 0, 0, ChainId::new(10));

        SignedTransaction::new(
            raw_transaction.clone(),
            public_key.clone(),
            private_key.sign(&raw_transaction).unwrap(),
        )
    }

    fn get_transactions() -> Vec<SignedTransaction> {
        vec![
            create_signed_transaction(str::parse("0x0::test0::add").unwrap()),
            create_signed_transaction(str::parse("0x1::test1::check").unwrap()),
            create_signed_transaction(str::parse("0x2::test2::new").unwrap()),
            create_signed_transaction(str::parse("0x3::test3::sub").unwrap()),
            create_signed_transaction(str::parse("0x4::test4::mul").unwrap()),
            create_signed_transaction(str::parse("0x5::test5::div").unwrap()),
            create_signed_transaction(str::parse("0x6::test6::mod").unwrap()),
        ]
    }

    fn get_block_id_and_transactions() -> (HashValue, Vec<SignedTransaction>) {
        let txns = get_transactions();
        let block_id = HashValue::random();
        (block_id, txns)
    }

    fn get_module_address(txn: &SignedTransaction) -> AccountAddress {
        match txn.payload().executable_ref() {
            Ok(TransactionExecutableRef::EntryFunction(entry_func)) => {
                *entry_func.module().address()
            },
            _ => panic!("Unexpected transaction payload"),
        }
    }

    fn get_module_name(txn: &SignedTransaction) -> String {
        match txn.payload().executable_ref() {
            Ok(TransactionExecutableRef::EntryFunction(entry_func)) => {
                entry_func.module().name().to_string()
            },
            _ => panic!("Unexpected transaction payload"),
        }
    }

    fn get_function_name(txn: &SignedTransaction) -> String {
        match txn.payload().executable_ref() {
            Ok(TransactionExecutableRef::EntryFunction(entry_func)) => {
                entry_func.function().to_string()
            },
            _ => panic!("Unexpected transaction payload"),
        }
    }

    #[test]
    fn test_empty_filter() {
        // Create an empty filter
        let filter = TransactionFilter::new(Filter::empty());

        // Verify that it returns all transactions
        let (block_id, txns) = get_block_id_and_transactions();
        let filtered_txns = filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, txns);
    }

    #[test]
    fn test_all_filter() {
        // Create a filter that allows all transactions
        let filter = TransactionFilter::new(Filter::empty().add_all_filter(true));

        // Verify that it returns all transactions
        let (block_id, txns) = get_block_id_and_transactions();
        let filtered_txns = filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, txns);

        // Create a filter that denies all transactions
        let filter = TransactionFilter::new(Filter::empty().add_all_filter(false));

        // Verify that it returns no transactions
        let filtered_txns = filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, vec![]);
    }

    #[test]
    fn test_block_id_filter() {
        // Create a filter that only allows transactions with a specific block ID
        let (block_id, txns) = get_block_id_and_transactions();
        let filter = TransactionFilter::new(
            Filter::empty()
                .add_block_id_filter(true, block_id)
                .add_all_filter(false),
        );

        // Verify that it returns all transactions with the specified block ID
        let filtered_txns = filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, txns);

        // Verify that it returns no transactions with a different block ID
        let different_block_id = HashValue::random();
        let filtered_txns = filter.filter(different_block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, vec![]);

        // Create a filter that denies transactions with a specific block ID
        let filter = TransactionFilter::new(
            Filter::empty()
                .add_block_id_filter(false, block_id)
                .add_all_filter(true),
        );

        // Verify that it returns all transactions except those with the specified block ID
        let filtered_txns = filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, vec![]);

        // Verify that it returns all transactions with a different block ID
        let different_block_id = HashValue::random();
        let filtered_txns = filter.filter(different_block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, txns);
    }

    #[test]
    fn test_block_timestamp_greater_than_filter() {
        // Create a filter that only allows transactions with a block timestamp greater than a specific value
        let filter = TransactionFilter::new(
            Filter::empty()
                .add_block_timestamp_greater_than_filter(true, 1000)
                .add_all_filter(false),
        );

        // Verify that it returns no transactions with a block timestamp less than or equal to 1000
        let (block_id, txns) = get_block_id_and_transactions();
        for block_timestamp in [0, 999, 1000] {
            let filtered_txns = filter.filter(block_id, 0, block_timestamp, txns.clone());
            assert_eq!(filtered_txns, vec![]);
        }

        // Verify that it returns all transactions with a block timestamp greater than 1000
        let filtered_txns = filter.filter(block_id, 0, 1001, txns.clone());
        assert_eq!(filtered_txns, txns);

        // Create a filter that denies transactions with a block timestamp greater than a specific value
        let filter = TransactionFilter::new(
            Filter::empty()
                .add_block_timestamp_greater_than_filter(false, 1000)
                .add_all_filter(true),
        );

        // Verify that it returns all transactions with a block timestamp less than or equal to 1000
        for block_timestamp in [0, 999, 1000] {
            let filtered_txns = filter.filter(block_id, 0, block_timestamp, txns.clone());
            assert_eq!(filtered_txns, txns);
        }

        // Verify that it returns no transactions with a block timestamp greater than 1000
        let filtered_txns = filter.filter(block_id, 0, 1001, txns.clone());
        assert_eq!(filtered_txns, vec![]);
    }

    #[test]
    fn test_block_timestamp_less_than_filter() {
        // Create a filter that only allows transactions with a block timestamp less than a specific value
        let filter = TransactionFilter::new(
            Filter::empty()
                .add_block_timestamp_less_than_filter(true, 1000)
                .add_all_filter(false),
        );

        // Verify that it returns all transactions with a block timestamp less than 1000
        let (block_id, txns) = get_block_id_and_transactions();
        let filtered_txns = filter.filter(block_id, 0, 999, txns.clone());
        assert_eq!(filtered_txns, txns);

        // Verify that it returns no transactions with a block timestamp greater than or equal to 1000
        for block_timestamp in [1000, 1001] {
            let filtered_txns = filter.filter(block_id, 0, block_timestamp, txns.clone());
            assert_eq!(filtered_txns, vec![]);
        }

        // Create a filter that denies transactions with a block timestamp less than a specific value
        let filter = TransactionFilter::new(
            Filter::empty()
                .add_block_timestamp_less_than_filter(false, 1000)
                .add_all_filter(true),
        );

        // Verify that it returns no transactions with a block timestamp less than 1000
        let filtered_txns = filter.filter(block_id, 0, 999, txns.clone());
        assert_eq!(filtered_txns, vec![]);

        // Verify that it returns all transactions with a block timestamp greater than or equal to 1000
        for block_timestamp in [1000, 1001] {
            let filtered_txns = filter.filter(block_id, 0, block_timestamp, txns.clone());
            assert_eq!(filtered_txns, txns);
        }
    }

    #[test]
    fn test_transaction_id_filter() {
        // Create a filter that only allows transactions with a specific transaction ID (txn 0)
        let (block_id, txns) = get_block_id_and_transactions();
        let filter = TransactionFilter::new(
            Filter::empty()
                .add_transaction_id_filter(true, txns[0].committed_hash())
                .add_all_filter(false),
        );

        // Verify that it returns the transaction with the specified ID
        let filtered_txns = filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, vec![txns[0].clone()]);

        // Create a filter that denies transactions with a specific transaction ID (txn 0)
        let filter = TransactionFilter::new(
            Filter::empty()
                .add_transaction_id_filter(false, txns[0].committed_hash())
                .add_all_filter(true),
        );

        // Verify that it returns all transactions except the one with the specified ID
        let filtered_txns = filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, txns[1..].to_vec());
    }

    #[test]
    fn test_sender_filter() {
        // Create a filter that only allows transactions from a specific sender (txn 0 and txn 1)
        let (block_id, txns) = get_block_id_and_transactions();
        let filter = TransactionFilter::new(
            Filter::empty()
                .add_sender_filter(true, txns[0].sender())
                .add_sender_filter(true, txns[1].sender())
                .add_all_filter(false),
        );

        // Verify that it returns transactions from the specified senders
        let filtered_txns = filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, txns[0..2].to_vec());

        // Create a filter that denies transactions from a specific sender (txn 0, txn 1 and txn 2)
        let filter = TransactionFilter::new(
            Filter::empty()
                .add_sender_filter(false, txns[0].sender())
                .add_sender_filter(false, txns[1].sender())
                .add_sender_filter(false, txns[2].sender()),
        );

        // Verify that it returns transactions from other senders
        let filtered_txns = filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, txns[3..].to_vec());
    }

    #[test]
    fn test_module_address_filter() {
        // Create a filter that only allows transactions from a specific module address (txn 0 and txn 1)
        let (block_id, txns) = get_block_id_and_transactions();
        let filter = TransactionFilter::new(
            Filter::empty()
                .add_module_address_filter(true, get_module_address(&txns[0]))
                .add_module_address_filter(true, get_module_address(&txns[1]))
                .add_all_filter(false),
        );

        // Verify that it returns transactions from the specified module addresses
        let filtered_txns = filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, txns[0..2].to_vec());

        // Create a filter that denies transactions from a specific module address (txn 0)
        let filter = TransactionFilter::new(
            Filter::empty().add_module_address_filter(false, get_module_address(&txns[0])),
        );

        // Verify that it returns transactions from other module addresses
        let filtered_txns = filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, txns[1..].to_vec());
    }

    #[test]
    fn test_entry_function_filter() {
        // Create a filter that only allows transactions with specific entry functions (txn 0 and txn 1)
        let filter = TransactionFilter::new(
            Filter::empty()
                .add_entry_function_filter(
                    true,
                    get_module_address(&get_transactions()[0]),
                    get_module_name(&get_transactions()[0]),
                    get_function_name(&get_transactions()[0]),
                )
                .add_entry_function_filter(
                    true,
                    get_module_address(&get_transactions()[1]),
                    get_module_name(&get_transactions()[1]),
                    get_function_name(&get_transactions()[1]),
                )
                .add_all_filter(false),
        );

        // Verify that it returns transactions with the specified entry functions
        let (block_id, txns) = get_block_id_and_transactions();
        let filtered_txns = filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, txns[0..2].to_vec());

        // Create a filter that denies transactions with specific entry functions (txn 0)
        let filter = TransactionFilter::new(Filter::empty().add_entry_function_filter(
            false,
            get_module_address(&get_transactions()[0]),
            get_module_name(&get_transactions()[0]),
            get_function_name(&get_transactions()[0]),
        ));

        // Verify that it returns transactions with other entry functions
        let filtered_txns = filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, txns[1..].to_vec());
    }

    #[test]
    fn test_block_epoch_greater_than_filter() {
        // Create a filter that only allows transactions with a block epoch greater than a specific value
        let filter = TransactionFilter::new(
            Filter::empty()
                .add_block_epoch_greater_than_filter(true, 1000)
                .add_all_filter(false),
        );

        // Verify that it returns no transactions with a block epoch less than or equal to 1000
        let (block_id, txns) = get_block_id_and_transactions();
        for block_epoch in [0, 999, 1000] {
            let filtered_txns = filter.filter(block_id, block_epoch, 0, txns.clone());
            assert_eq!(filtered_txns, vec![]);
        }

        // Verify that it returns all transactions with a block epoch greater than 1000
        let filtered_txns = filter.filter(block_id, 1001, 0, txns.clone());
        assert_eq!(filtered_txns, txns);

        // Create a filter that denies transactions with a block epoch greater than a specific value
        let filter = TransactionFilter::new(
            Filter::empty()
                .add_block_epoch_greater_than_filter(false, 1000)
                .add_all_filter(true),
        );

        // Verify that it returns all transactions with a block epoch less than or equal to 1000
        for block_epoch in [0, 999, 1000] {
            let filtered_txns = filter.filter(block_id, block_epoch, 0, txns.clone());
            assert_eq!(filtered_txns, txns);
        }

        // Verify that it returns no transactions with a block epoch greater than 1000
        let filtered_txns = filter.filter(block_id, 1001, 0, txns.clone());
        assert_eq!(filtered_txns, vec![]);
    }

    #[test]
    fn test_block_epoch_less_than_filter() {
        // Create a filter that only allows transactions with a block epoch less than a specific value
        let filter = TransactionFilter::new(
            Filter::empty()
                .add_block_epoch_less_than_filter(true, 1000)
                .add_all_filter(false),
        );

        // Verify that it returns all transactions with a block epoch less than 1000
        let (block_id, txns) = get_block_id_and_transactions();
        let filtered_txns = filter.filter(block_id, 999, 0, txns.clone());
        assert_eq!(filtered_txns, txns);

        // Verify that it returns no transactions with a block epoch greater than or equal to 1000
        for block_epoch in [1000, 1001] {
            let filtered_txns = filter.filter(block_id, block_epoch, 0, txns.clone());
            assert_eq!(filtered_txns, vec![]);
        }

        // Create a filter that denies transactions with a block epoch less than a specific value
        let filter = TransactionFilter::new(
            Filter::empty()
                .add_block_epoch_less_than_filter(false, 1000)
                .add_all_filter(true),
        );

        // Verify that it returns no transactions with a block epoch less than 1000
        let filtered_txns = filter.filter(block_id, 999, 0, txns.clone());
        assert_eq!(filtered_txns, vec![]);

        // Verify that it returns all transactions with a block epoch greater than or equal to 1000
        for block_epoch in [1000, 1001] {
            let filtered_txns = filter.filter(block_id, block_epoch, 0, txns.clone());
            assert_eq!(filtered_txns, txns);
        }
    }

    #[test]
    fn test_matches_all_of_filter() {
        // Create a filter that only matches transactions with epoch greater than 1000 and a specific sender (only txn 0)
        let (block_id, txns) = get_block_id_and_transactions();
        let matchers = vec![
            Matcher::BlockEpochGreaterThan(1000),
            Matcher::Sender(txns[0].sender()),
        ];
        let filter = TransactionFilter::new(
            Filter::empty()
                .add_matches_all_of_filter(true, matchers)
                .add_all_filter(false),
        );

        // Verify that it returns no transactions with block epoch less than or equal to 1000
        for block_epoch in [0, 999, 1000] {
            let filtered_txns = filter.filter(block_id, block_epoch, 0, txns.clone());
            assert_eq!(filtered_txns, vec![]);
        }

        // Verify that it returns transactions with block epoch greater than 1000 and the specified sender
        let filtered_txns = filter.filter(block_id, 1001, 0, txns.clone());
        assert_eq!(filtered_txns, txns[0..1].to_vec());

        // Create a filter that denies transactions with timestamp greater than 1000 and a specific sender (only txn 0)
        let matchers = vec![
            Matcher::BlockTimeStampGreaterThan(1000),
            Matcher::Sender(txns[0].sender()),
        ];
        let filter = TransactionFilter::new(
            Filter::empty()
                .add_matches_all_of_filter(false, matchers)
                .add_all_filter(true),
        );

        // Verify that it returns all transactions with block timestamp less than or equal to 1000
        for block_timestamp in [0, 999, 1000] {
            let filtered_txns = filter.filter(block_id, 0, block_timestamp, txns.clone());
            assert_eq!(filtered_txns, txns);
        }

        // Verify that it returns no transactions with block timestamp greater than 1000 and the specified sender
        let filtered_txns = filter.filter(block_id, 0, 1001, txns.clone());
        assert_eq!(filtered_txns, txns[1..].to_vec());
    }

    #[test]
    fn test_composite_allow_list_filter() {
        // Create a filter that only allows transactions based on multiple criteria
        let (block_id, txns) = get_block_id_and_transactions();
        let filter_string = format!(
            r#"
            rules:
                - Allow:
                    Sender: "{}"
                - Allow:
                    ModuleAddress: "0000000000000000000000000000000000000000000000000000000000000001"
                - Allow:
                    EntryFunction:
                        - "0000000000000000000000000000000000000000000000000000000000000002"
                        - test2
                        - new
                - Allow:
                    EntryFunction:
                        - "0000000000000000000000000000000000000000000000000000000000000003"
                        - test3
                        - sub
                - Deny: All
          "#,
            txns[0].sender().to_standard_string()
        );
        let filter = serde_yaml::from_str::<Filter>(&filter_string).unwrap();
        let allow_list_filter = TransactionFilter::new(filter);

        // Verify that only the first four transactions are allowed
        let filtered_txns = allow_list_filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, txns[0..4].to_vec());
    }

    #[test]
    fn test_composite_block_list_filter() {
        // Create a filter that denies transactions based on multiple criteria
        let (block_id, txns) = get_block_id_and_transactions();
        let filter_string = format!(
            r#"
            rules:
                - Deny:
                    ModuleAddress: "0000000000000000000000000000000000000000000000000000000000000000"
                - Deny:
                    Sender: "{}"
                - Deny:
                    EntryFunction:
                        - "0000000000000000000000000000000000000000000000000000000000000002"
                        - test2
                        - new
                - Deny:
                    ModuleAddress: "0000000000000000000000000000000000000000000000000000000000000003"
                - Allow: All
          "#,
            txns[1].sender().to_standard_string()
        );
        let filter = serde_yaml::from_str::<Filter>(&filter_string).unwrap();
        let block_list_filter = TransactionFilter::new(filter);

        // Verify that the first four transactions are denied
        let filtered_txns = block_list_filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, txns[4..].to_vec());
    }

    #[test]
    fn test_composite_matches_all_of_filter() {
        // Create a filter that denies transactions based on the matches all of rule
        let (block_id, txns) = get_block_id_and_transactions();
        let filter_string = format!(
            r#"
            rules:
                - Deny:
                    MatchesAllOf:
                        - Sender: "{}"
                        - ModuleAddress: "0000000000000000000000000000000000000000000000000000000000000000"
                        - BlockEpochGreaterThan: 10
                - Deny:
                    MatchesAllOf:
                        - Sender: "{}"
                        - ModuleAddress: "0000000000000000000000000000000000000000000000000000000000000001"
                        - BlockEpochGreaterThan: 10
                        - BlockTimeStampGreaterThan: 1000
                - Deny:
                    MatchesAllOf:
                        - Sender: "{}"
                        - ModuleAddress: "0000000000000000000000000000000000000000000000000000000000000002"
                        - BlockEpochGreaterThan: 10
                        - BlockTimeStampGreaterThan: 1000
                        - BlockId: "{}"
                - Allow: All
          "#,
            txns[0].sender().to_standard_string(),
            txns[1].sender().to_standard_string(),
            txns[2].sender().to_standard_string(),
            block_id.to_hex()
        );
        let filter = serde_yaml::from_str::<Filter>(&filter_string).unwrap();
        let block_list_filter = TransactionFilter::new(filter);

        // Filter transactions with a block epoch of 11, timestamp of 1001, and the expected block ID
        let filtered_txns = block_list_filter.filter(block_id, 11, 1001, txns.clone());

        // Verify that only the first three transactions are denied
        assert_eq!(filtered_txns, txns[3..].to_vec());

        // Filter transactions with a block epoch of 11, timestamp of 1001, and a random block ID
        let random_block_id = HashValue::random();
        let filtered_txns = block_list_filter.filter(random_block_id, 11, 1001, txns.clone());

        // Verify that only the first two transactions are denied
        assert_eq!(filtered_txns, txns[2..].to_vec());

        // Filter transactions with a block epoch of 11, timestamp of 999, and the expected block ID
        let filtered_txns = block_list_filter.filter(block_id, 11, 999, txns.clone());

        // Verify that only the first transaction is denied
        assert_eq!(filtered_txns, txns[1..].to_vec());
    }
}
