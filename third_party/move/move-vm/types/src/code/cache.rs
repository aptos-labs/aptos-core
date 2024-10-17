// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use std::{hash::Hash, sync::Arc};

/// Interface used by any script cache implementation.
pub trait ScriptCache {
    type Key: Eq + Hash + Clone;
    type Script: Clone;

    /// Inserts the script to the code cache.
    // TODO(loader_v2): Document the return type and when we insert, when not.
    fn insert_script(&self, key: Self::Key, script: Self::Script);

    /// Returns the script if it has been cached before, or [None] otherwise.
    fn get_script(&self, key: &Self::Key) -> Option<Self::Script>;
}

/// Interface used by any module cache implementation.
pub trait ModuleCache {
    type Key: Eq + Hash + Clone;
    type Module;

    /// Inserts the module to the code cache.
    // TODO(loader_v2): Document the return type and when we insert, when not.
    fn insert_module(&self, key: Self::Key, module: Self::Module);

    /// Ensures that the entry in the module cache is initialized based on the default value, if it
    /// was not stored before. Returns the stored module, or [None] if it does not exist.
    fn get_module_or_insert_with<F, E>(
        &self,
        key: &Self::Key,
        default: F,
    ) -> Result<Option<Arc<Self::Module>>, E>
    where
        F: FnOnce() -> Result<Option<Self::Module>, E>;
}
