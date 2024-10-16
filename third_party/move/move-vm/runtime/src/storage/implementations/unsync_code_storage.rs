// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    storage::{
        environment::{ambassador_impl_WithRuntimeEnvironment, WithRuntimeEnvironment},
        implementations::unsync_module_storage::AsUnsyncModuleStorage,
        module_storage::ambassador_impl_ModuleStorage,
    },
    CachedScript, Module, ModuleStorage, RuntimeEnvironment, UnsyncModuleStorage,
};
use ambassador::Delegate;
use bytes::Bytes;
use move_binary_format::{errors::VMResult, CompiledModule};
use move_core_types::{account_address::AccountAddress, identifier::IdentStr, metadata::Metadata};
use move_vm_types::code::{ModuleBytesStorage, ScriptCache};
#[cfg(test)]
use std::collections::BTreeSet;
use std::{cell::RefCell, collections::HashMap, sync::Arc};

/// Code storage that stores both modules and scripts (not thread-safe).
#[derive(Delegate)]
#[delegate(
    WithRuntimeEnvironment,
    target = "module_storage",
    where = "M: ModuleStorage"
)]
#[delegate(ModuleStorage, target = "module_storage", where = "M: ModuleStorage")]
pub struct UnsyncCodeStorage<M> {
    script_cache: RefCell<HashMap<[u8; 32], CachedScript>>,
    module_storage: M,
}

pub trait AsUnsyncCodeStorage<'a, S> {
    fn as_unsync_code_storage(
        &'a self,
        env: &'a RuntimeEnvironment,
    ) -> UnsyncCodeStorage<UnsyncModuleStorage<'a, S>>;

    fn into_unsync_code_storage(
        self,
        env: &'a RuntimeEnvironment,
    ) -> UnsyncCodeStorage<UnsyncModuleStorage<'a, S>>;
}

impl<'a, S: ModuleBytesStorage> AsUnsyncCodeStorage<'a, S> for S {
    fn as_unsync_code_storage(
        &'a self,
        env: &'a RuntimeEnvironment,
    ) -> UnsyncCodeStorage<UnsyncModuleStorage<'a, S>> {
        UnsyncCodeStorage::new(self.as_unsync_module_storage(env))
    }

    fn into_unsync_code_storage(
        self,
        env: &'a RuntimeEnvironment,
    ) -> UnsyncCodeStorage<UnsyncModuleStorage<'a, S>> {
        UnsyncCodeStorage::new(self.into_unsync_module_storage(env))
    }
}

impl<M: ModuleStorage> UnsyncCodeStorage<M> {
    /// Creates a new storage with no scripts. There are no constraints on which modules exist in
    /// module storage.
    fn new(module_storage: M) -> Self {
        Self {
            script_cache: RefCell::new(HashMap::new()),
            module_storage,
        }
    }

    /// Returns the underlying module storage used by this code storage.
    pub fn module_storage(&self) -> &M {
        &self.module_storage
    }
}

impl<M: ModuleStorage> ScriptCache for UnsyncCodeStorage<M> {
    type Key = [u8; 32];
    type Script = CachedScript;

    fn store_script(&self, key: Self::Key, script: Self::Script) {
        self.script_cache.borrow_mut().insert(key, script);
    }

    fn fetch_script(&self, key: &Self::Key) -> Option<Self::Script> {
        self.script_cache.borrow_mut().get(key).cloned()
    }
}

#[cfg(test)]
impl<M: ModuleStorage> UnsyncCodeStorage<M> {
    fn matches<P: Fn(&CachedScript) -> bool>(
        &self,
        script_hashes: impl IntoIterator<Item = [u8; 32]>,
        predicate: P,
    ) -> bool {
        let script_cache = self.script_cache.borrow();
        let script_hashes_in_cache = script_cache
            .iter()
            .filter_map(|(hash, entry)| predicate(entry).then_some(*hash))
            .collect::<BTreeSet<_>>();
        let script_hashes = script_hashes.into_iter().collect::<BTreeSet<_>>();
        script_hashes.is_subset(&script_hashes_in_cache)
            && script_hashes_in_cache.is_subset(&script_hashes)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        compute_code_hash,
        storage::{
            code_storage::CodeStorage,
            implementations::unsync_module_storage::{test::add_module_bytes, ModuleCacheEntry},
        },
    };
    use claims::assert_ok;
    use move_binary_format::{
        file_format::empty_script_with_dependencies, file_format_common::VERSION_DEFAULT,
    };
    use move_vm_test_utils::InMemoryStorage;

    fn script<'a>(dependencies: impl IntoIterator<Item = &'a str>) -> Vec<u8> {
        let mut script = empty_script_with_dependencies(dependencies);
        script.version = VERSION_DEFAULT;

        let mut serialized_script = vec![];
        assert_ok!(script.serialize(&mut serialized_script));
        serialized_script
    }

    #[test]
    fn test_deserialized_script_fetching() {
        use CachedScript::*;

        let mut module_bytes_storage = InMemoryStorage::new();
        add_module_bytes(&mut module_bytes_storage, "a", vec!["b", "c"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "b", vec![], vec![]);
        add_module_bytes(&mut module_bytes_storage, "c", vec![], vec![]);

        let runtime_environment = RuntimeEnvironment::new(vec![]);
        let code_storage = module_bytes_storage.into_unsync_code_storage(&runtime_environment);

        let serialized_script = script(vec!["a"]);
        let hash_1 = compute_code_hash(&serialized_script);

        assert_ok!(code_storage.deserialize_and_cache_script(&serialized_script));
        assert!(code_storage.matches(vec![hash_1], |e| matches!(e, Deserialized(..))));
        assert!(code_storage.matches(vec![], |e| matches!(e, Verified(..))));

        let serialized_script = script(vec!["b"]);
        let hash_2 = compute_code_hash(&serialized_script);

        assert_ok!(code_storage.deserialize_and_cache_script(&serialized_script));
        assert!(code_storage.module_storage().does_not_have_cached_modules());
        assert!(code_storage.matches(vec![hash_1, hash_2], |e| matches!(e, Deserialized(..))));
        assert!(code_storage.matches(vec![], |e| matches!(e, Verified(..))));
    }

    #[test]
    fn test_verified_script_fetching() {
        use CachedScript as S;
        use ModuleCacheEntry as M;

        let mut module_bytes_storage = InMemoryStorage::new();
        add_module_bytes(&mut module_bytes_storage, "a", vec!["b", "c"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "b", vec![], vec![]);
        add_module_bytes(&mut module_bytes_storage, "c", vec![], vec![]);

        let runtime_environment = RuntimeEnvironment::new(vec![]);
        let code_storage = module_bytes_storage.into_unsync_code_storage(&runtime_environment);

        let serialized_script = script(vec!["a"]);
        let hash = compute_code_hash(&serialized_script);
        assert_ok!(code_storage.deserialize_and_cache_script(&serialized_script));
        assert!(code_storage.module_storage().does_not_have_cached_modules());
        assert!(code_storage.matches(vec![hash], |e| matches!(e, S::Deserialized(..))));
        assert!(code_storage.matches(vec![], |e| matches!(e, S::Verified(..))));

        assert_ok!(code_storage.verify_and_cache_script(&serialized_script));

        assert!(code_storage.matches(vec![], |e| matches!(e, S::Deserialized(..))));
        assert!(code_storage.matches(vec![hash], |e| matches!(e, S::Verified(..))));
        assert!(code_storage
            .module_storage()
            .matches(vec![], |e| matches!(e, M::Deserialized { .. })));
        assert!(code_storage
            .module_storage()
            .matches(vec!["a", "b", "c"], |e| matches!(e, M::Verified { .. })));
    }
}
