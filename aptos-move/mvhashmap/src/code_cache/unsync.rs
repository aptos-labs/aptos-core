// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::types::VersionedModule;
use hashbrown::{hash_map::Entry, HashMap};
use move_vm_types::code::ModuleCache;
use std::{cell::RefCell, hash::Hash, sync::Arc};

/// A per-block code cache to be used for sequential transaction execution.
pub struct UnsyncModuleCache<K, M> {
    module_cache: RefCell<HashMap<K, Arc<VersionedModule<M>>>>,
}

impl<K, M> UnsyncModuleCache<K, M>
where
    K: Eq + Hash + Clone,
{
    /// Returns an empty module cache.
    pub(crate) fn empty() -> Self {
        Self {
            module_cache: RefCell::new(HashMap::new()),
        }
    }

    /// Returns the number of modules stored in code cache.
    pub fn num_modules(&self) -> usize {
        self.module_cache.borrow().len()
    }

    /// Returns all modules stored in the code cache.
    pub(crate) fn into_modules_iter(self) -> impl Iterator<Item = (K, M)> {
        // TODO(loader_v2): Use panic error instead?
        self.module_cache
            .into_inner()
            .into_iter()
            .map(|(key, versioned_module)| {
                (
                    key,
                    Arc::into_inner(versioned_module)
                        .expect("Should be uniquely owned")
                        .into_module(),
                )
            })
    }
}

impl<K, M> ModuleCache for UnsyncModuleCache<K, M>
where
    K: Eq + Hash + Clone,
{
    type Key = K;
    type Module = VersionedModule<M>;

    fn insert_module(&self, key: Self::Key, module: Self::Module) {
        self.module_cache.borrow_mut().insert(key, Arc::new(module));
    }

    fn get_module_or_insert_with<F, E>(
        &self,
        key: &Self::Key,
        default: F,
    ) -> Result<Option<Arc<Self::Module>>, E>
    where
        F: FnOnce() -> Result<Option<Self::Module>, E>,
    {
        Ok(match self.module_cache.borrow_mut().entry(key.clone()) {
            Entry::Occupied(e) => Some(e.get().clone()),
            Entry::Vacant(e) => default()?.map(|m| e.insert(Arc::new(m)).clone()),
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use claims::{assert_ok, assert_some};
    use std::ops::Deref;

    macro_rules! assert_ok_some_eq {
        ($result:expr, $expected:expr) => {
            let option = assert_ok!($result);
            let actual = assert_some!(option);
            assert_eq!(actual.as_ref().deref(), &$expected);
        };
    }

    fn ok<M>(maybe_module: Option<M>) -> Result<Option<VersionedModule<M>>, ()> {
        Ok(maybe_module.map(VersionedModule::from_pre_block_state))
    }

    fn new<M>(module: M) -> VersionedModule<M> {
        VersionedModule::from_pre_block_state(module)
    }

    #[test]
    fn test_empty() {
        let code_cache = UnsyncModuleCache::<usize, usize>::empty();
        assert_eq!(code_cache.num_modules(), 0);
    }

    #[test]
    fn test_cache_misses() {
        let code_cache = UnsyncModuleCache::<usize, usize>::empty();

        let result = code_cache.get_module_or_insert_with(&1, || ok(None));
        assert_some!(assert_ok!(result));
        assert_eq!(code_cache.num_modules(), 0);

        let result = code_cache.get_module_or_insert_with(&1, || ok(Some(77)));
        assert_ok_some_eq!(result, 77);
        assert_eq!(code_cache.num_modules(), 1);

        let result = code_cache.get_module_or_insert_with(&1, || ok(Some(2)));
        assert_ok_some_eq!(result, 77);

        let result = code_cache.get_module_or_insert_with(&2, || Err(()));
        assert!(result.is_err());
        assert_eq!(code_cache.num_modules(), 1);
    }

    #[test]
    fn test_module_cache() {
        let code_cache = UnsyncModuleCache::<usize, usize>::empty();
        code_cache.insert_module(1, new(1));

        assert_eq!(code_cache.num_modules(), 1);

        for default in [ok(None), ok(Some(10)), Err(())] {
            let result = code_cache.get_module_or_insert_with(&1, || default);
            assert_ok_some_eq!(result, 1);
        }
    }
}
