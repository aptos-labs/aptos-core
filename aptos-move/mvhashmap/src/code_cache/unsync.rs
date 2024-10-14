// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::vm::code_cache::{ModuleCache, ScriptCache};
use hashbrown::HashMap;
use std::{cell::RefCell, hash::Hash};

/// A per-block code cache to be used for sequential transaction execution.
pub struct UnsyncCodeCache<K, M, Q, S> {
    /// Script cache, indexed by keys such as hashes.
    script_cache: RefCell<HashMap<Q, S>>,
    /// Module cache, indexed by keys such as address and module name pairs.
    module_cache: RefCell<HashMap<K, M>>,
}

impl<K, M, Q, S> UnsyncCodeCache<K, M, Q, S>
where
    K: Eq + Hash + Clone,
    M: Clone,
    Q: Eq + Hash + Clone,
    S: Clone,
{
    /// Returns an empty code cache.
    pub(crate) fn empty() -> Self {
        Self {
            script_cache: RefCell::new(HashMap::new()),
            module_cache: RefCell::new(HashMap::new()),
        }
    }

    /// Returns all modules stored in the code cache.
    pub(crate) fn into_modules_iter(self) -> impl Iterator<Item = (K, M)> {
        self.module_cache.into_inner().into_iter()
    }
}

impl<K, M, Q, S> ScriptCache for UnsyncCodeCache<K, M, Q, S>
where
    K: Eq + Hash + Clone,
    M: Clone,
    Q: Eq + Hash + Clone,
    S: Clone,
{
    type Key = Q;
    type Script = S;

    fn store_script(&self, key: Self::Key, script: Self::Script) {
        self.script_cache.borrow_mut().insert(key, script);
    }

    fn fetch_script(&self, key: &Self::Key) -> Option<Self::Script> {
        self.script_cache.borrow().get(key).cloned()
    }

    fn flush_scripts(&self) {
        self.script_cache.borrow_mut().clear();
    }

    fn num_scripts(&self) -> usize {
        self.script_cache.borrow().len()
    }
}

impl<K, M, Q, S> ModuleCache for UnsyncCodeCache<K, M, Q, S>
where
    K: Eq + Hash + Clone,
    M: Clone,
    Q: Eq + Hash + Clone,
    S: Clone,
{
    type Key = K;
    type Module = M;

    fn store_module(&self, key: Self::Key, module: Self::Module) {
        self.module_cache.borrow_mut().insert(key, module);
    }

    fn fetch_module(&self, key: &Self::Key) -> Option<Self::Module> {
        self.module_cache.borrow().get(key).cloned()
    }

    fn flush_modules(&self) {
        self.module_cache.borrow_mut().clear()
    }

    fn num_modules(&self) -> usize {
        self.module_cache.borrow().len()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_empty() {
        let code_cache = UnsyncCodeCache::<usize, usize, usize, usize>::empty();
        assert_eq!(code_cache.num_scripts(), 0);
        assert_eq!(code_cache.num_modules(), 0);
    }

    #[test]
    fn test_cache_misses() {
        let code_cache = UnsyncCodeCache::<usize, usize, usize, usize>::empty();
        assert_eq!(code_cache.fetch_script(&1), None);
        assert_eq!(code_cache.fetch_module(&1), None);
    }

    #[test]
    fn test_script_cache() {
        let code_cache = UnsyncCodeCache::<usize, usize, usize, usize>::empty();
        code_cache.store_script(1, 1);

        assert_eq!(code_cache.num_scripts(), 1);
        assert_eq!(code_cache.num_modules(), 0);
        assert_eq!(code_cache.fetch_script(&1), Some(1));
    }

    #[test]
    fn test_module_cache() {
        let code_cache = UnsyncCodeCache::<usize, usize, usize, usize>::empty();
        code_cache.store_module(1, 1);

        assert_eq!(code_cache.num_scripts(), 0);
        assert_eq!(code_cache.num_modules(), 1);
        assert_eq!(code_cache.fetch_module(&1), Some(1));
    }

    #[test]
    fn test_flush() {
        let code_cache = UnsyncCodeCache::<usize, usize, usize, usize>::empty();

        code_cache.store_script(1, 1);
        code_cache.store_module(2, 3);
        code_cache.store_module(3, 3);
        assert_eq!(code_cache.num_scripts(), 1);
        assert_eq!(code_cache.num_modules(), 2);

        code_cache.flush_modules();
        assert_eq!(code_cache.num_scripts(), 1);
        assert_eq!(code_cache.num_modules(), 0);

        code_cache.store_script(4, 4);
        code_cache.store_script(5, 5);
        code_cache.store_module(6, 6);
        assert_eq!(code_cache.num_scripts(), 3);
        assert_eq!(code_cache.num_modules(), 1);

        code_cache.flush_scripts();
        assert_eq!(code_cache.num_scripts(), 0);
        assert_eq!(code_cache.num_modules(), 1);
    }
}
