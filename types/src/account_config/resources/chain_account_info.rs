// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::account_config::constants::{
    CORE_ACCOUNT_MODULE_IDENTIFIER, CORE_CODE_ADDRESS, DIEM_ACCOUNT_MODULE_IDENTIFIER,
};
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    identifier::{IdentStr, Identifier},
    language_storage::ModuleId,
    move_resource::{MoveResource, MoveStructType},
    vm_status::{known_locations, AbortLocation},
};
use once_cell::sync::Lazy;
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};

pub static DPN_CHAIN_INFO: Lazy<ChainSpecificAccountInfo> =
    Lazy::new(|| ChainSpecificAccountInfo {
        module_addr: CORE_CODE_ADDRESS,
        module_name: DIEM_ACCOUNT_MODULE_IDENTIFIER.to_owned(),
        script_prologue_name: Identifier::new("script_prologue").unwrap(),
        module_prologue_name: Identifier::new("module_prologue").unwrap(),
        writeset_prologue_name: Identifier::new("writeset_prologue").unwrap(),
        multi_agent_prologue_name: Identifier::new("multi_agent_script_prologue").unwrap(),
        user_epilogue_name: Identifier::new("epilogue").unwrap(),
        writeset_epilogue_name: Identifier::new("writeset_epilogue").unwrap(),
        currency_code_required: true,
    });

/// A Rust representation of chain-specific account information
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct ChainSpecificAccountInfo {
    pub module_addr: AccountAddress,
    pub module_name: Identifier,
    pub script_prologue_name: Identifier,
    pub module_prologue_name: Identifier,
    pub writeset_prologue_name: Identifier,
    pub multi_agent_prologue_name: Identifier,
    pub user_epilogue_name: Identifier,
    pub writeset_epilogue_name: Identifier,
    pub currency_code_required: bool,
}

impl ChainSpecificAccountInfo {
    pub fn module_id(&self) -> ModuleId {
        ModuleId::new(self.module_addr, self.module_name.clone())
    }

    pub fn is_account_module_abort(&self, location: &AbortLocation) -> bool {
        location == &AbortLocation::Module(self.module_id())
            || location == &AbortLocation::Module(known_locations::CORE_ACCOUNT_MODULE.clone())
    }
}

impl MoveStructType for ChainSpecificAccountInfo {
    const MODULE_NAME: &'static IdentStr = CORE_ACCOUNT_MODULE_IDENTIFIER;
    const STRUCT_NAME: &'static IdentStr = ident_str!("ChainSpecificAccountInfo");
}

impl MoveResource for ChainSpecificAccountInfo {}
