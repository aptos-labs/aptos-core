// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::transaction::{
    signature_verified_transaction::SignatureVerifiedTransaction, SignedTransaction,
};
use move_core_types::account_address::AccountAddress;

#[derive(Clone, Eq, Hash, PartialEq)]
pub enum UseCaseKey {
    Platform,
    ContractAddress(AccountAddress),
    // ModuleBundle (deprecated anyway), scripts, Multisig.
    Others,
}

impl std::fmt::Debug for UseCaseKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use UseCaseKey::*;

        match self {
            Platform => write!(f, "PP"),
            ContractAddress(addr) => write!(f, "c{}", hex::encode_upper(&addr[29..])),
            Others => write!(f, "OO"),
        }
    }
}

pub trait UseCaseAwareTransaction {
    fn parse_sender(&self) -> AccountAddress;

    fn parse_use_case(&self) -> UseCaseKey;
}

impl UseCaseAwareTransaction for SignedTransaction {
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

impl UseCaseAwareTransaction for SignatureVerifiedTransaction {
    fn parse_sender(&self) -> AccountAddress {
        let txn = match self {
            SignatureVerifiedTransaction::Valid(txn) => txn,
            SignatureVerifiedTransaction::Invalid(txn) => txn,
        };
        match txn {
            crate::transaction::Transaction::UserTransaction(txn) => txn.parse_sender(),
            _ => unreachable!("UseCaseAwareTransaction should not be given non-UserTransaction"),
        }
    }

    fn parse_use_case(&self) -> UseCaseKey {
        let txn = match self {
            SignatureVerifiedTransaction::Valid(txn) => txn,
            SignatureVerifiedTransaction::Invalid(txn) => txn,
        };
        match txn {
            crate::transaction::Transaction::UserTransaction(txn) => txn.parse_use_case(),
            _ => unreachable!("UseCaseAwareTransaction should not be given non-UserTransaction"),
        }
    }
}
