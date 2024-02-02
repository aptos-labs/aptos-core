// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::transaction_shuffler::fairness::conflict_key::ConflictKey;
use aptos_types::transaction::{SignedTransaction, TransactionPayload};
use move_core_types::language_storage::ModuleId;

#[derive(Eq, Hash, PartialEq)]
pub enum EntryFunModuleKey {
    Module(ModuleId),
    AnyScriptOrModuleBundle,
    AnyMultiSig,
    Exempt,
}

impl ConflictKey<SignedTransaction> for EntryFunModuleKey {
    fn extract_from(txn: &SignedTransaction) -> Self {
        match txn.payload() {
            TransactionPayload::EntryFunction(entry_fun) => {
                // FIXME(aldenhu): exempt framework modules
                Self::Module(entry_fun.module().clone())
            },
            // FIXME(aldenhu): deal with multisig
            TransactionPayload::Multisig(..) => Self::AnyMultiSig,
            TransactionPayload::Script(_) | TransactionPayload::ModuleBundle(_) => {
                Self::AnyScriptOrModuleBundle
            },
        }
    }

    fn conflict_exempt(&self) -> bool {
        match self {
            Self::Exempt => true,
            Self::Module(..) | Self::AnyScriptOrModuleBundle | Self::AnyMultiSig => false,
        }
    }
}
