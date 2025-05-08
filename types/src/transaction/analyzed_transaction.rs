// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account_config::{AccountResource, CoinInfoResource, CoinStoreResource},
    chain_id::ChainId,
    on_chain_config::{CurrentTimeMicroseconds, Features, TransactionFeeBurnCap},
    state_store::{state_key::StateKey, table::TableHandle},
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction, EntryFunction, Transaction,
        TransactionExecutableRef,
    },
    AptosCoinType, CoinType,
};
use aptos_crypto::HashValue;
pub use move_core_types::abi::{
    ArgumentABI, ScriptFunctionABI as EntryFunctionABI, TransactionScriptABI, TypeArgumentABI,
};
use move_core_types::{account_address::AccountAddress, language_storage::StructTag};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AnalyzedTransaction {
    transaction: SignatureVerifiedTransaction,
    /// Set of storage locations that are read by the transaction - this doesn't include location
    /// that are written by the transactions to avoid duplication of locations across read and write sets
    /// This can be accurate or strictly overestimated.
    pub read_hints: Vec<StorageLocation>,
    /// Set of storage locations that are written by the transaction. This can be accurate or strictly
    /// overestimated.
    pub write_hints: Vec<StorageLocation>,
    /// A transaction is predictable if neither the read_hint or the write_hint have wildcards.
    predictable_transaction: bool,
    /// The hash of the transaction - this is cached for performance reasons.
    submitted_txn_hash: HashValue,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
// TODO(skedia): Evaluate if we need to cache the HashValue for efficiency reasons.
pub enum StorageLocation {
    // A specific storage location denoted by an address and a struct tag.
    Specific(StateKey),
    // Storage location denoted by a struct tag and any arbitrary address.
    // Example read<T>(*), write<T>(*) in Move
    WildCardStruct(StructTag),
    // Storage location denoted by a table handle and any arbitrary item in the table.
    WildCardTable(TableHandle),
}

impl StorageLocation {
    pub fn into_state_key(self) -> StateKey {
        match self {
            StorageLocation::Specific(state_key) => state_key,
            _ => panic!("Cannot convert wildcard storage location to state key"),
        }
    }

    pub fn state_key(&self) -> &StateKey {
        match self {
            StorageLocation::Specific(state_key) => state_key,
            _ => panic!("Cannot convert wildcard storage location to state key"),
        }
    }
}

impl AnalyzedTransaction {
    pub fn new(transaction: SignatureVerifiedTransaction) -> Self {
        let (read_hints, write_hints) = transaction.get_read_write_hints();
        let hints_contain_wildcard = read_hints
            .iter()
            .chain(write_hints.iter())
            .any(|hint| !matches!(hint, StorageLocation::Specific(_)));
        let submitted_txn_hash = transaction.submitted_txn_hash();
        AnalyzedTransaction {
            transaction,
            read_hints,
            write_hints,
            predictable_transaction: !hints_contain_wildcard,
            submitted_txn_hash,
        }
    }

    pub fn into_txn(self) -> SignatureVerifiedTransaction {
        self.transaction
    }

    pub fn transaction(&self) -> &SignatureVerifiedTransaction {
        &self.transaction
    }

    pub fn read_hints(&self) -> &[StorageLocation] {
        &self.read_hints
    }

    pub fn write_hints(&self) -> &[StorageLocation] {
        &self.write_hints
    }

    pub fn predictable_transaction(&self) -> bool {
        self.predictable_transaction
    }

    pub fn sender(&self) -> Option<AccountAddress> {
        self.transaction.sender()
    }

    pub fn expect_p_txn(self) -> (SignatureVerifiedTransaction, Vec<StateKey>, Vec<StateKey>) {
        assert!(self.predictable_transaction());
        (
            self.transaction,
            Self::expect_specific_locations(self.read_hints),
            Self::expect_specific_locations(self.write_hints),
        )
    }

    fn expect_specific_locations(locations: Vec<StorageLocation>) -> Vec<StateKey> {
        locations
            .into_iter()
            .map(|loc| match loc {
                StorageLocation::Specific(key) => key,
                _ => unreachable!(),
            })
            .collect()
    }
}

impl PartialEq<Self> for AnalyzedTransaction {
    fn eq(&self, other: &Self) -> bool {
        self.submitted_txn_hash == other.submitted_txn_hash
    }
}

impl Eq for AnalyzedTransaction {}

impl Hash for AnalyzedTransaction {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(self.submitted_txn_hash.as_ref());
    }
}

impl From<SignatureVerifiedTransaction> for AnalyzedTransaction {
    fn from(txn: SignatureVerifiedTransaction) -> Self {
        AnalyzedTransaction::new(txn)
    }
}

impl From<AnalyzedTransaction> for SignatureVerifiedTransaction {
    fn from(val: AnalyzedTransaction) -> Self {
        val.transaction
    }
}

impl From<Transaction> for AnalyzedTransaction {
    fn from(txn: Transaction) -> Self {
        AnalyzedTransaction::new(txn.into())
    }
}

pub fn account_resource_location(address: AccountAddress) -> StorageLocation {
    StorageLocation::Specific(StateKey::resource_typed::<AccountResource>(&address).unwrap())
}

pub fn coin_store_location(address: AccountAddress) -> StorageLocation {
    StorageLocation::Specific(
        StateKey::resource_typed::<CoinStoreResource<AptosCoinType>>(&address).unwrap(),
    )
}

pub fn current_ts_location() -> StorageLocation {
    StorageLocation::Specific(StateKey::on_chain_config::<CurrentTimeMicroseconds>().unwrap())
}

pub fn features_location() -> StorageLocation {
    StorageLocation::Specific(StateKey::on_chain_config::<Features>().unwrap())
}

pub fn aptos_coin_info_location() -> StorageLocation {
    StorageLocation::Specific(
        StateKey::resource_typed::<CoinInfoResource<AptosCoinType>>(
            &AptosCoinType::coin_info_address(),
        )
        .unwrap(),
    )
}

pub fn chain_id_location() -> StorageLocation {
    StorageLocation::Specific(StateKey::on_chain_config::<ChainId>().unwrap())
}

pub fn transaction_fee_burn_cap_location() -> StorageLocation {
    StorageLocation::Specific(StateKey::on_chain_config::<TransactionFeeBurnCap>().unwrap())
}

pub fn rw_set_for_coin_transfer(
    sender_address: AccountAddress,
    receiver_address: AccountAddress,
    receiver_exists: bool,
) -> (Vec<StorageLocation>, Vec<StorageLocation>) {
    let mut write_hints = vec![
        account_resource_location(sender_address),
        coin_store_location(sender_address),
    ];
    if sender_address != receiver_address {
        write_hints.push(coin_store_location(receiver_address));
    }
    if !receiver_exists {
        // If the receiver doesn't exist, we create the receiver account, so we need to write the
        // receiver account resource.
        write_hints.push(account_resource_location(receiver_address));
    }

    let read_hints = vec![
        current_ts_location(),
        features_location(),
        aptos_coin_info_location(),
        chain_id_location(),
        transaction_fee_burn_cap_location(),
    ];
    (read_hints, write_hints)
}

pub fn rw_set_for_create_account(
    sender_address: AccountAddress,
    receiver_address: AccountAddress,
) -> (Vec<StorageLocation>, Vec<StorageLocation>) {
    let read_hints = vec![
        account_resource_location(sender_address),
        coin_store_location(sender_address),
        account_resource_location(receiver_address),
        coin_store_location(receiver_address),
    ];
    (vec![], read_hints)
}

pub fn empty_rw_set() -> (Vec<StorageLocation>, Vec<StorageLocation>) {
    (vec![], vec![])
}

trait AnalyzedTransactionProvider {
    fn get_read_write_hints(&self) -> (Vec<StorageLocation>, Vec<StorageLocation>);
}

impl AnalyzedTransactionProvider for Transaction {
    fn get_read_write_hints(&self) -> (Vec<StorageLocation>, Vec<StorageLocation>) {
        let process_entry_function = |func: &EntryFunction,
                                      sender_address: AccountAddress|
         -> (Vec<StorageLocation>, Vec<StorageLocation>) {
            match (
                *func.module().address(),
                func.module().name().as_str(),
                func.function().as_str(),
            ) {
                (AccountAddress::ONE, "coin", "transfer") => {
                    let receiver_address = bcs::from_bytes(&func.args()[0]).unwrap();
                    rw_set_for_coin_transfer(sender_address, receiver_address, true)
                },
                (AccountAddress::ONE, "aptos_account", "transfer") => {
                    let receiver_address = bcs::from_bytes(&func.args()[0]).unwrap();
                    rw_set_for_coin_transfer(sender_address, receiver_address, false)
                },
                (AccountAddress::ONE, "aptos_account", "create_account") => {
                    let receiver_address = bcs::from_bytes(&func.args()[0]).unwrap();
                    rw_set_for_create_account(sender_address, receiver_address)
                },
                _ => todo!(
                    "Only coin transfer and create account transactions are supported for now"
                ),
            }
        };
        if let Some(signed_txn) = self.try_as_signed_user_txn() {
            match signed_txn.payload().executable_ref() {
                Ok(TransactionExecutableRef::EntryFunction(func))
                    if !signed_txn.payload().is_multisig() =>
                {
                    process_entry_function(func, signed_txn.sender())
                },
                _ => todo!("Only entry function transactions are supported for now"),
            }
        } else {
            empty_rw_set()
        }
    }
}

impl AnalyzedTransactionProvider for SignatureVerifiedTransaction {
    fn get_read_write_hints(&self) -> (Vec<StorageLocation>, Vec<StorageLocation>) {
        match self {
            SignatureVerifiedTransaction::Valid(txn) => txn.get_read_write_hints(),
            SignatureVerifiedTransaction::Invalid(_) => {
                // Invalid transactions are not execute by the VM, so we don't need to provide
                // read/write hints for them.
                empty_rw_set()
            },
        }
    }
}
