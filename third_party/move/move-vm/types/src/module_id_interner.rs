// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::interner::BTreeInterner;
use move_core_types::language_storage::ModuleId;
use parking_lot::RwLock;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct InternedModuleId(pub usize);

pub struct InternedModuleIdPool {
    inner: RwLock<BTreeInterner<ModuleId>>,
}

impl InternedModuleIdPool {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(BTreeInterner::new()),
        }
    }

    pub fn intern(&self, module_id: ModuleId) -> InternedModuleId {
        InternedModuleId(self.inner.write().intern(module_id))
    }

    pub fn len(&self) -> usize {
        self.inner.read().len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.read().is_empty()
    }
}

impl Default for InternedModuleIdPool {
    fn default() -> Self {
        Self::new()
    }
}
