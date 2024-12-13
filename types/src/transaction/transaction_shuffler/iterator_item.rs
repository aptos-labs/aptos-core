// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::transaction::{
    signature_verified_transaction::SignatureVerifiedTransaction, use_case::UseCaseKey,
    SignedTransaction,
};
use move_core_types::account_address::AccountAddress;
use std::fmt::Debug;

/// This trait deprecates the `UseCaseAwareTransaction` trait and generalizes it for all
/// ShuffledTransaction types, not just transactions handled by the `UseCaseAwareShuffler`
///
/// A `TransactionShufflerIteratorItem` is often just a transaction. Some notable examples:
/// 1. [`SignedTransaction`](transaction::SignedTransaction) or
/// 2. [`SignatureVerifiedTransaction`](transaction::signature_verified_transaction::SignatureVerifiedTransaction)
pub trait TransactionShufflerIteratorItem: Debug {
    fn parse_sender(&self) -> AccountAddress;

    fn parse_use_case(&self) -> UseCaseKey;
}

impl TransactionShufflerIteratorItem for SignedTransaction {
    fn parse_sender(&self) -> AccountAddress {
        self.sender()
    }

    fn parse_use_case(&self) -> UseCaseKey {
        use crate::transaction::TransactionPayload::*;
        use UseCaseKey::*;

        match self.payload() {
            Script(_) | ModuleBundle(_) | Multisig(_) => Others,
            EntryFunction(entry_fun) => {
                let module_id = entry_fun.module();
                if module_id.address().is_special() {
                    Platform
                } else {
                    ContractAddress(*module_id.address())
                }
            },
        }
    }
}

impl TransactionShufflerIteratorItem for SignatureVerifiedTransaction {
    fn parse_sender(&self) -> AccountAddress {
        let txn = match self {
            SignatureVerifiedTransaction::Valid(txn) => txn,
            SignatureVerifiedTransaction::Invalid(txn) => txn,
        };
        match txn {
            crate::transaction::Transaction::UserTransaction(txn) => txn.parse_sender(),
            _ => unreachable!(
                "TransactionShufflerIteratorItem should not be given non-UserTransaction"
            ),
        }
    }

    fn parse_use_case(&self) -> UseCaseKey {
        let txn = match self {
            SignatureVerifiedTransaction::Valid(txn) => txn,
            SignatureVerifiedTransaction::Invalid(txn) => txn,
        };
        match txn {
            crate::transaction::Transaction::UserTransaction(txn) => txn.parse_use_case(),
            _ => unreachable!(
                "TransactionShufflerIteratorItem should not be given non-UserTransaction"
            ),
        }
    }
}
