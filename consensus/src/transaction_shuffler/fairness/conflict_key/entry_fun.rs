// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::transaction_shuffler::fairness::conflict_key::ConflictKey;
use aptos_types::transaction::{SignedTransaction, TransactionPayload};
use move_core_types::{identifier::Identifier, language_storage::ModuleId};

#[derive(Eq, Hash, PartialEq)]
pub enum EntryFunKey {
    EntryFun {
        module: ModuleId,
        function: Identifier,
    },
    Exempt,
}

impl ConflictKey<SignedTransaction> for EntryFunKey {
    fn extract_from(txn: &SignedTransaction) -> Self {
        match txn.payload() {
            TransactionPayload::EntryFunction(entry_fun) => {
                let module_id = entry_fun.module();
                if module_id.address().is_special() {
                    // Exempt framework modules
                    Self::Exempt
                } else {
                    // n.b. Generics ignored.
                    Self::EntryFun {
                        module: module_id.clone(),
                        function: entry_fun.function().to_owned(),
                    }
                }
            },
            TransactionPayload::Multisig(_)
            | TransactionPayload::Script(_)
            | TransactionPayload::ModuleBundle(_) => Self::Exempt,
        }
    }

    fn conflict_exempt(&self) -> bool {
        match self {
            Self::Exempt => true,
            Self::EntryFun { .. } => false,
        }
    }
}
