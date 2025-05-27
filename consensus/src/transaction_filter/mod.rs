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
    fn test_no_filter() {
        let txns = get_transactions();
        let block_id = HashValue::random();
        let no_filter = TransactionFilter::new(Filter::empty());
        let filtered_txns = no_filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, txns);
    }

    #[test]
    fn test_all_filter() {
        let txns = get_transactions();
        let block_id = HashValue::random();
        let all_filter = TransactionFilter::new(Filter::empty().add_deny_all());
        let filtered_txns = all_filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, vec![]);
    }

    #[test]
    fn test_block_id_filter() {
        let txns = get_transactions();
        let block_id = HashValue::random();
        let block_id_filter = TransactionFilter::new(Filter::empty().add_deny_block_id(block_id));

        let filtered_txns = block_id_filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, vec![]);
        let block_id = HashValue::random();
        let filtered_txns = block_id_filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, txns);
    }

    #[test]
    fn test_block_timestamp_filter() {
        let txns = get_transactions();
        let block_id = HashValue::random();
        // Allows all transactions with block timestamp greater than 1000
        let block_timestamp_filter = TransactionFilter::new(
            Filter::empty()
                .add_allow_block_timestamp_greater_than(1000)
                .add_deny_all(),
        );

        let filtered_txns = block_timestamp_filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, vec![]);
        let filtered_txns = block_timestamp_filter.filter(block_id, 0, 1001, txns.clone());
        assert_eq!(filtered_txns, txns);
    }

    #[test]
    fn test_transaction_hash_filter() {
        let txns = get_transactions();
        let block_id = HashValue::random();
        let transaction_hash_filter = TransactionFilter::new(
            Filter::empty().add_deny_transaction_id(txns[0].committed_hash()),
        );
        let filtered_txns = transaction_hash_filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, txns[1..].to_vec());
    }

    #[test]
    fn test_sender_filter() {
        let txns = get_transactions();
        let block_id = HashValue::random();
        let block_list_sender_filter = TransactionFilter::new(
            Filter::empty()
                .add_deny_sender(txns[0].sender())
                .add_deny_sender(txns[1].sender()),
        );
        let filtered_txns = block_list_sender_filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, txns[2..].to_vec());
    }

    #[test]
    fn test_entry_function_filter() {
        let txns = get_transactions();
        let block_id = HashValue::random();
        let allow_list_entry_function_filter = TransactionFilter::new(
            Filter::empty()
                .add_allow_entry_function(
                    get_module_address(&txns[0]),
                    get_module_name(&txns[0]),
                    get_function_name(&txns[0]),
                )
                .add_allow_entry_function(
                    get_module_address(&txns[1]),
                    get_module_name(&txns[1]),
                    get_function_name(&txns[1]),
                )
                .add_deny_all(),
        );
        let filtered_txns = allow_list_entry_function_filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, txns[0..2].to_vec());

        let deny_list_entry_function_filter = TransactionFilter::new(
            Filter::empty()
                .add_deny_entry_function(
                    get_module_address(&txns[0]),
                    get_module_name(&txns[0]),
                    get_function_name(&txns[0]),
                )
                .add_deny_entry_function(
                    get_module_address(&txns[1]),
                    get_module_name(&txns[1]),
                    get_function_name(&txns[1]),
                ),
        );
        let filtered_txns = deny_list_entry_function_filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, txns[2..].to_vec());
    }

    #[test]
    fn test_allow_list_module_address_filter() {
        let txns = get_transactions();
        let block_id = HashValue::random();
        let allow_list_module_address_filter = TransactionFilter::new(
            Filter::empty()
                .add_allow_module_address(get_module_address(&txns[0]))
                .add_allow_module_address(get_module_address(&txns[1]))
                .add_deny_all(),
        );
        let filtered_txns = allow_list_module_address_filter.filter(block_id, 0, 0, txns.clone());
        assert_eq!(filtered_txns, txns[0..4].to_vec());

        let block_list_module_address_filter = TransactionFilter::new(
            Filter::empty()
                .add_deny_module_address(get_module_address(&txns[0]))
                .add_deny_module_address(get_module_address(&txns[1])),
        );
        let filtered_txns = block_list_module_address_filter.filter(block_id, 0, 0, txns.clone());
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
                - Deny: All
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
