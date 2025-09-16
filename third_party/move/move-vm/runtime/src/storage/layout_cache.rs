// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::LayoutWithDelayedFields;
use move_binary_format::errors::PartialVMResult;
use move_core_types::language_storage::ModuleId;
use move_vm_types::loaded_data::struct_name_indexing::StructNameIndex;
use std::{collections::BTreeSet, sync::Arc};

#[derive(Debug, Clone)]
pub enum LayoutCacheHit {
    Charged(LayoutWithDelayedFields),
    NotYetCharged(LayoutWithDelayedFields, Arc<BTreeSet<ModuleId>>),
}

#[derive(Debug, Clone)]
pub struct LayoutCacheEntry {
    layout: LayoutWithDelayedFields,
    modules: Arc<BTreeSet<ModuleId>>,
}

impl LayoutCacheEntry {
    pub(crate) fn new(layout: LayoutWithDelayedFields, modules: BTreeSet<ModuleId>) -> Self {
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

pub trait LayoutCache {
    fn get_non_generic_struct_layout(&self, idx: &StructNameIndex) -> Option<LayoutCacheHit>;

    fn store_non_generic_struct_layout(
        &self,
        _idx: &StructNameIndex,
        _entry: LayoutCacheEntry,
    ) -> PartialVMResult<()>;
}
