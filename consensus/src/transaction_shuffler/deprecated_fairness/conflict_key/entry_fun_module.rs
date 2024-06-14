// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::transaction_shuffler::deprecated_fairness::conflict_key::ConflictKey;
use aptos_types::transaction::{SignedTransaction, TransactionPayload};
use move_core_types::language_storage::ModuleId;

#[derive(Eq, Hash, PartialEq)]
pub enum EntryFunModuleKey {
    Module(ModuleId),
    AnyScriptOrMultiSig,
    Exempt,
}

impl ConflictKey<SignedTransaction> for EntryFunModuleKey {
    fn extract_from(txn: &SignedTransaction) -> Self {
        match txn.payload() {
            TransactionPayload::EntryFunction(entry_fun) => {
                let module_id = entry_fun.module();

                if module_id.address().is_special() {
                    Self::Exempt
                } else {
                    Self::Module(module_id.clone())
                }
            },
            TransactionPayload::Multisig(..)
            | TransactionPayload::Script(_)
            | TransactionPayload::ModuleBundle(_) => Self::AnyScriptOrMultiSig,
        }
    }

    fn conflict_exempt(&self) -> bool {
        match self {
            Self::Exempt => true,
            Self::Module(..) | Self::AnyScriptOrMultiSig => false,
        }
    }
}
