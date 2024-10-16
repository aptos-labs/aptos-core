// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use hashbrown::{hash_map::Entry, HashMap};
use move_vm_types::code::{ModuleCache, ScriptCache};
use std::{cell::RefCell, hash::Hash, marker::PhantomData};

/// A per-block code cache to be used for sequential transaction execution.
pub struct UnsyncCodeCache<K, M, Q, S, E> {
    /// Script cache, indexed by keys such as hashes.
    script_cache: RefCell<HashMap<Q, S>>,
    /// Module cache, indexed by keys such as address and module name pairs.
    module_cache: RefCell<HashMap<K, M>>,

    phantom_data: PhantomData<E>,
}

impl<K, M, Q, S, E> UnsyncCodeCache<K, M, Q, S, E>
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
            phantom_data: PhantomData,
        }
    }

    /// Returns the number of scripts stored in code cache.
    pub fn num_scripts(&self) -> usize {
        self.script_cache.borrow().len()
    }

    /// Returns the number of modules stored in code cache.
    pub fn num_modules(&self) -> usize {
        self.module_cache.borrow().len()
    }

    /// Returns all modules stored in the code cache.
    pub(crate) fn into_modules_iter(self) -> impl Iterator<Item = (K, M)> {
        self.module_cache.into_inner().into_iter()
    }
}

impl<K, M, Q, S, E> ScriptCache for UnsyncCodeCache<K, M, Q, S, E>
where
    K: Eq + Hash + Clone,
    M: Clone,
    Q: Eq + Hash + Clone,
    S: Clone,
{
    type Key = Q;
    type Script = S;

    fn insert_script(&self, key: Self::Key, script: Self::Script) {
        self.script_cache.borrow_mut().insert(key, script);
    }

    fn get_script(&self, key: &Self::Key) -> Option<Self::Script> {
        self.script_cache.borrow().get(key).cloned()
    }
}

impl<K, M, Q, S, E> ModuleCache for UnsyncCodeCache<K, M, Q, S, E>
where
    K: Eq + Hash + Clone,
    M: Clone,
    Q: Eq + Hash + Clone,
    S: Clone,
{
    type Error = E;
    type Key = K;
    type Module = M;

    fn insert_module(&self, key: Self::Key, module: Self::Module) {
        self.module_cache.borrow_mut().insert(key, module);
    }

    fn get_module_or_insert_with<F>(
        &self,
        key: &Self::Key,
        default: F,
    ) -> Result<Option<Self::Module>, Self::Error>
    where
        F: FnOnce() -> Result<Option<Self::Module>, Self::Error>,
    {
        Ok(match self.module_cache.borrow_mut().entry(key.clone()) {
            Entry::Occupied(e) => Some(e.get().clone()),
            Entry::Vacant(e) => default()?.map(|m| e.insert(m).clone()),
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use claims::assert_err;

    #[test]
    fn test_empty() {
        let code_cache = UnsyncCodeCache::<usize, usize, usize, usize, ()>::empty();
        assert_eq!(code_cache.num_scripts(), 0);
        assert_eq!(code_cache.num_modules(), 0);
    }

    #[test]
    fn test_cache_misses() {
        let code_cache = UnsyncCodeCache::<usize, usize, usize, usize, ()>::empty();
        assert_eq!(code_cache.get_script(&1), None);
        assert_eq!(
            code_cache.get_module_or_insert_with(&1, || Ok(None)),
            Ok(None)
        );
        assert_eq!(code_cache.num_modules(), 0);

        assert_eq!(
            code_cache.get_module_or_insert_with(&1, || Ok(Some(77))),
            Ok(Some(77))
        );
        assert_eq!(code_cache.num_scripts(), 0);
        assert_eq!(code_cache.num_modules(), 1);

        assert_eq!(
            code_cache.get_module_or_insert_with(&1, || Ok(Some(2))),
            Ok(Some(77))
        );

        assert_err!(code_cache.get_module_or_insert_with(&2, || Err(())));
        assert_eq!(code_cache.num_modules(), 1);
    }

    #[test]
    fn test_script_cache() {
        let code_cache = UnsyncCodeCache::<usize, usize, usize, usize, ()>::empty();
        code_cache.insert_script(1, 1);

        assert_eq!(code_cache.num_scripts(), 1);
        assert_eq!(code_cache.num_modules(), 0);
        assert_eq!(code_cache.get_script(&1), Some(1));
    }

    #[test]
    fn test_module_cache() {
        let code_cache = UnsyncCodeCache::<usize, usize, usize, usize, ()>::empty();
        code_cache.insert_module(1, 1);

        assert_eq!(code_cache.num_scripts(), 0);
        assert_eq!(code_cache.num_modules(), 1);
        assert_eq!(
            code_cache.get_module_or_insert_with(&1, || Ok(None)),
            Ok(Some(1))
        );
        assert_eq!(
            code_cache.get_module_or_insert_with(&1, || Ok(Some(10))),
            Ok(Some(1))
        );
    }
}
