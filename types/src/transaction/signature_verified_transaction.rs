// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    contract_event::ContractEvent,
    state_store::state_key::StateKey,
    transaction::{BlockExecutableTransaction, SignedTransaction, Transaction},
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

impl PartialEq for SignatureVerifiedTransaction {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Invalid(a), Self::Invalid(b)) => a.eq(b),
            (Self::Valid(a), Self::Valid(b)) => a.eq(b),
            _ => {
                panic!("Unexpected equality check on {:?} and {:?}", self, other)
            },
        }
    }
}

impl Eq for SignatureVerifiedTransaction {}

impl SignatureVerifiedTransaction {
    pub fn into_inner(self) -> Transaction {
        match self {
            Self::Valid(txn) => txn,
            Self::Invalid(txn) => txn,
        }
    }

    pub fn borrow_into_inner(&self) -> &Transaction {
        match self {
            Self::Valid(ref txn) => txn,
            Self::Invalid(ref txn) => txn,
        }
    }

    pub fn is_valid(&self) -> bool {
        match self {
            Self::Valid(_) => true,
            Self::Invalid(_) => false,
        }
    }

    pub fn sender(&self) -> Option<AccountAddress> {
        match self {
            Self::Valid(txn) => match txn {
                Transaction::UserTransaction(txn) => Some(txn.sender()),
                _ => None,
            },
            Self::Invalid(_) => None,
        }
    }

    pub fn hash(&self) -> HashValue {
        match self {
            Self::Valid(txn) => txn.hash(),
            Self::Invalid(txn) => txn.hash(),
        }
    }

    pub fn expect_valid(&self) -> &Transaction {
        match self {
            Self::Valid(txn) => txn,
            Self::Invalid(_) => panic!("Expected valid transaction"),
        }
    }
}

impl BlockExecutableTransaction for SignatureVerifiedTransaction {
    type Event = ContractEvent;
    type Key = StateKey;
    type Tag = StructTag;
    type Value = WriteOp;

    fn user_txn_bytes_len(&self) -> usize {
        match self {
            Self::Valid(Transaction::UserTransaction(txn)) => txn.txn_bytes_len(),
            _ => 0,
        }
    }

    fn try_as_signed_user_txn(&self) -> Option<&SignedTransaction> {
        match self {
            Self::Valid(Transaction::UserTransaction(txn)) => Some(txn),
            _ => None,
        }
    }

    fn from_txn(txn: Transaction) -> Self {
        txn.into()
    }
}

impl From<Transaction> for SignatureVerifiedTransaction {
    fn from(txn: Transaction) -> Self {
        match txn {
            Transaction::UserTransaction(txn) => match txn.verify_signature() {
                Ok(_) => Self::Valid(Transaction::UserTransaction(txn)),
                Err(_) => Self::Invalid(Transaction::UserTransaction(txn)),
            },
            _ => Self::Valid(txn),
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
