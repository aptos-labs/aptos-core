// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::interner::ConcurrentBTreeInterner;
use move_core_types::language_storage::ModuleId;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct InternedModuleId(usize);

pub struct InternedModuleIdPool {
    inner: ConcurrentBTreeInterner<ModuleId>,
}

impl InternedModuleIdPool {
    pub fn new() -> Self {
        Self {
            inner: ConcurrentBTreeInterner::new(),
        }
    }

    pub fn intern(&self, module_id: ModuleId) -> InternedModuleId {
        InternedModuleId(self.inner.intern(module_id))
    }

    pub fn intern_by_ref(&self, module_id: &ModuleId) -> InternedModuleId {
        InternedModuleId(self.inner.intern_by_ref(module_id))
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn flush(&self) {
        self.inner.flush();
    }
}

impl Default for InternedModuleIdPool {
    fn default() -> Self {
        Self::new()
    }
}

/// Utilities to allow easy creation of [`InternedModuleId`] for testing, while still preserving
/// its equality property.
///
/// This is done by allocating module ids from a global pool, which is only present in tests.
#[cfg(test)]
pub mod test_util {
    use super::*;
    use once_cell::sync::Lazy;

    pub static TEST_MODULE_ID_POOL: Lazy<InternedModuleIdPool> =
        Lazy::new(InternedModuleIdPool::new);

    impl InternedModuleId {
        pub fn from_module_id_for_test(module_id: ModuleId) -> Self {
            TEST_MODULE_ID_POOL.intern(module_id)
        }
    }
}
