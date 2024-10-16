// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::{file_format::CompiledScript, CompiledModule};
use move_core_types::{
    account_address::AccountAddress, identifier::IdentStr, language_storage::ModuleId,
};
use std::{collections::BTreeMap, sync::Arc};
use typed_arena::Arena;

pub struct TraversalStorage {
    referenced_scripts: Arena<Arc<CompiledScript>>,
    referenced_modules: Arena<Arc<CompiledModule>>,
    referenced_module_ids: Arena<ModuleId>,
    referenced_module_bundles: Arena<Vec<CompiledModule>>,
}

pub struct TraversalContext<'a> {
    pub visited: BTreeMap<(&'a AccountAddress, &'a IdentStr), ()>,

    pub referenced_scripts: &'a Arena<Arc<CompiledScript>>,
    pub referenced_modules: &'a Arena<Arc<CompiledModule>>,
    pub referenced_module_ids: &'a Arena<ModuleId>,
    pub referenced_module_bundles: &'a Arena<Vec<CompiledModule>>,
}

impl TraversalStorage {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            referenced_scripts: Arena::new(),
            referenced_modules: Arena::new(),
            referenced_module_ids: Arena::new(),
            referenced_module_bundles: Arena::new(),
        }
    }
}

impl<'a> TraversalContext<'a> {
    pub fn new(storage: &'a TraversalStorage) -> Self {
        Self {
            visited: BTreeMap::new(),

            referenced_scripts: &storage.referenced_scripts,
            referenced_modules: &storage.referenced_modules,
            referenced_module_ids: &storage.referenced_module_ids,
            referenced_module_bundles: &storage.referenced_module_bundles,
        }
    }
}
