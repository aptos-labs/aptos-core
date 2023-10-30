// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_config::config::TransactionFilterType;
use aptos_crypto::HashValue;
use aptos_types::transaction::{SignedTransaction, TransactionPayload};
use move_core_types::account_address::AccountAddress;
use std::{collections::HashSet, sync::Arc};

pub trait TransactionFilter: Send + Sync {
    fn filter(&self, block_id: HashValue, txns: Vec<SignedTransaction>) -> Vec<SignedTransaction>;
}

pub fn create_transaction_filter(
    transaction_filter_type: TransactionFilterType,
) -> Arc<dyn TransactionFilter> {
    match transaction_filter_type {
        TransactionFilterType::NoFilter => Arc::new(NoTransactionFilter {}),
        TransactionFilterType::AllTransactions => Arc::new(AllTransactionsFilter {}),
        TransactionFilterType::BlockListedBlockIdBased(block_id) => {
            Arc::new(BlockListedBlockIdsFilter { block_id })
        },
        TransactionFilterType::BlockListedTransactionHashBased(transaction_hash) => {
            Arc::new(BlockListedTransactionHashesFilter { transaction_hash })
        },
        TransactionFilterType::BlockListedSenderBased(blocked_senders) => {
            Arc::new(BlockListedSendersFilter { blocked_senders })
        },
        TransactionFilterType::AllowListEntryFunctions(allowed_entry_functions) => {
            Arc::new(AllowListEntryFunctionFilter {
                allowed_entry_functions,
            })
        },
        TransactionFilterType::BlockListEntryFunctions(blocked_entry_functions) => {
            Arc::new(BlockListEntryFunctionFilter {
                blocked_entry_functions,
            })
        },
        TransactionFilterType::AllowListModuleAddresses(allowed_module_addresses) => {
            Arc::new(AllowListModuleAddressFilter {
                allowed_module_addresses,
            })
        },
        TransactionFilterType::BlockListModuleAddresses(blocked_module_addresses) => {
            Arc::new(BlockListModuleAddressFilter {
                blocked_module_addresses,
            })
        },
    }
}

struct NoTransactionFilter {}

impl TransactionFilter for NoTransactionFilter {
    fn filter(&self, _block_id: HashValue, txns: Vec<SignedTransaction>) -> Vec<SignedTransaction> {
        txns
    }
}

/// A filter that filters out all user transactions.
struct AllTransactionsFilter {}

impl TransactionFilter for AllTransactionsFilter {
    fn filter(
        &self,
        _block_id: HashValue,
        _txns: Vec<SignedTransaction>,
    ) -> Vec<SignedTransaction> {
        vec![]
    }
}

struct BlockListedBlockIdsFilter {
    block_id: HashSet<HashValue>,
}

impl TransactionFilter for BlockListedBlockIdsFilter {
    fn filter(&self, block_id: HashValue, txns: Vec<SignedTransaction>) -> Vec<SignedTransaction> {
        if self.block_id.contains(&block_id) {
            vec![]
        } else {
            txns
        }
    }
}

struct BlockListedTransactionHashesFilter {
    transaction_hash: HashSet<HashValue>,
}

impl TransactionFilter for BlockListedTransactionHashesFilter {
    fn filter(&self, _block_id: HashValue, txns: Vec<SignedTransaction>) -> Vec<SignedTransaction> {
        txns.into_iter()
            .filter(|txn| {
                !self
                    .transaction_hash
                    .contains(&txn.clone().committed_hash())
            })
            .collect()
    }
}

struct BlockListedSendersFilter {
    blocked_senders: HashSet<AccountAddress>,
}

impl TransactionFilter for BlockListedSendersFilter {
    fn filter(&self, _block_id: HashValue, txns: Vec<SignedTransaction>) -> Vec<SignedTransaction> {
        txns.into_iter()
            .filter(|txn| !self.blocked_senders.contains(&txn.sender()))
            .collect()
    }
}

struct AllowListEntryFunctionFilter {
    allowed_entry_functions: HashSet<(AccountAddress, String, String)>,
}

impl TransactionFilter for AllowListEntryFunctionFilter {
    fn filter(&self, _block_id: HashValue, txns: Vec<SignedTransaction>) -> Vec<SignedTransaction> {
        txns.into_iter()
            .filter(|txn| match txn.payload() {
                TransactionPayload::EntryFunction(entry_function) => {
                    self.allowed_entry_functions.contains(&(
                        *entry_function.module().address(),
                        entry_function.module().name().to_string(),
                        entry_function.function().to_string(),
                    ))
                },
                _ => false,
            })
            .collect()
    }
}

struct BlockListEntryFunctionFilter {
    blocked_entry_functions: HashSet<(AccountAddress, String, String)>,
}

impl TransactionFilter for BlockListEntryFunctionFilter {
    fn filter(&self, _block_id: HashValue, txns: Vec<SignedTransaction>) -> Vec<SignedTransaction> {
        txns.into_iter()
            .filter(|txn| match txn.payload() {
                TransactionPayload::EntryFunction(entry_function) => {
                    !self.blocked_entry_functions.contains(&(
                        *entry_function.module().address(),
                        entry_function.module().name().to_string(),
                        entry_function.function().to_string(),
                    ))
                },
                _ => true,
            })
            .collect()
    }
}

struct AllowListModuleAddressFilter {
    allowed_module_addresses: HashSet<AccountAddress>,
}

impl TransactionFilter for AllowListModuleAddressFilter {
    fn filter(&self, _block_id: HashValue, txns: Vec<SignedTransaction>) -> Vec<SignedTransaction> {
        txns.into_iter()
            .filter(|txn| match txn.payload() {
                TransactionPayload::EntryFunction(entry_function) => self
                    .allowed_module_addresses
                    .contains(entry_function.module().address()),
                _ => false,
            })
            .collect()
    }
}

struct BlockListModuleAddressFilter {
    blocked_module_addresses: HashSet<AccountAddress>,
}

impl TransactionFilter for BlockListModuleAddressFilter {
    fn filter(&self, _block_id: HashValue, txns: Vec<SignedTransaction>) -> Vec<SignedTransaction> {
        txns.into_iter()
            .filter(|txn| match txn.payload() {
                TransactionPayload::EntryFunction(entry_function) => !self
                    .blocked_module_addresses
                    .contains(entry_function.module().address()),
                _ => true,
            })
            .collect()
    }
}

#[cfg(test)]
mod test {
    use aptos_config::config::TransactionFilterType;
    use aptos_crypto::{ed25519::Ed25519PrivateKey, HashValue, PrivateKey, SigningKey, Uniform};
    use aptos_types::{
        chain_id::ChainId,
        move_utils::MemberId,
        transaction::{EntryFunction, RawTransaction, SignedTransaction, TransactionPayload},
    };
    use move_core_types::account_address::AccountAddress;
    use std::collections::HashSet;

    fn create_signed_transaction(function: MemberId) -> SignedTransaction {
        let private_key = Ed25519PrivateKey::generate_for_testing();
        let public_key = private_key.public_key();
        let sender = AccountAddress::random();
        let sequence_number = 0;
        let MemberId {
            module_id,
            member_id: function_id,
        } = function;

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
        match txn.payload() {
            TransactionPayload::EntryFunction(entry_func) => *entry_func.module().address(),
            _ => panic!("Unexpected transaction payload"),
        }
    }

    fn get_module_name(txn: &SignedTransaction) -> String {
        match txn.payload() {
            TransactionPayload::EntryFunction(entry_func) => entry_func.module().name().to_string(),
            _ => panic!("Unexpected transaction payload"),
        }
    }

    fn get_function_name(txn: &SignedTransaction) -> String {
        match txn.payload() {
            TransactionPayload::EntryFunction(entry_func) => entry_func.function().to_string(),
            _ => panic!("Unexpected transaction payload"),
        }
    }

    #[test]
    fn test_no_filter() {
        let txns = get_transactions();
        let block_id = HashValue::random();
        let no_filter = super::create_transaction_filter(TransactionFilterType::NoFilter);
        let filtered_txns = no_filter.filter(block_id, txns.clone());
        assert_eq!(filtered_txns, txns);
    }

    #[test]
    fn test_all_filter() {
        let txns = get_transactions();
        let block_id = HashValue::random();
        let all_filter = super::create_transaction_filter(TransactionFilterType::AllTransactions);
        let filtered_txns = all_filter.filter(block_id, txns.clone());
        assert_eq!(filtered_txns, vec![]);
    }

    #[test]
    fn test_block_id_filter() {
        let txns = get_transactions();
        let block_id = HashValue::random();
        let mut block_id_set = HashSet::new();
        block_id_set.insert(block_id);
        let block_id_filter = super::create_transaction_filter(
            TransactionFilterType::BlockListedBlockIdBased(block_id_set),
        );

        let filtered_txns = block_id_filter.filter(block_id, txns.clone());
        assert_eq!(filtered_txns, vec![]);

        let block_id = HashValue::random();
        let filtered_txns = block_id_filter.filter(block_id, txns.clone());
        assert_eq!(filtered_txns, txns);
    }

    #[test]
    fn test_transaction_hash_filter() {
        let txns = get_transactions();
        let block_id = HashValue::random();
        let mut transaction_hash_set = HashSet::new();
        transaction_hash_set.insert(txns[0].clone().committed_hash());
        let transaction_hash_filter = super::create_transaction_filter(
            TransactionFilterType::BlockListedTransactionHashBased(transaction_hash_set),
        );
        let filtered_txns = transaction_hash_filter.filter(block_id, txns.clone());
        assert_eq!(filtered_txns, txns[1..].to_vec());

        let mut transaction_hash_set = HashSet::new();
        transaction_hash_set.insert(HashValue::random());
        let transaction_hash_filter = super::create_transaction_filter(
            TransactionFilterType::BlockListedTransactionHashBased(transaction_hash_set),
        );
        let filtered_txns = transaction_hash_filter.filter(block_id, txns.clone());
        assert_eq!(filtered_txns, txns);
    }

    #[test]
    fn test_sender_filter() {
        let txns = get_transactions();
        let block_id = HashValue::random();
        let mut blocked_senders = HashSet::new();
        blocked_senders.insert(txns[0].sender());
        blocked_senders.insert(txns[1].sender());
        let block_list_sender_filter = super::create_transaction_filter(
            TransactionFilterType::BlockListedSenderBased(blocked_senders),
        );
        let filtered_txns = block_list_sender_filter.filter(block_id, txns.clone());
        assert_eq!(filtered_txns, txns[2..].to_vec());

        let blocked_senders = HashSet::new();
        let block_list_sender_filter = super::create_transaction_filter(
            TransactionFilterType::BlockListedSenderBased(blocked_senders),
        );
        let filtered_txns = block_list_sender_filter.filter(block_id, txns.clone());
        assert_eq!(filtered_txns, txns);
    }

    #[test]
    fn test_allow_list_entry_function_filter() {
        let txns = get_transactions();
        let block_id = HashValue::random();
        let mut allowed_entry_functions = HashSet::new();

        allowed_entry_functions.insert((
            get_module_address(&txns[0]),
            get_module_name(&txns[0]),
            get_function_name(&txns[0]),
        ));
        allowed_entry_functions.insert((
            get_module_address(&txns[1]),
            get_module_name(&txns[1]),
            get_function_name(&txns[1]),
        ));

        let allow_list_entry_function_filter = super::create_transaction_filter(
            TransactionFilterType::AllowListEntryFunctions(allowed_entry_functions),
        );
        let filtered_txns = allow_list_entry_function_filter.filter(block_id, txns.clone());
        assert_eq!(filtered_txns, txns[0..2].to_vec());

        let allowed_entry_functions = HashSet::new();
        let allow_list_entry_function_filter = super::create_transaction_filter(
            TransactionFilterType::AllowListEntryFunctions(allowed_entry_functions),
        );
        let filtered_txns = allow_list_entry_function_filter.filter(block_id, txns.clone());
        assert_eq!(filtered_txns, vec![]);
    }

    #[test]
    fn test_block_list_entry_function_filter() {
        let txns = get_transactions();
        let block_id = HashValue::random();
        let mut blocked_entry_functions = HashSet::new();
        blocked_entry_functions.insert((
            get_module_address(&txns[0]),
            get_module_name(&txns[0]),
            get_function_name(&txns[0]),
        ));

        blocked_entry_functions.insert((
            get_module_address(&txns[1]),
            get_module_name(&txns[1]),
            get_function_name(&txns[1]),
        ));

        let block_list_entry_function_filter = super::create_transaction_filter(
            TransactionFilterType::BlockListEntryFunctions(blocked_entry_functions),
        );
        let filtered_txns = block_list_entry_function_filter.filter(block_id, txns.clone());
        assert_eq!(filtered_txns, txns[2..].to_vec());

        let blocked_entry_functions = HashSet::new();
        let block_list_entry_function_filter = super::create_transaction_filter(
            TransactionFilterType::BlockListEntryFunctions(blocked_entry_functions),
        );
        let filtered_txns = block_list_entry_function_filter.filter(block_id, txns.clone());
        assert_eq!(filtered_txns, txns);
    }

    #[test]
    fn test_allow_list_module_address_filter() {
        let txns = get_transactions();
        let block_id = HashValue::random();
        let mut allowed_module_addresses = HashSet::new();
        allowed_module_addresses.insert(get_module_address(&txns[0])); // Only 0x1 is allowed
        let allow_list_module_address_filter = super::create_transaction_filter(
            TransactionFilterType::AllowListModuleAddresses(allowed_module_addresses),
        );
        let filtered_txns = allow_list_module_address_filter.filter(block_id, txns.clone());
        assert_eq!(filtered_txns, txns[0..4].to_vec());

        let allowed_module_addresses = HashSet::new();
        let allow_list_module_address_filter = super::create_transaction_filter(
            TransactionFilterType::AllowListModuleAddresses(allowed_module_addresses),
        );
        let filtered_txns = allow_list_module_address_filter.filter(block_id, txns.clone());
        assert_eq!(filtered_txns, vec![]);
    }

    #[test]
    fn test_block_list_module_address_filter() {
        let txns = get_transactions();
        let block_id = HashValue::random();
        let mut blocked_module_addresses = HashSet::new();
        blocked_module_addresses.insert(get_module_address(&txns[0])); // 0x1 is blocked

        let block_list_module_address_filter = super::create_transaction_filter(
            TransactionFilterType::BlockListModuleAddresses(blocked_module_addresses),
        );
        let filtered_txns = block_list_module_address_filter.filter(block_id, txns.clone());
        assert_eq!(filtered_txns, txns[4..].to_vec());

        let blocked_module_addresses = HashSet::new();
        let block_list_module_address_filter = super::create_transaction_filter(
            TransactionFilterType::BlockListModuleAddresses(blocked_module_addresses),
        );
        let filtered_txns = block_list_module_address_filter.filter(block_id, txns.clone());
        assert_eq!(filtered_txns, txns);
    }
}
