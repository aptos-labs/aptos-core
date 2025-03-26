// Copyright (c) 2024 Supra.
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::transaction_shuffler::fairness::conflict_key::ConflictKey;
use aptos_types::transaction::{SignedTransaction, TransactionPayload};
use move_core_types::{identifier::Identifier, language_storage::ModuleId};
use move_core_types::identifier::IdentStr;

#[derive(Eq, Hash, PartialEq)]
pub enum EntryFunKey {
    EntryFun {
        module: ModuleId,
        function: Identifier,
    },
    Exempt,
}

impl From<(&ModuleId, &IdentStr)> for EntryFunKey {
    fn from((module_id, function): (&ModuleId, &IdentStr)) -> Self {
        if module_id.address().is_special() {
            // Exempt framework modules
            Self::Exempt
        } else {
            // n.b. Generics ignored.
            Self::EntryFun {
                module: module_id.clone(),
                function: function.to_owned(),
            }
        }
    }
}

impl ConflictKey<SignedTransaction> for EntryFunKey {
    fn extract_from(txn: &SignedTransaction) -> Self {
        match txn.payload() {
            TransactionPayload::AutomationRegistration(auto_payload) => {
                EntryFunKey::from((auto_payload.module_id(), auto_payload.function()))
            }
            TransactionPayload::EntryFunction(entry_fun) => {
                EntryFunKey::from((entry_fun.module(), entry_fun.function()))
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
