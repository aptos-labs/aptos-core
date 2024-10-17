// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    storage::{
        environment::{ambassador_impl_WithRuntimeEnvironment, WithRuntimeEnvironment},
        implementations::unsync_module_storage::AsUnsyncModuleStorage,
        module_storage::ambassador_impl_ModuleStorage,
    },
    Module, ModuleStorage, RuntimeEnvironment, Script, UnsyncModuleStorage,
};
use ambassador::Delegate;
use bytes::Bytes;
use move_binary_format::{errors::VMResult, file_format::CompiledScript, CompiledModule};
use move_core_types::{account_address::AccountAddress, identifier::IdentStr, metadata::Metadata};
use move_vm_types::code::{CachedScript, ModuleBytesStorage, ScriptCache, UnsyncScriptCache};
use std::sync::Arc;

/// Code storage that stores both modules and scripts (not thread-safe).
#[derive(Delegate)]
#[delegate(
    WithRuntimeEnvironment,
    target = "module_storage",
    where = "M: ModuleStorage"
)]
#[delegate(ModuleStorage, target = "module_storage", where = "M: ModuleStorage")]
pub struct UnsyncCodeStorage<M> {
    script_cache: UnsyncScriptCache<[u8; 32], CompiledScript, Script>,
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
            script_cache: UnsyncScriptCache::empty(),
            module_storage,
        }
    }

    /// Returns the underlying module storage used by this code storage.
    pub fn module_storage(&self) -> &M {
        &self.module_storage
    }
}

impl<M: ModuleStorage> ScriptCache for UnsyncCodeStorage<M> {
    type Deserialized = CompiledScript;
    type Key = [u8; 32];
    type Verified = Script;

    fn insert_deserialized_script(
        &self,
        key: Self::Key,
        deserialized_script: Self::Deserialized,
    ) -> Arc<Self::Deserialized> {
        self.script_cache
            .insert_deserialized_script(key, deserialized_script)
    }

    fn insert_verified_script(
        &self,
        key: Self::Key,
        verified_script: Self::Verified,
    ) -> Arc<Self::Verified> {
        self.script_cache
            .insert_verified_script(key, verified_script)
    }

    fn get_script(
        &self,
        key: &Self::Key,
    ) -> Option<CachedScript<Self::Deserialized, Self::Verified>> {
        self.script_cache.get_script(key)
    }

    fn num_scripts(&self) -> usize {
        self.script_cache.num_scripts()
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
    use claims::{assert_ok, assert_some};
    use move_binary_format::{
        file_format::empty_script_with_dependencies, file_format_common::VERSION_DEFAULT,
    };
    use move_vm_test_utils::InMemoryStorage;

    fn new_script<'a>(dependencies: impl IntoIterator<Item = &'a str>) -> Vec<u8> {
        let mut script = empty_script_with_dependencies(dependencies);
        script.version = VERSION_DEFAULT;

        let mut serialized_script = vec![];
        assert_ok!(script.serialize(&mut serialized_script));
        serialized_script
    }

    #[test]
    fn test_deserialized_script_caching() {
        use CachedScript::*;

        let mut module_bytes_storage = InMemoryStorage::new();
        add_module_bytes(&mut module_bytes_storage, "a", vec!["b", "c"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "b", vec![], vec![]);
        add_module_bytes(&mut module_bytes_storage, "c", vec![], vec![]);

        let runtime_environment = RuntimeEnvironment::new(vec![]);
        let code_storage = module_bytes_storage.into_unsync_code_storage(&runtime_environment);

        let serialized_script = new_script(vec!["a"]);
        let hash_1 = compute_code_hash(&serialized_script);

        assert_ok!(code_storage.deserialize_and_cache_script(&serialized_script));

        assert_eq!(code_storage.script_cache.num_scripts(), 1);
        let script = assert_some!(code_storage.script_cache.get_script(&hash_1));
        assert!(matches!(script, Deserialized(..)));

        let serialized_script = new_script(vec!["b"]);
        let hash_2 = compute_code_hash(&serialized_script);

        assert_ok!(code_storage.deserialize_and_cache_script(&serialized_script));

        assert_eq!(code_storage.script_cache.num_scripts(), 2);
        let script_1 = assert_some!(code_storage.script_cache.get_script(&hash_1));
        assert!(matches!(script_1, Deserialized(..)));
        let script_2 = assert_some!(code_storage.script_cache.get_script(&hash_2));
        assert!(matches!(script_2, Deserialized(..)));
    }

    #[test]
    fn test_verified_script_caching() {
        use CachedScript as S;
        use ModuleCacheEntry as M;

        let mut module_bytes_storage = InMemoryStorage::new();
        add_module_bytes(&mut module_bytes_storage, "a", vec!["b", "c"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "b", vec![], vec![]);
        add_module_bytes(&mut module_bytes_storage, "c", vec![], vec![]);

        let runtime_environment = RuntimeEnvironment::new(vec![]);
        let code_storage = module_bytes_storage.into_unsync_code_storage(&runtime_environment);

        let serialized_script = new_script(vec!["a"]);
        let hash = compute_code_hash(&serialized_script);
        assert_ok!(code_storage.deserialize_and_cache_script(&serialized_script));
        assert!(code_storage.module_storage().does_not_have_cached_modules());

        assert_eq!(code_storage.script_cache.num_scripts(), 1);
        let script = assert_some!(code_storage.script_cache.get_script(&hash));
        assert!(matches!(script, S::Deserialized(..)));

        assert_ok!(code_storage.verify_and_cache_script(&serialized_script));

        assert_eq!(code_storage.script_cache.num_scripts(), 1);
        let script = assert_some!(code_storage.script_cache.get_script(&hash));
        assert!(matches!(script, S::Verified(..)));

        assert!(code_storage
            .module_storage()
            .matches(vec![], |e| matches!(e, M::Deserialized { .. })));
        assert!(code_storage
            .module_storage()
            .matches(vec!["a", "b", "c"], |e| matches!(e, M::Verified { .. })));
    }
}
