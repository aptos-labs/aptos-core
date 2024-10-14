// Copyright © Aptos Foundation
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

    /// Removes all cached scripts from the cache.
    fn flush_scripts(&self);

    /// Returns the number of cached scripts in the cache.
    fn num_scripts(&self) -> usize;
}

/// Interface used by any module cache implementation.
pub trait ModuleCache {
    type Key: Eq + Hash + Clone;
    type Module: Clone;

    /// Stores the module to the code cache.
    fn store_module(&self, key: Self::Key, module: Self::Module);

    /// Returns the module if it has been cached before, or [None] otherwise.
    fn fetch_module(&self, key: &Self::Key) -> Option<Self::Module>;

    /// Removes all cached modules from the cache.
    fn flush_modules(&self);

    /// Returns the number of cached modules in the cache.
    fn num_modules(&self) -> usize;
}
