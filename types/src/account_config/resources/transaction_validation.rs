// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::account_config::constants::CORE_CODE_ADDRESS;
use move_deps::move_core_types::{
    account_address::AccountAddress,
    ident_str,
    identifier::{IdentStr, Identifier},
    language_storage::ModuleId,
    move_resource::{MoveResource, MoveStructType},
    vm_status::AbortLocation,
};
use once_cell::sync::Lazy;
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};

pub static APTOS_TRANSACTION_VALIDATION: Lazy<TransactionValidation> =
    Lazy::new(|| TransactionValidation {
        module_addr: CORE_CODE_ADDRESS,
        module_name: Identifier::new("transaction_validation").unwrap(),
        script_prologue_name: Identifier::new("script_prologue").unwrap(),
        module_prologue_name: Identifier::new("module_prologue").unwrap(),
        multi_agent_prologue_name: Identifier::new("multi_agent_script_prologue").unwrap(),
        user_epilogue_name: Identifier::new("epilogue").unwrap(),
    });

/// A Rust representation of chain-specific account information
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct TransactionValidation {
    pub module_addr: AccountAddress,
    pub module_name: Identifier,
    pub script_prologue_name: Identifier,
    pub module_prologue_name: Identifier,
    pub multi_agent_prologue_name: Identifier,
    pub user_epilogue_name: Identifier,
}

impl TransactionValidation {
    pub fn module_id(&self) -> ModuleId {
        ModuleId::new(self.module_addr, self.module_name.clone())
    }

    pub fn is_account_module_abort(&self, location: &AbortLocation) -> bool {
        location == &AbortLocation::Module(self.module_id())
            || location
                == &AbortLocation::Module(ModuleId::new(
                    CORE_CODE_ADDRESS,
                    ident_str!("transaction_validation").to_owned(),
                ))
    }
}

impl MoveStructType for TransactionValidation {
    const MODULE_NAME: &'static IdentStr = ident_str!("transaction_validation");
    const STRUCT_NAME: &'static IdentStr = ident_str!("TransactionValidation");
}

impl MoveResource for TransactionValidation {}
