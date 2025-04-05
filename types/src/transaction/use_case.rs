// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::transaction::{
    signature_verified_transaction::SignatureVerifiedTransaction, SignedTransaction, Transaction,
    TransactionExecutableRef, TransactionPayload,
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
        use UseCaseKey::*;

        match self.payload().executable_ref() {
            Ok(TransactionExecutableRef::EntryFunction(entry_fun)) => {
                let module_id = entry_fun.module();
                if module_id.address().is_special() {
                    Platform
                } else {
                    ContractAddress(*module_id.address())
                }
            },
            _ => Others,
        }
    }
}

impl UseCaseAwareTransaction for SignatureVerifiedTransaction {
    fn parse_sender(&self) -> AccountAddress {
        self.sender()
            .expect("Expected a sender on SignatureVerifiedTransaction but received None")
    }

    fn parse_use_case(&self) -> UseCaseKey {
        use UseCaseKey::*;

        let payload: Option<&TransactionPayload> = match self {
            SignatureVerifiedTransaction::Valid(txn) => match txn {
                Transaction::UserTransaction(signed_txn) => Some(signed_txn.payload()),
                Transaction::GenesisTransaction(_)
                | Transaction::BlockMetadata(_)
                | Transaction::StateCheckpoint(_)
                | Transaction::ValidatorTransaction(_)
                | Transaction::BlockMetadataExt(_)
                | Transaction::BlockEpilogue(_) => None,
            },
            // TODO I don't think we want invalid transactions during shuffling, but double check this logic...
            SignatureVerifiedTransaction::Invalid(_) => None,
        };

        let payload =
            payload.expect("No payload found for SignatureVerifiedTransaction in parse_use_case");

        match payload.executable_ref() {
            Ok(TransactionExecutableRef::EntryFunction(entry_fun)) => {
                let module_id = entry_fun.module();
                if module_id.address().is_special() {
                    Platform
                } else {
                    ContractAddress(*module_id.address())
                }
            },
            _ => Others,
        }
    }
}
