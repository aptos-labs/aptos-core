// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::LayoutWithDelayedFields;
use move_binary_format::errors::PartialVMResult;
use move_core_types::language_storage::ModuleId;
use move_vm_types::loaded_data::{
    runtime_types::Type, struct_name_indexing::StructNameIndex,
    ty_args_fingerprint::TyArgsFingerprint,
};
use std::{collections::BTreeSet, hash::Hash, sync::Arc};

/// Set of unique modules that define this layout. Iterating over the modules uses the insertion
/// order.
// TODO: consider using IndexedSet
#[derive(Debug)]
pub struct DefiningModules {
    modules: BTreeSet<ModuleId>,
    seen_modules: Vec<ModuleId>,
}

impl DefiningModules {
    pub fn new() -> Self {
        Self {
            modules: BTreeSet::new(),
            seen_modules: vec![],
        }
    }

    pub fn insert(&mut self, module_id: &ModuleId) {
        if !self.modules.contains(module_id) {
            self.modules.insert(module_id.clone());
            self.seen_modules.push(module_id.clone())
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &ModuleId> {
        self.seen_modules.iter()
    }
}

impl Default for DefiningModules {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoIterator for DefiningModules {
    type IntoIter = std::vec::IntoIter<ModuleId>;
    type Item = ModuleId;

    fn into_iter(self) -> Self::IntoIter {
        self.seen_modules.into_iter()
    }
}

#[derive(Debug, Clone)]
pub enum LayoutCacheHit {
    Charged(LayoutWithDelayedFields),
    NotYetCharged(LayoutWithDelayedFields, Arc<DefiningModules>),
}

#[derive(Debug, Clone)]
pub struct LayoutCacheEntry {
    layout: LayoutWithDelayedFields,
    modules: Arc<DefiningModules>,
}

impl LayoutCacheEntry {
    pub(crate) fn new(layout: LayoutWithDelayedFields, modules: DefiningModules) -> Self {
        Self {
            layout,
            modules: Arc::new(modules),
        }
    }

    pub fn layout(&self) -> &LayoutWithDelayedFields {
        &self.layout
    }

    pub fn into_cache_hit(self) -> LayoutCacheHit {
        LayoutCacheHit::NotYetCharged(self.layout, self.modules)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct GenericKey {
    idx: StructNameIndex,
    ty_args_fingerprint: TyArgsFingerprint,
}

impl GenericKey {
    pub(crate) fn new(idx: StructNameIndex, ty_args: &[Type]) -> Self {
        Self {
            idx,
            ty_args_fingerprint: TyArgsFingerprint::from_ty_args(ty_args),
        }
    }
}

pub trait LayoutCache {
    fn get_non_generic_struct_layout(&self, idx: &StructNameIndex) -> Option<LayoutCacheHit>;

    fn store_non_generic_struct_layout(
        &self,
        idx: &StructNameIndex,
        entry: LayoutCacheEntry,
    ) -> PartialVMResult<()>;

    fn get_generic_struct_layout(&self, key: &GenericKey) -> Option<LayoutCacheHit>;

    fn store_generic_struct_layout(
        &self,
        key: GenericKey,
        entry: LayoutCacheEntry,
    ) -> PartialVMResult<()>;
}
