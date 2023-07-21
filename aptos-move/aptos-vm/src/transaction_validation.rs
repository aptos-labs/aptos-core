// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::account_config::constants::CORE_CODE_ADDRESS;
use move_core_types::{
    account_address::AccountAddress, ident_str, identifier::Identifier, language_storage::ModuleId,
    vm_status::AbortLocation,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

pub static APTOS_TRANSACTION_VALIDATION: Lazy<TransactionValidation> =
    Lazy::new(|| TransactionValidation {
        module_addr: CORE_CODE_ADDRESS,
        module_name: Identifier::new("transaction_validation").unwrap(),
        fee_payer_prologue_name: Identifier::new("fee_payer_script_prologue").unwrap(),
        script_prologue_name: Identifier::new("script_prologue").unwrap(),
        module_prologue_name: Identifier::new("module_prologue").unwrap(),
        multi_agent_prologue_name: Identifier::new("multi_agent_script_prologue").unwrap(),
        user_epilogue_name: Identifier::new("epilogue").unwrap(),
        user_epilogue_gas_payer_name: Identifier::new("epilogue_gas_payer").unwrap(),
    });

/// On-chain functions used to validate transactions
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransactionValidation {
    pub module_addr: AccountAddress,
    pub module_name: Identifier,
    pub fee_payer_prologue_name: Identifier,
    pub script_prologue_name: Identifier,
    pub module_prologue_name: Identifier,
    pub multi_agent_prologue_name: Identifier,
    pub user_epilogue_name: Identifier,
    pub user_epilogue_gas_payer_name: Identifier,
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
