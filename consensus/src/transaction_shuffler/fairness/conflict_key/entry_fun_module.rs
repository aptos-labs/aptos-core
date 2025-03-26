// Copyright (c) 2024 Supra.
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::transaction_shuffler::fairness::conflict_key::ConflictKey;
use aptos_types::transaction::{SignedTransaction, TransactionPayload};
use move_core_types::language_storage::ModuleId;

#[derive(Eq, Hash, PartialEq)]
pub enum EntryFunModuleKey {
    Module(ModuleId),
    AnyScriptOrMultiSig,
    Exempt,
}

impl From<&ModuleId> for EntryFunModuleKey {
    fn from(module_id: &ModuleId) -> Self {
        if module_id.address().is_special() {
            Self::Exempt
        } else {
            Self::Module(module_id.clone())
        }
    }
}

impl ConflictKey<SignedTransaction> for EntryFunModuleKey {
    fn extract_from(txn: &SignedTransaction) -> Self {
        match txn.payload() {
            TransactionPayload::AutomationRegistration(auto_payload) => Self::from(auto_payload.module_id()),
            TransactionPayload::EntryFunction(entry_fun) => Self::from(entry_fun.module()),
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
