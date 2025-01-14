// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::transaction::{
    signature_verified_transaction::SignatureVerifiedTransaction, SignedTransaction,
};
use move_core_types::account_address::AccountAddress;

pub trait SenderAwareTransaction {
    fn parse_sender(&self) -> AccountAddress;
}

impl SenderAwareTransaction for SignedTransaction {
    fn parse_sender(&self) -> AccountAddress {
        self.sender()
    }
}

impl SenderAwareTransaction for SignatureVerifiedTransaction {
    fn parse_sender(&self) -> AccountAddress {
        let txn = match self {
            SignatureVerifiedTransaction::Valid(txn) => txn,
            SignatureVerifiedTransaction::Invalid(txn) => txn,
        };
        match txn {
            crate::transaction::Transaction::UserTransaction(txn) => txn.parse_sender(),
            _ => unreachable!("SenderAwareTransaction should not be given non-UserTransaction"),
        }
    }
}
