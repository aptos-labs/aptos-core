// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use std::hash::Hash;

/// Interface used by any script cache implementation.
pub trait ScriptCache {
    type Key: Eq + Hash + Clone;
    type Script: Clone;

    /// Stores the script to the code cache.
    fn store_script(&self, key: Self::Key, script: Self::Script);

    /// Returns the script if it has been cached before, or [None] otherwise.
    fn fetch_script(&self, key: &Self::Key) -> Option<Self::Script>;
}

/// Interface used by any module cache implementation.
pub trait ModuleCache {
    type Key: Eq + Hash + Clone;
    type Module: Clone;
    type Error;

    /// Stores the module to the code cache.
    fn store_module(&self, key: Self::Key, module: Self::Module);

    /// Ensures that the entry in the module cache is initialized based on the default value, if it
    /// was not stored before. Returns the stored module, or [None] if it does not exist.
    fn fetch_module_or_store_with<F>(
        &self,
        key: &Self::Key,
        default: F,
    ) -> Result<Option<Self::Module>, Self::Error>
    where
        F: FnOnce() -> Result<Option<Self::Module>, Self::Error>;
}
