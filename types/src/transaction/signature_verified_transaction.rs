// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    contract_event::ContractEvent,
    delayed_fields::DelayedFieldID,
    state_store::state_key::StateKey,
    transaction::{BlockExecutableTransaction, Transaction},
    write_set::WriteOp,
};
use aptos_crypto::{hash::CryptoHash, HashValue};
use move_core_types::{account_address::AccountAddress, language_storage::StructTag};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum SignatureVerifiedTransaction {
    Valid(Transaction),
    Invalid(Transaction),
}

impl SignatureVerifiedTransaction {
    pub fn into_inner(self) -> Transaction {
        match self {
            SignatureVerifiedTransaction::Valid(txn) => txn,
            SignatureVerifiedTransaction::Invalid(txn) => txn,
        }
    }

    pub fn is_valid(&self) -> bool {
        match self {
            SignatureVerifiedTransaction::Valid(_) => true,
            SignatureVerifiedTransaction::Invalid(_) => false,
        }
    }

    pub fn sender(&self) -> Option<AccountAddress> {
        match self {
            SignatureVerifiedTransaction::Valid(txn) => match txn {
                Transaction::UserTransaction(txn) => Some(txn.sender()),
                _ => None,
            },
            SignatureVerifiedTransaction::Invalid(_) => None,
        }
    }

    pub fn hash(&self) -> HashValue {
        match self {
            SignatureVerifiedTransaction::Valid(txn) => txn.hash(),
            SignatureVerifiedTransaction::Invalid(txn) => txn.hash(),
        }
    }

    pub fn expect_valid(&self) -> &Transaction {
        match self {
            SignatureVerifiedTransaction::Valid(txn) => txn,
            SignatureVerifiedTransaction::Invalid(_) => panic!("Expected valid transaction"),
        }
    }
}

impl BlockExecutableTransaction for SignatureVerifiedTransaction {
    type Event = ContractEvent;
    type Identifier = DelayedFieldID;
    type Key = StateKey;
    type Tag = StructTag;
    type Value = WriteOp;

    fn user_txn_bytes_len(&self) -> usize {
        match self {
            SignatureVerifiedTransaction::Valid(Transaction::UserTransaction(txn)) => {
                txn.txn_bytes_len()
            },
            _ => 0,
        }
    }
}

impl From<Transaction> for SignatureVerifiedTransaction {
    fn from(txn: Transaction) -> Self {
        match txn {
            Transaction::UserTransaction(txn) => match txn.verify_signature() {
                Ok(_) => SignatureVerifiedTransaction::Valid(Transaction::UserTransaction(txn)),
                Err(_) => SignatureVerifiedTransaction::Invalid(Transaction::UserTransaction(txn)),
            },
            _ => SignatureVerifiedTransaction::Valid(txn),
        }
    }
}

pub fn into_signature_verified_block(txns: Vec<Transaction>) -> Vec<SignatureVerifiedTransaction> {
    txns.into_iter().map(|t| t.into()).collect()
}

pub trait TransactionProvider: Debug {
    fn get_transaction(&self) -> Option<&Transaction>;
}

impl TransactionProvider for SignatureVerifiedTransaction {
    fn get_transaction(&self) -> Option<&Transaction> {
        if self.is_valid() {
            Some(self.expect_valid())
        } else {
            None
        }
    }
}

impl TransactionProvider for Transaction {
    fn get_transaction(&self) -> Option<&Transaction> {
        Some(self)
    }
}
