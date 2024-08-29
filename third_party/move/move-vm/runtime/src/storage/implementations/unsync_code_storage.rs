// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    script_hash,
    storage::{
        environment::WithRuntimeEnvironment,
        implementations::unsync_module_storage::IntoUnsyncModuleStorage,
        module_storage::{ambassador_impl_ModuleStorage, ModuleBytesStorage},
    },
    CodeStorage, Module, ModuleStorage, RuntimeEnvironment, Script, UnsyncModuleStorage,
};
use ambassador::Delegate;
use bytes::Bytes;
use move_binary_format::{
    access::ScriptAccess,
    errors::{Location, PartialVMError, VMResult},
    file_format::CompiledScript,
    CompiledModule,
};
use move_core_types::{
    account_address::AccountAddress, identifier::IdentStr, metadata::Metadata,
    vm_status::StatusCode,
};
#[cfg(test)]
use std::collections::BTreeSet;
use std::{
    cell::RefCell,
    collections::{hash_map, HashMap},
    sync::Arc,
};

/// Represents an entry in script cache, either deserialized or verified.
#[derive(Debug)]
enum ScriptStorageEntry {
    Deserialized(Arc<CompiledScript>),
    Verified(Arc<Script>),
}

/// Code storage that stores both modules and scripts (not thread-safe).
#[derive(Delegate)]
#[delegate(ModuleStorage, target = "module_storage")]
pub struct UnsyncCodeStorage<M> {
    script_storage: RefCell<HashMap<[u8; 32], ScriptStorageEntry>>,
    module_storage: M,
}

pub trait IntoUnsyncCodeStorage<'e, B> {
    fn into_unsync_code_storage(
        self,
        env: &'e RuntimeEnvironment,
    ) -> UnsyncCodeStorage<UnsyncModuleStorage<'e, B>>;
}

impl<'e, B: ModuleBytesStorage> IntoUnsyncCodeStorage<'e, B> for B {
    fn into_unsync_code_storage(
        self,
        env: &'e RuntimeEnvironment,
    ) -> UnsyncCodeStorage<UnsyncModuleStorage<'e, B>> {
        UnsyncCodeStorage::new(self.into_unsync_module_storage(env))
    }
}

impl<M: ModuleStorage + WithRuntimeEnvironment> UnsyncCodeStorage<M> {
    /// Create a new storage with no scripts. There are no constrains on which
    /// modules exist in module storage.
    fn new(module_storage: M) -> Self {
        Self {
            script_storage: RefCell::new(HashMap::new()),
            module_storage,
        }
    }

    /// Returns the underlying module storage used by this code storage.
    pub fn module_storage(&self) -> &M {
        &self.module_storage
    }

    /// Deserializes the script into its compiled representation. The deserialization
    /// is based on the current environment configurations.
    fn deserialize_script(&self, serialized_script: &[u8]) -> VMResult<Arc<CompiledScript>> {
        let deserializer_config = &self
            .module_storage
            .runtime_environment()
            .vm_config()
            .deserializer_config;
        let compiled_script =
            CompiledScript::deserialize_with_config(serialized_script, deserializer_config)
                .map_err(|err| {
                    // Note: we remap the error to be consistent with loader V1 implementation!
                    let msg = format!("[VM] deserializer for script returned error: {:?}", err);
                    PartialVMError::new(StatusCode::CODE_DESERIALIZATION_ERROR)
                        .with_message(msg)
                        .finish(Location::Script)
                })?;
        Ok(Arc::new(compiled_script))
    }

    /// Given a deserialized script, verifies it. The verification consists of three steps:
    ///   1. Verify the script locally, e.g., using bytecode verifier.
    ///   2. Load dependencies used by this script. How the dependencies are loaded is
    ///      opaque to this code storage, and up to the module storage it uses. In any
    ///      case, loading returns a vector of verified dependencies.
    ///   3. Verify the script correctly imports its dependencies.
    /// If any of this steps fail, an error is returned.
    fn verify_deserialized_script(
        &self,
        compiled_script: Arc<CompiledScript>,
    ) -> VMResult<Arc<Script>> {
        let partially_compiled_script = self
            .module_storage
            .runtime_environment()
            .build_partially_verified_script(compiled_script.clone())?;
        let immediate_dependencies = compiled_script
            .immediate_dependencies_iter()
            .map(|(addr, name)| self.module_storage.fetch_verified_module(addr, name))
            .collect::<VMResult<Vec<_>>>()?;
        Ok(Arc::new(
            self.module_storage
                .runtime_environment()
                .build_verified_script(partially_compiled_script, &immediate_dependencies)?,
        ))
    }
}

impl<M: ModuleStorage + WithRuntimeEnvironment> CodeStorage for UnsyncCodeStorage<M> {
    fn deserialize_and_cache_script(
        &self,
        serialized_script: &[u8],
    ) -> VMResult<Arc<CompiledScript>> {
        use hash_map::Entry::*;
        use ScriptStorageEntry::*;

        let hash = script_hash(serialized_script);
        let mut storage = self.script_storage.borrow_mut();

        Ok(match storage.entry(hash) {
            Occupied(e) => match e.get() {
                Deserialized(compiled_script) => compiled_script.clone(),
                Verified(script) => script.script.clone(),
            },
            Vacant(e) => {
                let compiled_script = self.deserialize_script(serialized_script)?;
                e.insert(Deserialized(compiled_script.clone()));
                compiled_script
            },
        })
    }

    fn verify_and_cache_script(&self, serialized_script: &[u8]) -> VMResult<Arc<Script>> {
        use hash_map::Entry::*;
        use ScriptStorageEntry::*;

        let hash = script_hash(serialized_script);
        let mut storage = self.script_storage.borrow_mut();

        Ok(match storage.entry(hash) {
            Occupied(mut e) => match e.get() {
                Deserialized(compiled_script) => {
                    let script = self.verify_deserialized_script(compiled_script.clone())?;
                    e.insert(Verified(script.clone()));
                    script
                },
                Verified(script) => script.clone(),
            },
            Vacant(e) => {
                let compiled_script = self.deserialize_script(serialized_script)?;
                let script = self.verify_deserialized_script(compiled_script)?;
                e.insert(Verified(script.clone()));
                script
            },
        })
    }
}


#[cfg(test)]
impl<M: ModuleStorage + WithRuntimeEnvironment> UnsyncCodeStorage<M> {
    fn matches<P: Fn(&ScriptStorageEntry) -> bool>(
        &self,
        script_hashes: impl IntoIterator<Item = [u8; 32]>,
        predicate: P,
    ) -> bool {
        let script_storage = self.script_storage.borrow();
        let script_hashes_in_storage = script_storage
            .iter()
            .filter_map(|(hash, entry)| predicate(entry).then_some(*hash))
            .collect::<BTreeSet<_>>();
        let script_hashes = script_hashes.into_iter().collect::<BTreeSet<_>>();
        script_hashes.is_subset(&script_hashes_in_storage)
            && script_hashes_in_storage.is_subset(&script_hashes)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        storage::implementations::unsync_module_storage::{
            test::add_module_bytes, ModuleStorageEntry,
        },
        LocalModuleBytesStorage,
    };
    use claims::assert_ok;
    use move_binary_format::{
        file_format::empty_script_with_dependencies, file_format_common::VERSION_DEFAULT,
    };

    fn script<'a>(dependencies: impl IntoIterator<Item = &'a str>) -> Vec<u8> {
        let mut script = empty_script_with_dependencies(dependencies);
        script.version = VERSION_DEFAULT;

        let mut serialized_script = vec![];
        assert_ok!(script.serialize(&mut serialized_script));
        serialized_script
    }

    #[test]
    fn test_deserialized_script_fetching() {
        use ScriptStorageEntry::*;

        let mut module_bytes_storage = LocalModuleBytesStorage::empty();
        add_module_bytes(&mut module_bytes_storage, "a", vec!["b", "c"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "b", vec![], vec![]);
        add_module_bytes(&mut module_bytes_storage, "c", vec![], vec![]);

        let env = RuntimeEnvironment::test();
        let code_storage = module_bytes_storage.into_unsync_code_storage(&env);

        let serialized_script = script(vec!["a"]);
        let hash_1 = script_hash(&serialized_script);

        assert_ok!(code_storage.deserialize_and_cache_script(&serialized_script));
        assert!(code_storage.matches(vec![hash_1], |e| matches!(e, Deserialized(..))));
        assert!(code_storage.matches(vec![], |e| matches!(e, Verified(..))));

        let serialized_script = script(vec!["b"]);
        let hash_2 = script_hash(&serialized_script);

        assert_ok!(code_storage.deserialize_and_cache_script(&serialized_script));
        assert!(code_storage.module_storage().does_not_have_cached_modules());
        assert!(code_storage.matches(vec![hash_1, hash_2], |e| matches!(e, Deserialized(..))));
        assert!(code_storage.matches(vec![], |e| matches!(e, Verified(..))));
    }

    #[test]
    fn test_verified_script_fetching() {
        use ModuleStorageEntry as M;
        use ScriptStorageEntry as S;

        let mut module_bytes_storage = LocalModuleBytesStorage::empty();
        add_module_bytes(&mut module_bytes_storage, "a", vec!["b", "c"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "b", vec![], vec![]);
        add_module_bytes(&mut module_bytes_storage, "c", vec![], vec![]);

        let env = RuntimeEnvironment::test();
        let code_storage = module_bytes_storage.into_unsync_code_storage(&env);

        let serialized_script = script(vec!["a"]);
        let hash = script_hash(&serialized_script);
        assert_ok!(code_storage.deserialize_and_cache_script(&serialized_script));
        assert!(code_storage.module_storage().does_not_have_cached_modules());
        assert!(code_storage.matches(vec![hash], |e| matches!(e, S::Deserialized(..))));
        assert!(code_storage.matches(vec![], |e| matches!(e, S::Verified(..))));

        assert_ok!(code_storage.verify_and_cache_script(&serialized_script));

        assert!(code_storage.matches(vec![], |e| matches!(e, S::Deserialized(..))));
        assert!(code_storage.matches(vec![hash], |e| matches!(e, S::Verified(..))));
        assert!(code_storage
            .module_storage()
            .matches(vec![], |e| matches!(e, M::Deserialized(..))));
        assert!(code_storage
            .module_storage()
            .matches(vec!["a", "b", "c"], |e| matches!(e, M::Verified(..))));
    }
}
