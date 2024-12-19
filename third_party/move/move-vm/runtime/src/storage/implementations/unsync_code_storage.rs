// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    loader::{Module, Script},
    storage::{
        code_storage::{ambassador_impl_CodeStorage, CodeStorage},
        environment::{
            ambassador_impl_WithRuntimeEnvironment, RuntimeEnvironment, WithRuntimeEnvironment,
        },
        implementations::unsync_module_storage::{AsUnsyncModuleStorage, UnsyncModuleStorage},
        module_storage::{ambassador_impl_ModuleStorage, ModuleStorage},
    },
};
use ambassador::Delegate;
use bytes::Bytes;
use move_binary_format::{errors::VMResult, file_format::CompiledScript, CompiledModule};
use move_core_types::{account_address::AccountAddress, identifier::IdentStr, metadata::Metadata};
use move_vm_types::code::{
    ambassador_impl_ScriptCache, Code, ModuleBytesStorage, ScriptCache, UnsyncScriptCache,
};
use std::sync::Arc;

/// Code storage that stores both modules and scripts (not thread-safe).
#[allow(clippy::duplicated_attributes)]
#[derive(Delegate)]
#[delegate(WithRuntimeEnvironment, where = "M: ModuleStorage")]
#[delegate(ModuleStorage, where = "M: ModuleStorage")]
#[delegate(CodeStorage, where = "M: ModuleStorage")]
pub struct UnsyncCodeStorage<M>(UnsyncCodeStorageImpl<M>);

impl<M: ModuleStorage> UnsyncCodeStorage<M> {
    /// Returns the reference to the underlying module storage used by this code storage.
    pub fn module_storage(&self) -> &M {
        &self.0.module_storage
    }

    /// Returns the underlying module storage used by this code storage.
    pub fn into_module_storage(self) -> M {
        self.0.module_storage
    }

    /// Test-only method that checks the state of the script cache.
    #[cfg(test)]
    pub(crate) fn assert_cached_state<'b>(
        &self,
        deserialized: Vec<&'b [u8; 32]>,
        verified: Vec<&'b [u8; 32]>,
    ) {
        assert_eq!(self.0.num_scripts(), deserialized.len() + verified.len());
        for hash in deserialized {
            let script = claims::assert_some!(self.0.get_script(hash));
            assert!(!script.is_verified())
        }
        for hash in verified {
            let script = claims::assert_some!(self.0.get_script(hash));
            assert!(script.is_verified())
        }
    }
}

/// Private implementation of code storage based on non-[Sync] script cache.
#[allow(clippy::duplicated_attributes)]
#[derive(Delegate)]
#[delegate(
    WithRuntimeEnvironment,
    target = "module_storage",
    where = "M: ModuleStorage"
)]
#[delegate(ModuleStorage, target = "module_storage", where = "M: ModuleStorage")]
#[delegate(ScriptCache, target = "script_cache", where = "M: ModuleStorage")]
pub struct UnsyncCodeStorageImpl<M> {
    script_cache: UnsyncScriptCache<[u8; 32], CompiledScript, Script>,
    module_storage: M,
}

impl<M: ModuleStorage> UnsyncCodeStorageImpl<M> {
    /// Creates a new storage with no scripts. There are no constraints on which modules exist in
    /// module storage.
    fn new(module_storage: M) -> Self {
        Self {
            script_cache: UnsyncScriptCache::empty(),
            module_storage,
        }
    }
}

pub trait AsUnsyncCodeStorage<'s, S, E> {
    fn as_unsync_code_storage(
        &'s self,
        runtime_environment: E,
    ) -> UnsyncCodeStorage<UnsyncModuleStorage<'s, S, E>>;

    fn into_unsync_code_storage(
        self,
        runtime_environment: E,
    ) -> UnsyncCodeStorage<UnsyncModuleStorage<'s, S, E>>;
}

impl<'s, S: ModuleBytesStorage, E: WithRuntimeEnvironment> AsUnsyncCodeStorage<'s, S, E> for S {
    fn as_unsync_code_storage(
        &'s self,
        runtime_environment: E,
    ) -> UnsyncCodeStorage<UnsyncModuleStorage<'s, S, E>> {
        UnsyncCodeStorage(UnsyncCodeStorageImpl::new(
            self.as_unsync_module_storage(runtime_environment),
        ))
    }

    fn into_unsync_code_storage(
        self,
        runtime_environment: E,
    ) -> UnsyncCodeStorage<UnsyncModuleStorage<'s, S, E>> {
        UnsyncCodeStorage(UnsyncCodeStorageImpl::new(
            self.into_unsync_module_storage(runtime_environment),
        ))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::storage::{
        code_storage::CodeStorage, implementations::unsync_module_storage::test::add_module_bytes,
    };
    use claims::assert_ok;
    use move_binary_format::{
        file_format::empty_script_with_dependencies, file_format_common::VERSION_DEFAULT,
    };
    use move_core_types::{identifier::Identifier, language_storage::ModuleId};
    use move_vm_test_utils::InMemoryStorage;
    use move_vm_types::sha3_256;

    fn make_script<'a>(dependencies: impl IntoIterator<Item = &'a str>) -> Vec<u8> {
        let mut script = empty_script_with_dependencies(dependencies);
        script.version = VERSION_DEFAULT;

        let mut serialized_script = vec![];
        assert_ok!(script.serialize(&mut serialized_script));
        serialized_script
    }

    #[test]
    fn test_deserialized_script_caching() {
        let mut module_bytes_storage = InMemoryStorage::new();
        add_module_bytes(&mut module_bytes_storage, "a", vec!["b", "c"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "b", vec![], vec![]);
        add_module_bytes(&mut module_bytes_storage, "c", vec![], vec![]);

        let runtime_environment = RuntimeEnvironment::new(vec![]);
        let code_storage = module_bytes_storage.into_unsync_code_storage(runtime_environment);

        let serialized_script = make_script(vec!["a"]);
        let hash_1 = sha3_256(&serialized_script);
        assert_ok!(code_storage.deserialize_and_cache_script(&serialized_script));

        let serialized_script = make_script(vec!["b"]);
        let hash_2 = sha3_256(&serialized_script);
        assert_ok!(code_storage.deserialize_and_cache_script(&serialized_script));

        code_storage.assert_cached_state(vec![&hash_1, &hash_2], vec![]);
    }

    #[test]
    fn test_verified_script_caching() {
        let mut module_bytes_storage = InMemoryStorage::new();

        let a_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("a").unwrap());
        let b_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("b").unwrap());
        let c_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("c").unwrap());

        add_module_bytes(&mut module_bytes_storage, "a", vec!["b", "c"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "b", vec![], vec![]);
        add_module_bytes(&mut module_bytes_storage, "c", vec![], vec![]);

        let runtime_environment = RuntimeEnvironment::new(vec![]);
        let code_storage = module_bytes_storage.into_unsync_code_storage(runtime_environment);

        let serialized_script = make_script(vec!["a"]);
        let hash = sha3_256(&serialized_script);
        assert_ok!(code_storage.deserialize_and_cache_script(&serialized_script));

        // Nothing gets loaded into module cache.
        code_storage
            .module_storage()
            .assert_cached_state(vec![], vec![]);
        code_storage.assert_cached_state(vec![&hash], vec![]);

        assert_ok!(code_storage.verify_and_cache_script(&serialized_script));

        // Script is verified, so its dependencies are loaded into cache.
        code_storage
            .module_storage()
            .assert_cached_state(vec![], vec![&a_id, &b_id, &c_id]);
        code_storage.assert_cached_state(vec![], vec![&hash]);
    }
}
