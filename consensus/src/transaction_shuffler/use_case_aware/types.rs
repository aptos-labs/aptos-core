// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::transaction::SignedTransaction;
use move_core_types::account_address::AccountAddress;

pub(crate) type InputIdx = usize;
pub(crate) type OutputIdx = usize;

#[derive(Clone, Eq, Hash, PartialEq)]
pub(crate) enum UseCaseKey {
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
            ContractAddress(addr) => write!(f, "c{}", hex::encode_upper(&addr[31..])),
            Others => write!(f, "OO"),
        }
    }
}

pub(crate) trait UseCaseAwareTransaction {
    fn parse_sender(&self) -> AccountAddress;

    fn parse_use_case(&self) -> UseCaseKey;
}

impl UseCaseAwareTransaction for SignedTransaction {
    fn parse_sender(&self) -> AccountAddress {
        self.sender()
    }

    fn parse_use_case(&self) -> UseCaseKey {
        use aptos_types::transaction::TransactionPayload::*;
        use UseCaseKey::*;

        match self.payload() {
            Script(_) | ModuleBundle(_) | Multisig(_) => Others,
            EntryFunction(entry_fun) => {
                let module_id = entry_fun.module();
                if module_id.address().is_special() {
                    Platform
                } else {
                    // n.b. Generics ignored.
                    ContractAddress(*module_id.address())
                }
            },
        }
    }
}
