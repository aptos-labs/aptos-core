// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    aggregator::AggregatorID,
    contract_event::ContractEvent,
    state_store::state_key::StateKey,
    transaction::{BlockExecutableTransaction, Transaction},
    write_set::WriteOp,
};
use move_core_types::language_storage::StructTag;
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

    pub fn inner(&self) -> &Transaction {
        match self {
            SignatureVerifiedTransaction::Valid(txn) => txn,
            SignatureVerifiedTransaction::Invalid(txn) => txn,
        }
    }
}

impl BlockExecutableTransaction for SignatureVerifiedTransaction {
    type Event = ContractEvent;
    type Identifier = AggregatorID;
    type Key = StateKey;
    type Tag = StructTag;
    type Value = WriteOp;
}

pub fn into_signature_verified_block(txns: Vec<Transaction>) -> Vec<SignatureVerifiedTransaction> {
    txns.into_iter().map(into_signature_verified).collect()
}

pub fn into_signature_verified(txn: Transaction) -> SignatureVerifiedTransaction {
    match txn {
        Transaction::UserTransaction(txn) => match txn.verify_signature() {
            Ok(_) => SignatureVerifiedTransaction::Valid(Transaction::UserTransaction(txn)),
            Err(_) => SignatureVerifiedTransaction::Invalid(Transaction::UserTransaction(txn)),
        },
        _ => SignatureVerifiedTransaction::Valid(txn),
    }
}

pub trait TransactionProvider: Debug {
    fn get_transaction(&self) -> &Transaction;
}

impl TransactionProvider for SignatureVerifiedTransaction {
    fn get_transaction(&self) -> &Transaction {
        self.inner()
    }
}

impl TransactionProvider for Transaction {
    fn get_transaction(&self) -> &Transaction {
        self
    }
}
