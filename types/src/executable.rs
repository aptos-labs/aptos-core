// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::state_key::{inner::StateKeyInner, StateKey};
use move_core_types::{account_address::AccountAddress, identifier::IdentStr};

pub trait ModulePath: Sized {
    fn is_module_path(&self) -> bool;

    fn from_address_and_module_name(address: &AccountAddress, module_name: &IdentStr) -> Self;
}

impl ModulePath for StateKey {
    fn is_module_path(&self) -> bool {
        matches!(self.inner(), StateKeyInner::AccessPath(ap) if ap.is_code())
    }

    fn from_address_and_module_name(address: &AccountAddress, module_name: &IdentStr) -> Self {
        StateKey::module(address, module_name)
    }
}

/// For now we will handle the VM code cache / arena memory consumption on the
/// executor side, likely naively in the beginning (e.g. flushing after a threshold).
/// For the executor to manage memory consumption, executables should provide size.
/// Note: explore finer-grained eviction mechanisms, e.g. LRU-based, or having
/// different ownership for the arena / memory.
pub trait Executable: Clone + Send + Sync {
    fn size_bytes(&self) -> usize;
}

#[derive(Clone)]
pub struct ExecutableTestType(());

impl Executable for ExecutableTestType {
    fn size_bytes(&self) -> usize {
        0
    }
}
