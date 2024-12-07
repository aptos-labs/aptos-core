// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::transaction::{SignedTransaction, TransactionExecutable, TransactionPayloadV2};
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
            // Question: MultiSig contains an entry function too. Why isn't it handled like the entry function?
            Script(_) | ModuleBundle(_) | Multisig(_) => Others,
            EntryFunction(entry_fun) => {
                let module_id = entry_fun.module();
                if module_id.address().is_special() {
                    Platform
                } else {
                    ContractAddress(*module_id.address())
                }
            },
            V2(TransactionPayloadV2::V1 {
                executable: TransactionExecutable::EntryFunction(entry_fun),
                extra_config: _,
            }) => {
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
