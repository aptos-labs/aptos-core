// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::state_key::{inner::StateKeyInner, StateKey};
use move_core_types::{account_address::AccountAddress, identifier::IdentStr};

pub trait ModulePath {
    // TODO(loader_v2):
    //   Improve this in the future, right now all writes use state keys and we need to use this
    //   trait to check if a generic state key is for code or not.
    fn is_module_path(&self) -> bool;

    fn from_address_and_module_name(address: &AccountAddress, module_name: &IdentStr) -> Self;
}

impl ModulePath for StateKey {
    fn is_module_path(&self) -> bool {
        matches!(self.inner(), StateKeyInner::AccessPath(ap) if ap.is_code())
    }

    fn from_address_and_module_name(address: &AccountAddress, module_name: &IdentStr) -> Self {
        Self::module(address, module_name)
    }
}
