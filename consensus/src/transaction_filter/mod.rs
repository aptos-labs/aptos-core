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
    use aptos_config::config::transaction_filter_type::Filter;
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

    fn get_block_id_and_transactions() -> (HashValue, Vec<SignedTransaction>) {
        let block_id = HashValue::random();
        let transactions = get_transactions();
        (block_id, transactions)
    }

    fn get_transactions() -> Vec<SignedTransaction> {
        vec![
            create_signed_transaction(str::parse("0x1::test::add").unwrap()),
            create_signed_transaction(str::parse("0x1::test::check").unwrap()),
            create_signed_transaction(str::parse("0x1::test::new").unwrap()),
            create_signed_transaction(str::parse("0x1::test::sub").unwrap()),
            create_signed_transaction(str::parse("0x2::test2::mul").unwrap()),
            create_signed_transaction(str::parse("0x3::test2::div").unwrap()),
            create_signed_transaction(str::parse("0x4::test2::mod").unwrap()),
        ]
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
        let empty_filter = TransactionFilter::new(Filter::empty());

        // Verify that the empty filter allows all transactions
        let (block_id, txns) = get_block_id_and_transactions();
        let filtered_txns = empty_filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, txns);
    }

    #[test]
    fn test_any_filter() {
        // Create a filter that denies all transactions
        let any_filter = TransactionFilter::new(Filter::empty().add_any_filter(false));

        // Verify that the filter denies all transactions
        let (block_id, txns) = get_block_id_and_transactions();
        let filtered_txns = any_filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, vec![]);

        // Create a filter that allows all transactions
        let any_filter = TransactionFilter::new(Filter::empty().add_any_filter(true));

        // Verify that the filter allows all transactions
        let filtered_txns = any_filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, txns);
    }

    #[test]
    fn test_block_id_filter() {
        // Create a filter that denies transactions with a specific block ID
        let (block_id, txns) = get_block_id_and_transactions();
        let block_id_filter =
            TransactionFilter::new(Filter::empty().add_block_id_filter(false, block_id));

        // Verify that the filter denies transactions with the specified block ID
        let filtered_txns = block_id_filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, vec![]);

        // Create a filter that only allows transactions with a specific block ID
        let block_id_filter =
            TransactionFilter::new(Filter::empty().add_block_id_filter(true, block_id));

        // Verify that the filter allows transactions with the specified block ID
        let filtered_txns = block_id_filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, txns);

        // Verify that the filter denies transactions with a different block ID
        let block_id = HashValue::random();
        let filtered_txns = block_id_filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, txns);
    }

    #[test]
    fn test_deny_block_timestamp_filter_greater_than() {
        // Create a filter that only allows transactions with block timestamp greater than 1000
        let block_timestamp_filter = TransactionFilter::new(
            Filter::empty()
                .add_block_timestamp_greater_than_filter(true, 1000)
                .add_any_filter(false),
        );

        // Verify that the filter denies transactions with block timestamp less than or equal to 1000
        let (block_id, txns) = get_block_id_and_transactions();
        let filtered_txns = block_timestamp_filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, vec![]);
        let filtered_txns = block_timestamp_filter.filter(block_id, 0, 1000, txns.clone());
        assert_eq!(filtered_txns, vec![]);

        // Verify that the filter allows transactions with block timestamp greater than 1000
        let filtered_txns = block_timestamp_filter.filter(block_id, 0, 1001, txns.clone());
        assert_eq!(filtered_txns, txns);
    }

    #[test]
    fn test_block_timestamp_filter_less_than() {
        // Create a filter that only allows transactions with block timestamp less than 1000
        let block_timestamp_filter = TransactionFilter::new(
            Filter::empty()
                .add_block_timestamp_less_than_filter(true, 1000)
                .add_any_filter(false),
        );

        // Verify that the filter allows transactions with block timestamp less than 1000
        let (block_id, txns) = get_block_id_and_transactions();
        let filtered_txns = block_timestamp_filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, txns);
        let filtered_txns = block_timestamp_filter.filter(block_id, 0, 999, txns.clone());
        assert_eq!(filtered_txns, txns);

        // Verify that the filter denies transactions with block timestamp greater than or equal to 1000
        let filtered_txns = block_timestamp_filter.filter(block_id, 0, 1000, txns.clone());
        assert_eq!(filtered_txns, vec![]);
    }

    #[test]
    fn test_epoch_filter_greater_than() {
        // Create a filter that only allows transactions with block epoch greater than 1000
        let block_epoch_filter = TransactionFilter::new(
            Filter::empty()
                .add_epoch_greater_than_filter(true, 1000)
                .add_any_filter(false),
        );

        // Verify that the filter denies transactions with block epoch less than or equal to 1000
        let (block_id, txns) = get_block_id_and_transactions();
        let filtered_txns = block_epoch_filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, vec![]);
        let filtered_txns = block_epoch_filter.filter(block_id, 1000, 0, txns.clone());
        assert_eq!(filtered_txns, vec![]);

        // Verify that the filter allows transactions with block epoch greater than 1000
        let filtered_txns = block_epoch_filter.filter(block_id, 1001, 0, txns.clone());
        assert_eq!(filtered_txns, txns);
    }

    #[test]
    fn test_epoch_filter_less_than() {
        // Create a filter that only allows transactions with block epoch less than 1000
        let block_epoch_filter = TransactionFilter::new(
            Filter::empty()
                .add_epoch_less_than_filter(true, 1000)
                .add_any_filter(false),
        );

        // Verify that the filter allows transactions with block epoch less than 1000
        let (block_id, txns) = get_block_id_and_transactions();
        let filtered_txns = block_epoch_filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, txns);
        let filtered_txns = block_epoch_filter.filter(block_id, 999, 0, txns.clone());
        assert_eq!(filtered_txns, txns);

        // Verify that the filter denies transactions with block epoch greater than or equal to 1000
        let filtered_txns = block_epoch_filter.filter(block_id, 1000, 0, txns.clone());
        assert_eq!(filtered_txns, vec![]);
    }

    #[test]
    fn test_transaction_hash_filter() {
        // Create a filter that denies transactions with a specific transaction hash (txn 0)
        let (block_id, txns) = get_block_id_and_transactions();
        let transaction_hash_filter = TransactionFilter::new(
            Filter::empty().add_transaction_id_filter(false, txns[0].committed_hash()),
        );

        // Verify that the filter denies the transaction with the specified hash
        let filtered_txns = transaction_hash_filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, txns[1..].to_vec());
    }

    #[test]
    fn test_sender_filter() {
        // Create a filter that denies transactions from specific senders (txn 0 and txn 1)
        let (block_id, txns) = get_block_id_and_transactions();
        let sender_filter = TransactionFilter::new(
            Filter::empty()
                .add_sender_filter(false, txns[0].sender())
                .add_sender_filter(false, txns[1].sender())
                .add_any_filter(true),
        );

        // Verify that the filter denies transactions from the specified senders
        let filtered_txns = sender_filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, txns[2..].to_vec());
    }

    #[test]
    fn test_entry_function_filter() {
        // Create a filter that allows specific entry functions (txn 0 and txn 1)
        let (block_id, txns) = get_block_id_and_transactions();
        let entry_function_filter = TransactionFilter::new(
            Filter::empty()
                .add_entry_function_filter(
                    true,
                    get_module_address(&txns[0]),
                    get_module_name(&txns[0]),
                    get_function_name(&txns[0]),
                )
                .add_entry_function_filter(
                    true,
                    get_module_address(&txns[1]),
                    get_module_name(&txns[1]),
                    get_function_name(&txns[1]),
                )
                .add_any_filter(false),
        );

        // Verify that the filter allows transactions with the specified entry functions
        let filtered_txns = entry_function_filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, txns[0..2].to_vec());

        // Create a filter that denies specific entry functions (txn 0 and txn 1)
        let deny_list_entry_function_filter = TransactionFilter::new(
            Filter::empty()
                .add_entry_function_filter(
                    false,
                    get_module_address(&txns[0]),
                    get_module_name(&txns[0]),
                    get_function_name(&txns[0]),
                )
                .add_entry_function_filter(
                    false,
                    get_module_address(&txns[1]),
                    get_module_name(&txns[1]),
                    get_function_name(&txns[1]),
                ),
        );

        // Verify that the filter denies transactions with the specified entry functions
        let filtered_txns = deny_list_entry_function_filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, txns[2..].to_vec());
    }

    #[test]
    fn test_allow_list_module_address_filter() {
        // Create a filter that allows transactions from specific module addresses (txn 0 and txn 1)
        let (block_id, txns) = get_block_id_and_transactions();
        let module_address_filter = TransactionFilter::new(
            Filter::empty()
                .add_module_address_filter(true, get_module_address(&txns[0]))
                .add_module_address_filter(true, get_module_address(&txns[1]))
                .add_any_filter(false),
        );

        // Verify that the filter allows transactions from the specified module addresses
        let filtered_txns = module_address_filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, txns[0..2].to_vec());

        // Create a filter that denies transactions from specific module addresses (txn 0 to txn 3)
        let allow_list_module_address_filter = TransactionFilter::new(
            Filter::empty()
                .add_module_address_filter(false, get_module_address(&txns[0]))
                .add_module_address_filter(false, get_module_address(&txns[1]))
                .add_module_address_filter(false, get_module_address(&txns[2]))
                .add_module_address_filter(false, get_module_address(&txns[3]))
                .add_any_filter(true),
        );

        // Verify that the filter denies transactions from the specified module addresses
        let filtered_txns = allow_list_module_address_filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, txns[4..].to_vec());
    }

    #[test]
    fn test_composite_allow_list_filter() {
        let txns = get_transactions();
        let block_id = HashValue::random();
        let filter = serde_yaml::from_str::<Filter>(r#"
            rules:
                - Allow:
                    Sender: f8871acf2c827d40e23b71f6ff2b9accef8dbb17709b88bd9eb95e6bb748c25a
                - Allow:
                    ModuleAddress: "0000000000000000000000000000000000000000000000000000000000000001"
                - Allow:
                    EntryFunction:
                        - "0000000000000000000000000000000000000000000000000000000000000001"
                        - test
                        - check
                - Allow:
                    EntryFunction:
                        - "0000000000000000000000000000000000000000000000000000000000000001"
                        - test
                        - new
                - Deny: Any
              "#).unwrap();

        let allow_list_filter = TransactionFilter::new(filter);
        let filtered_txns = allow_list_filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, txns[0..4].to_vec());
    }

    #[test]
    fn test_composite_block_list_filter() {
        let txns = get_transactions();
        let block_id = HashValue::random();
        let filter = serde_yaml::from_str::<Filter>(r#"
            rules:
                - Deny:
                    Sender: f8871acf2c827d40e23b71f6ff2b9accef8dbb17709b88bd9eb95e6bb748c25a
                - Deny:
                    ModuleAddress: "0000000000000000000000000000000000000000000000000000000000000001"
                - Deny:
                    EntryFunction:
                        - "0000000000000000000000000000000000000000000000000000000000000001"
                        - test
                        - check
                - Deny:
                    EntryFunction:
                        - "0000000000000000000000000000000000000000000000000000000000000001"
                        - test
                        - new
              "#).unwrap();

        let allow_list_filter = TransactionFilter::new(filter);
        let filtered_txns = allow_list_filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, txns[4..].to_vec());
    }
}
