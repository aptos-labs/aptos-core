// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::fmt::{Debug, format, Formatter, Write};
use crate::{
    access_path::AccessPath,
    account_config::{AccountResource, CoinStoreResource},
    state_store::{state_key::StateKey, table::TableHandle},
    transaction::{SignedTransaction, Transaction, TransactionPayload},
};
use aptos_crypto::{
    hash::{CryptoHash, DummyHasher},
    HashValue,
};
pub use move_core_types::abi::{
    ArgumentABI, ScriptFunctionABI as EntryFunctionABI, TransactionScriptABI, TypeArgumentABI,
};
use move_core_types::{
    account_address::AccountAddress, language_storage::StructTag, move_resource::MoveStructType,
};
use std::hash::{Hash, Hasher};

#[derive(Clone)]
pub struct AnalyzedTransaction {
    transaction: Transaction,
    /// Set of storage locations that are read by the transaction - this doesn't include location
    /// that are written by the transactions to avoid duplication of locations across read and write sets
    /// This can be accurate or strictly overestimated.
    read_set: Vec<StorageLocation>,
    /// Set of storage locations that are written by the transaction. This can be accurate or strictly
    /// overestimated.
    write_set: Vec<StorageLocation>,
    /// A transaction is predictable if neither the read_hint or the write_hint have wildcards.
    predictable_transaction: bool,
    /// The hash of the transaction - this is cached for performance reasons.
    hash: HashValue,

    pub recipient: Option<AccountAddress>,
}

impl Debug for AnalyzedTransaction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let sender = self.sender().unwrap().brief();
        let receiver = self.recipient.unwrap().brief();
        f.write_str(format!("{sender}->{receiver}").as_str())
    }
}
#[derive(Clone, Hash, Eq, PartialEq)]
pub struct StorageLocation {
    inner: StorageLocationInner,
    owner: Option<AccountAddress>,
}

impl Debug for StorageLocation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let owner = match self.owner {
            Some(addr) => addr.brief(),
            None => "????".to_string(),
        };
        f.write_str(format!("{owner}...").as_str())
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum StorageLocationInner {
    // A specific storage location denoted by an address and a struct tag.
    Specific(StateKey),
    // Storage location denoted by a struct tag and any arbitrary address.
    // Example read<T>(*), write<T>(*) in Move
    WildCardStruct(StructTag),
    // Storage location denoted by a table handle and any arbitrary item in the table.
    WildCardTable(TableHandle),
}

impl CryptoHash for StorageLocation {
    type Hasher = DummyHasher;

    fn hash(&self) -> HashValue {
        match &self.inner {
            StorageLocationInner::Specific(state_key) => CryptoHash::hash(state_key),
            _ => todo!("hashing of wildcard storage location is not supported yet"),
        }
    }
}

impl StorageLocation {
    pub fn specific(x: StateKey, y: Option<AccountAddress>) -> Self {
        Self {
            inner: StorageLocationInner::Specific(x),
            owner: y,
        }
    }
}


impl AnalyzedTransaction {
    pub fn new(
        transaction: Transaction,
        read_set: Vec<StorageLocation>,
        write_set: Vec<StorageLocation>,
        recipient: Option<AccountAddress>,
    ) -> Self {
        let hints_contain_wildcard = read_set
            .iter()
            .chain(write_set.iter())
            .any(|hint| !matches!(hint.inner, StorageLocationInner::Specific(_)));
        let hash = transaction.hash();
        AnalyzedTransaction {
            transaction,
            read_set,
            write_set,
            predictable_transaction: !hints_contain_wildcard,
            hash,
            recipient,
        }
    }

    pub fn into_inner(self) -> Transaction {
        self.transaction
    }

    pub fn transaction(&self) -> &Transaction {
        &self.transaction
    }

    pub fn read_set(&self) -> &[StorageLocation] {
        &self.read_set
    }

    pub fn write_set(&self) -> &[StorageLocation] {
        &self.write_set
    }

    pub fn predictable_transaction(&self) -> bool {
        self.predictable_transaction
    }

    pub fn sender(&self) -> Option<AccountAddress> {
        match &self.transaction {
            Transaction::UserTransaction(signed_txn) => Some(signed_txn.sender()),
            _ => None,
        }
    }

    pub fn analyzed_transaction_for_coin_transfer(
        signed_txn: SignedTransaction,
        sender_address: AccountAddress,
        receiver_address: AccountAddress,
        receiver_exists: bool,
    ) -> Self {
        let sender_account_resource_key = StateKey::access_path(AccessPath::new(
            sender_address,
            AccountResource::struct_tag().access_vector(),
        ));

        let sender_coin_store_key = StateKey::access_path(AccessPath::new(
            sender_address,
            CoinStoreResource::struct_tag().access_vector(),
        ));
        let receiver_account_resource_key = StateKey::access_path(AccessPath::new(
            receiver_address,
            AccountResource::struct_tag().access_vector(),
        ));
        let receiver_coin_store_key = StateKey::access_path(AccessPath::new(
            receiver_address,
            CoinStoreResource::struct_tag().access_vector(),
        ));
        let mut write_set = vec![
            StorageLocation::specific(sender_coin_store_key, Some(sender_address)),
            StorageLocation::specific(receiver_coin_store_key, Some(receiver_address)),
            StorageLocation::specific(sender_account_resource_key, Some(sender_address)),
        ];
        if !receiver_exists {
            // If the receiver doesn't exist, we create the receiver account, so we need to read the
            // receiver account resource.
            write_set.push(StorageLocation::specific(receiver_account_resource_key, Some(receiver_address)));
        }
        AnalyzedTransaction::new(
            Transaction::UserTransaction(signed_txn),
            vec![],
            // read and write locations are same for coin transfer
            write_set,
            Some(receiver_address),
        )
    }

    pub fn analyzed_transaction_for_create_account(
        signed_txn: SignedTransaction,
        sender_address: AccountAddress,
        receiver_address: AccountAddress,
    ) -> Self {
        let sender_account_resource_key = StateKey::access_path(AccessPath::new(
            sender_address,
            AccountResource::struct_tag().access_vector(),
        ));
        let sender_coin_store_key = StateKey::access_path(AccessPath::new(
            sender_address,
            CoinStoreResource::struct_tag().access_vector(),
        ));
        let receiver_account_resource_key = StateKey::access_path(AccessPath::new(
            receiver_address,
            AccountResource::struct_tag().access_vector(),
        ));
        let receiver_coin_store_key = StateKey::access_path(AccessPath::new(
            receiver_address,
            CoinStoreResource::struct_tag().access_vector(),
        ));
        let read_hints = vec![
            StorageLocation::specific(sender_coin_store_key, Some(sender_address)),
            StorageLocation::specific(sender_account_resource_key, Some(sender_address)),
            StorageLocation::specific(receiver_coin_store_key, Some(receiver_address)),
            StorageLocation::specific(receiver_account_resource_key, Some(receiver_address)),
        ];
        AnalyzedTransaction::new(
            Transaction::UserTransaction(signed_txn),
            vec![],
            // read and write locations are same for create account
            read_hints,
            Some(receiver_address),
        )
    }
}

impl PartialEq<Self> for AnalyzedTransaction {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

impl Eq for AnalyzedTransaction {}

impl Hash for AnalyzedTransaction {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(self.hash.as_ref());
    }
}

impl From<Transaction> for AnalyzedTransaction {
    fn from(txn: Transaction) -> Self {
        match txn {
            Transaction::UserTransaction(signed_txn) => match signed_txn.payload() {
                TransactionPayload::EntryFunction(func) => {
                    match (
                        *func.module().address(),
                        func.module().name().as_str(),
                        func.function().as_str(),
                    ) {
                        (AccountAddress::ONE, "coin", "transfer") => {
                            let sender_address = signed_txn.sender();
                            let receiver_address = bcs::from_bytes(&func.args()[0]).unwrap();
                            AnalyzedTransaction::analyzed_transaction_for_coin_transfer(
                                signed_txn,
                                sender_address,
                                receiver_address,
                                true,
                            )
                        },
                        (AccountAddress::ONE, "aptos_account", "transfer") => {
                            let sender_address = signed_txn.sender();
                            let receiver_address = bcs::from_bytes(&func.args()[0]).unwrap();
                            AnalyzedTransaction::analyzed_transaction_for_coin_transfer(
                                signed_txn,
                                sender_address,
                                receiver_address,
                                false,
                            )
                        },
                        (AccountAddress::ONE, "aptos_account", "create_account") => {
                            let sender_address = signed_txn.sender();
                            let receiver_address = bcs::from_bytes(&func.args()[0]).unwrap();
                            AnalyzedTransaction::analyzed_transaction_for_create_account(
                                signed_txn,
                                sender_address,
                                receiver_address,
                            )
                        },
                        _ => AnalyzedTransaction::new(
                            Transaction::UserTransaction(signed_txn),
                            vec![],
                            vec![],
                            None,
                        ),
                    }
                },
                _ => AnalyzedTransaction::new(
                    Transaction::UserTransaction(signed_txn),
                    vec![],
                    vec![],
                    None,
                ),
            },
            _ => AnalyzedTransaction::new(txn, vec![], vec![], None),
        }
    }
}

impl From<AnalyzedTransaction> for Transaction {
    fn from(val: AnalyzedTransaction) -> Self {
        val.transaction
    }
}
