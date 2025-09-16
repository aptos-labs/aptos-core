// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Data structures for caching layouts of VM values ina long-living, concurrent cache. The design
//! goal is to make sure that cache hit is semantically equivalent to layout construction. That is,
//! if layout construction costs 5 units of gas, so does the cache hit. Failed layout construction
//! never results in insertion into the cache.
//!
//! Additionally, caches store only roots of the layout tree. There is NO caching for sub-layouts
//! because:
//!   1. This is more error-prone because enforcing of semantic equivalence when reading sub-layout
//!      is more difficult: e.g., one needs to ensure the depth and size of layouts are correct.
//!   2. Arguably, we need layouts for root-like values only (e.g., those with `key` ability).

use crate::LayoutWithDelayedFields;
use move_binary_format::errors::PartialVMResult;
use move_core_types::language_storage::ModuleId;
use move_vm_types::{loaded_data::struct_name_indexing::StructNameIndex, ty_interner::TypeVecId};
use std::collections::BTreeSet;
use triomphe::Arc as TriompheArc;

/// Set of unique modules that are used to construct a type layout. Iterating over the modules uses
/// the same order as when constructing layout.
#[derive(Debug, Default)]
pub struct DefiningModules {
    modules: BTreeSet<ModuleId>,
    seen_modules: Vec<ModuleId>,
}

impl DefiningModules {
    /// Returns a new empty set of modules.
    pub fn new() -> Self {
        Self {
            modules: BTreeSet::new(),
            seen_modules: vec![],
        }
    }

    /// If module is not in the set, adds it.
    pub fn insert(&mut self, module_id: &ModuleId) {
        if !self.modules.contains(module_id) {
            self.modules.insert(module_id.clone());
            self.seen_modules.push(module_id.clone())
        }
    }

    /// Returns an iterator over modules in their insertion order.
    pub fn iter(&self) -> impl Iterator<Item = &ModuleId> {
        self.seen_modules.iter()
    }
}

/// An entry into layout cache: layout and a set of modules used to construct it.
#[derive(Debug, Clone)]
pub struct LayoutCacheEntry {
    layout: LayoutWithDelayedFields,
    modules: TriompheArc<DefiningModules>,
}

impl LayoutCacheEntry {
    pub(crate) fn new(layout: LayoutWithDelayedFields, modules: DefiningModules) -> Self {
        Self {
            layout,
            modules: TriompheArc::new(modules),
        }
    }

    pub(crate) fn unpack(self) -> (LayoutWithDelayedFields, TriompheArc<DefiningModules>) {
        (self.layout, self.modules)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct StructKey {
    pub idx: StructNameIndex,
    pub ty_args_id: TypeVecId,
}

/// Interface for fetching and storing layouts into the cache.
pub trait LayoutCache {
    /// If layout root is cached, returns the cached entry (with the modules that were used to
    /// construct it). The reader must ensure to read the module-set for gas charging of validation
    /// purposes.
    fn get_struct_layout(&self, key: &StructKey) -> Option<LayoutCacheEntry>;

    /// Stores layout into cache. If layout already exists (e.g., concurrent insertion) - a no-op.
    fn store_struct_layout(&self, key: &StructKey, entry: LayoutCacheEntry) -> PartialVMResult<()>;
}

/// Marker for no-op layout caches.
pub trait NoOpLayoutCache {}

impl<T> LayoutCache for T
where
    T: NoOpLayoutCache,
{
    fn get_struct_layout(&self, _key: &StructKey) -> Option<LayoutCacheEntry> {
        None
    }

    fn store_struct_layout(
        &self,
        _key: &StructKey,
        _entry: LayoutCacheEntry,
    ) -> PartialVMResult<()> {
        Ok(())
    }
}
