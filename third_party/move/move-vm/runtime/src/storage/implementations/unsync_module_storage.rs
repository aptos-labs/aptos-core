// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    module_cyclic_dependency_error, module_linker_error,
    storage::{
        environment::{RuntimeEnvironment, WithRuntimeEnvironment},
        module_storage::{ModuleBytesStorage, ModuleStorage},
    },
    Module,
};
use bytes::Bytes;
#[cfg(test)]
use claims::assert_some;
use move_binary_format::{
    errors::{Location, PartialVMError, VMResult},
    CompiledModule,
};
use move_core_types::{
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
    language_storage::ModuleId,
    metadata::Metadata,
    vm_status::StatusCode,
};
use std::{
    cell::RefCell,
    collections::{btree_map, BTreeMap, BTreeSet},
    sync::Arc,
};

/// Represents an in-memory storage that contains modules' bytes.
#[derive(Clone)]
pub struct LocalModuleBytesStorage(BTreeMap<AccountAddress, BTreeMap<Identifier, Bytes>>);

impl LocalModuleBytesStorage {
    /// Create an empty storage for module bytes.
    pub fn empty() -> Self {
        Self(BTreeMap::new())
    }

    /// Adds serialized module to this module byte storage.
    pub fn add_module_bytes(
        &mut self,
        address: &AccountAddress,
        module_name: &IdentStr,
        bytes: Bytes,
    ) {
        use btree_map::Entry::*;
        let account_module_storage = match self.0.entry(*address) {
            Occupied(entry) => entry.into_mut(),
            Vacant(entry) => entry.insert(BTreeMap::new()),
        };
        account_module_storage.insert(module_name.to_owned(), bytes);
    }
}

impl ModuleBytesStorage for LocalModuleBytesStorage {
    fn fetch_module_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Bytes>> {
        if let Some(account_storage) = self.0.get(address) {
            return Ok(account_storage.get(module_name).cloned());
        }
        Ok(None)
    }
}

/// An entry in [UnsyncModuleStorage]. As modules are accessed, entries can be
/// "promoted", e.g., a deserialized representation can be converted into the
/// verified one.
#[derive(Debug, Clone)]
pub(crate) enum ModuleStorageEntry {
    Deserialized(Arc<CompiledModule>, usize),
    Verified(Arc<Module>),
}

impl ModuleStorageEntry {
    /// Returns the verified module if the entry is verified, and [None] otherwise.
    fn into_verified(self) -> Option<Arc<Module>> {
        match self {
            Self::Deserialized(_, _) => None,
            Self::Verified(module) => Some(module),
        }
    }
}

/// Implementation of (not thread-safe) module storage used for Move unit tests,
/// and externally.
pub struct UnsyncModuleStorage<'e, B> {
    /// Environment where this module storage is defined in.
    runtime_environment: &'e RuntimeEnvironment,
    /// Storage with deserialized modules, i.e., module cache.
    module_storage: RefCell<BTreeMap<AccountAddress, BTreeMap<Identifier, ModuleStorageEntry>>>,
    /// Immutable baseline byte storage from which one can fetch raw module bytes.
    byte_storage: B,
}

pub trait IntoUnsyncModuleStorage<'e, B> {
    fn into_unsync_module_storage(self, env: &'e RuntimeEnvironment) -> UnsyncModuleStorage<'e, B>;
}

impl<'e, B: ModuleBytesStorage> IntoUnsyncModuleStorage<'e, B> for B {
    fn into_unsync_module_storage(self, env: &'e RuntimeEnvironment) -> UnsyncModuleStorage<'e, B> {
        UnsyncModuleStorage::new(env, self)
    }
}

impl<'e, B: ModuleBytesStorage> UnsyncModuleStorage<'e, B> {
    /// Creates a new storage with empty module cache, but no constraints on the byte storage.
    fn new(env: &'e RuntimeEnvironment, byte_storage: B) -> Self {
        Self {
            runtime_environment: env,
            module_storage: RefCell::new(BTreeMap::new()),
            byte_storage,
        }
    }

    /// Returns module bytes, and an error if it does not exist.
    fn get_existing_module_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Bytes> {
        self.fetch_module_bytes(address, module_name)?
            .ok_or_else(|| module_linker_error!(address, module_name))
    }

    /// Returns true if the module is cached.
    fn is_module_cached(&self, address: &AccountAddress, module_name: &IdentStr) -> bool {
        let module_storage = self.module_storage.borrow();
        module_storage
            .get(address)
            .is_some_and(|account_module_storage| account_module_storage.contains_key(module_name))
    }

    /// If module is not yet cached in module storage, fetches it from the baseline
    /// byte storage and caches as a deserialized entry.
    fn initialize_module_storage_entry(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<()> {
        use btree_map::Entry::*;
        use ModuleStorageEntry::*;

        if !self.is_module_cached(address, module_name) {
            let bytes = self.get_existing_module_bytes(address, module_name)?;
            let compiled_module = CompiledModule::deserialize_with_config(
                &bytes,
                &self.runtime_environment.vm_config().deserializer_config,
            )
            .map_err(|err| {
                let msg = format!("Deserialization error: {:?}", err);
                PartialVMError::new(StatusCode::CODE_DESERIALIZATION_ERROR)
                    .with_message(msg)
                    .finish(Location::Module(ModuleId::new(
                        *address,
                        module_name.to_owned(),
                    )))
            })?;
            let mut module_storage = self.module_storage.borrow_mut();
            let account_module_storage = match module_storage.entry(*address) {
                Occupied(entry) => entry.into_mut(),
                Vacant(entry) => entry.insert(BTreeMap::new()),
            };
            account_module_storage.insert(
                module_name.to_owned(),
                Deserialized(Arc::new(compiled_module), bytes.len()),
            );
        }
        Ok(())
    }

    /// Returns the entry in module storage (deserialized or verified) and an error if it
    /// does not exist. This API clones the underlying entry pointers.
    fn fetch_existing_module_storage_entry(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<ModuleStorageEntry> {
        self.initialize_module_storage_entry(address, module_name)?;

        // At this point module storage contains a deserialized entry, because the function
        // above puts it there if it was not cached already.
        let module_storage = self.module_storage.borrow();
        Ok(get_module_entry_or_panic(&module_storage, address, module_name).clone())
    }

    /// Visits the dependencies of the given module. If dependencies form a cycle (which
    /// should not be the case as we check this when modules are added to the module
    /// storage), an error is returned.
    ///
    /// Important: this implementation **does not** load transitive friends. While it is
    /// possible to view friends as `used-by` relation, it cannot be checked fully. For
    /// example, consider the case when we have four modules A, B, C, D and let `X --> Y`
    /// be a dependency relation (Y is a dependency of X) and `X ==> Y ` a friend relation
    /// (X declares Y a friend). Then consider the case `A --> B <== C --> D <== A`. Here,
    /// if we opt for `used-by` semantics, there is a cycle. But it cannot be checked,
    /// since, A only sees B and D, and C sees B and D, but both B and D do not see any
    /// dependencies or friends. Hence, A cannot discover C and vice-versa, making detection
    /// of such corner cases only possible if **all existing modules are checked**, which
    /// is clearly infeasible.
    fn fetch_verified_module_and_visit_all_transitive_dependencies(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
        visited: &mut BTreeSet<ModuleId>,
    ) -> VMResult<Arc<Module>> {
        use ModuleStorageEntry::*;

        // Get the module, and in case it is verified, return early.
        let entry = self.fetch_existing_module_storage_entry(address, module_name)?;
        let (compiled_module, size) = match entry {
            Deserialized(compiled_module, size) => (compiled_module, size),
            Verified(module) => return Ok(module),
        };
        self.runtime_environment
            .paranoid_check_module_address_and_name(
                compiled_module.as_ref(),
                address,
                module_name,
            )?;

        // Step 1: verify compiled module locally.
        let partially_verified_module = self
            .runtime_environment
            .build_partially_verified_module(compiled_module, size)?;

        // Step 2: visit all dependencies and collect them for later verification.
        let mut verified_immediate_dependencies = vec![];
        for (addr, name) in partially_verified_module.immediate_dependencies_iter() {
            // Check if the module has been already visited and verified.
            let dep_entry = self.fetch_existing_module_storage_entry(addr, name)?;
            if let Some(dep_module) = dep_entry.into_verified() {
                verified_immediate_dependencies.push(dep_module);
                continue;
            }

            // Otherwise, either we have visited this module but not yet verified (hence,
            // we found a cycle) or we have not visited it yet and need to verify it.
            let module_id = ModuleId::new(*addr, name.to_owned());
            if visited.insert(module_id) {
                let module = self.fetch_verified_module_and_visit_all_transitive_dependencies(
                    addr, name, visited,
                )?;
                verified_immediate_dependencies.push(module);
            } else {
                return Err(module_cyclic_dependency_error!(address, module_name));
            }
        }

        // Step 3: verify module with dependencies.
        let module =
            Arc::new(self.runtime_environment.build_verified_module(
                partially_verified_module,
                &verified_immediate_dependencies,
            )?);

        // Step 4: update storage representation to fully verified one.
        let mut module_storage = self.module_storage.borrow_mut();
        let entry = get_module_entry_mut_or_panic(&mut module_storage, address, module_name);
        *entry = Verified(module.clone());
        Ok(module)
    }

    /// The baseline byte storage used by this module storage.
    pub fn byte_storage(&self) -> &B {
        &self.byte_storage
    }

    /// Returns the byte storage used by this module storage.
    pub(crate) fn release_byte_storage(self) -> B {
        self.byte_storage
    }
}

impl<'e, B: ModuleBytesStorage> WithRuntimeEnvironment for UnsyncModuleStorage<'e, B> {
    fn runtime_environment(&self) -> &RuntimeEnvironment {
        self.runtime_environment
    }
}

impl<'e, B: ModuleBytesStorage> ModuleStorage for UnsyncModuleStorage<'e, B> {
    fn check_module_exists(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<bool> {
        // Cached modules in module storage are a subset of modules in byte
        // storage, so it is sufficient to check existence based on it.
        Ok(self
            .byte_storage
            .fetch_module_bytes(address, module_name)?
            .is_some())
    }

    fn fetch_module_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Bytes>> {
        self.byte_storage.fetch_module_bytes(address, module_name)
    }

    fn fetch_module_size_in_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<usize>> {
        Ok(self
            .fetch_module_bytes(address, module_name)?
            .map(|b| b.len()))
    }

    fn fetch_module_metadata(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Vec<Metadata>> {
        Ok(self
            .fetch_deserialized_module(address, module_name)?
            .metadata
            .clone())
    }

    fn fetch_deserialized_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Arc<CompiledModule>> {
        use ModuleStorageEntry::*;

        self.initialize_module_storage_entry(address, module_name)?;

        // At this point module storage contains a deserialized entry, because the function
        // above puts it there if it was not cached already.
        let module_storage = self.module_storage.borrow();
        let entry = get_module_entry_or_panic(&module_storage, address, module_name);

        Ok(match entry {
            Deserialized(compiled_module, _) => compiled_module.clone(),
            Verified(module) => module.as_compiled_module(),
        })
    }

    fn fetch_verified_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Arc<Module>> {
        let mut visited = BTreeSet::new();
        self.fetch_verified_module_and_visit_all_transitive_dependencies(
            address,
            module_name,
            &mut visited,
        )
    }
}

fn get_module_entry_or_panic<'a>(
    module_storage: &'a BTreeMap<AccountAddress, BTreeMap<Identifier, ModuleStorageEntry>>,
    address: &AccountAddress,
    module_name: &IdentStr,
) -> &'a ModuleStorageEntry {
    module_storage
        .get(address)
        .unwrap()
        .get(module_name)
        .unwrap()
}

fn get_module_entry_mut_or_panic<'a>(
    module_storage: &'a mut BTreeMap<AccountAddress, BTreeMap<Identifier, ModuleStorageEntry>>,
    address: &AccountAddress,
    module_name: &IdentStr,
) -> &'a mut ModuleStorageEntry {
    module_storage
        .get_mut(address)
        .unwrap()
        .get_mut(module_name)
        .unwrap()
}

#[cfg(test)]
impl<'e, B: ModuleBytesStorage> UnsyncModuleStorage<'e, B> {
    pub(crate) fn does_not_have_cached_modules(&self) -> bool {
        let module_storage = self.module_storage.borrow();
        module_storage.get(&AccountAddress::ZERO).is_none()
    }

    pub(crate) fn matches<P: Fn(&ModuleStorageEntry) -> bool>(
        &self,
        module_names: impl IntoIterator<Item = &'e str>,
        predicate: P,
    ) -> bool {
        let module_storage = self.module_storage.borrow();
        let module_names_in_storage = assert_some!(module_storage.get(&AccountAddress::ZERO))
            .iter()
            .filter_map(|(name, entry)| predicate(entry).then_some(name.as_str()))
            .collect::<BTreeSet<_>>();
        let module_names = module_names.into_iter().collect::<BTreeSet<_>>();
        module_names.is_subset(&module_names_in_storage)
            && module_names_in_storage.is_subset(&module_names)
    }
}

// TODO(loader_v2): Go over all tests, and consider different corner cases.
#[cfg(test)]
pub(crate) mod test {
    use super::*;
    use claims::{assert_err, assert_none, assert_ok};
    use move_binary_format::{
        file_format::empty_module_with_dependencies_and_friends,
        file_format_common::VERSION_DEFAULT,
    };
    use move_core_types::{ident_str, vm_status::StatusCode};

    fn module<'a>(
        module_name: &'a str,
        dependencies: impl IntoIterator<Item = &'a str>,
        friends: impl IntoIterator<Item = &'a str>,
    ) -> (CompiledModule, Bytes) {
        let mut module =
            empty_module_with_dependencies_and_friends(module_name, dependencies, friends);
        module.version = VERSION_DEFAULT;

        let mut module_bytes = vec![];
        assert_ok!(module.serialize(&mut module_bytes));

        (module, module_bytes.into())
    }

    pub(crate) fn add_module_bytes<'a>(
        module_bytes_storage: &mut LocalModuleBytesStorage,
        module_name: &'a str,
        dependencies: impl IntoIterator<Item = &'a str>,
        friends: impl IntoIterator<Item = &'a str>,
    ) {
        let (module, bytes) = module(module_name, dependencies, friends);
        module_bytes_storage.add_module_bytes(module.self_addr(), module.self_name(), bytes);
    }

    #[test]
    fn test_module_does_not_exist() {
        let env = RuntimeEnvironment::test();
        let module_storage = LocalModuleBytesStorage::empty().into_unsync_module_storage(&env);

        let result = module_storage.check_module_exists(&AccountAddress::ZERO, ident_str!("a"));
        assert!(!assert_ok!(result));

        let result =
            module_storage.fetch_module_size_in_bytes(&AccountAddress::ZERO, ident_str!("a"));
        assert_none!(assert_ok!(result));

        let result = module_storage.fetch_module_metadata(&AccountAddress::ZERO, ident_str!("a"));
        assert_eq!(assert_err!(result).major_status(), StatusCode::LINKER_ERROR);

        let result =
            module_storage.fetch_deserialized_module(&AccountAddress::ZERO, ident_str!("a"));
        assert_eq!(assert_err!(result).major_status(), StatusCode::LINKER_ERROR);

        let result = module_storage.fetch_verified_module(&AccountAddress::ZERO, ident_str!("a"));
        assert_eq!(assert_err!(result).major_status(), StatusCode::LINKER_ERROR);
        assert!(module_storage.does_not_have_cached_modules());
    }

    #[test]
    fn test_module_exists() {
        let mut module_bytes_storage = LocalModuleBytesStorage::empty();
        add_module_bytes(&mut module_bytes_storage, "a", vec![], vec![]);

        let env = RuntimeEnvironment::test();
        let module_storage = module_bytes_storage.into_unsync_module_storage(&env);

        let result = module_storage.check_module_exists(&AccountAddress::ZERO, ident_str!("a"));
        assert!(assert_ok!(result));
        assert!(module_storage.does_not_have_cached_modules());
    }

    #[test]
    fn test_deserialized_caching() {
        use ModuleStorageEntry::*;

        let mut module_bytes_storage = LocalModuleBytesStorage::empty();
        add_module_bytes(&mut module_bytes_storage, "a", vec!["b", "c"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "b", vec![], vec![]);
        add_module_bytes(&mut module_bytes_storage, "c", vec!["d", "e"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "d", vec![], vec![]);
        add_module_bytes(&mut module_bytes_storage, "e", vec![], vec![]);

        let env = RuntimeEnvironment::test();
        let module_storage = module_bytes_storage.into_unsync_module_storage(&env);

        let result = module_storage.fetch_module_metadata(&AccountAddress::ZERO, ident_str!("a"));
        assert_eq!(
            assert_ok!(result),
            module("a", vec!["b", "c"], vec![]).0.metadata
        );

        assert!(module_storage.matches(vec!["a"], |e| { matches!(e, Deserialized(..)) }));
        assert!(module_storage.matches(vec![], |e| matches!(e, Verified(..))));

        let result =
            module_storage.fetch_deserialized_module(&AccountAddress::ZERO, ident_str!("c"));
        assert_eq!(
            assert_ok!(result).as_ref(),
            &module("c", vec!["d", "e"], vec![]).0
        );

        assert!(module_storage.matches(vec!["a", "c"], |e| { matches!(e, Deserialized(..)) }));
        assert!(module_storage.matches(vec![], |e| matches!(e, Verified(..))));
    }

    #[test]
    fn test_dependency_tree_traversal() {
        use ModuleStorageEntry::*;

        let mut module_bytes_storage = LocalModuleBytesStorage::empty();
        add_module_bytes(&mut module_bytes_storage, "a", vec!["b", "c"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "b", vec![], vec![]);
        add_module_bytes(&mut module_bytes_storage, "c", vec!["d", "e"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "d", vec![], vec![]);
        add_module_bytes(&mut module_bytes_storage, "e", vec![], vec![]);

        let env = RuntimeEnvironment::test();
        let module_storage = module_bytes_storage.into_unsync_module_storage(&env);

        let result = module_storage.fetch_verified_module(&AccountAddress::ZERO, ident_str!("c"));
        assert_ok!(result);
        assert!(module_storage.matches(vec![], |e| matches!(e, Deserialized(..))));
        assert!(module_storage.matches(vec!["c", "d", "e"], |e| { matches!(e, Verified(..)) }));

        let result = module_storage.fetch_verified_module(&AccountAddress::ZERO, ident_str!("a"));
        assert_ok!(result);
        assert!(module_storage.matches(vec!["a", "b", "c", "d", "e"], |e| {
            matches!(e, Verified(..))
        }));

        let result = module_storage.fetch_verified_module(&AccountAddress::ZERO, ident_str!("a"));
        assert_ok!(result);
    }

    #[test]
    fn test_dependency_dag_traversal() {
        use ModuleStorageEntry::*;

        let mut module_bytes_storage = LocalModuleBytesStorage::empty();
        add_module_bytes(&mut module_bytes_storage, "a", vec!["b", "c"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "b", vec!["d"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "c", vec!["d"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "d", vec!["e", "f"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "e", vec!["g"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "f", vec!["g"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "g", vec![], vec![]);

        let env = RuntimeEnvironment::test();
        let module_storage = module_bytes_storage.into_unsync_module_storage(&env);

        assert_ok!(module_storage.fetch_deserialized_module(&AccountAddress::ZERO, ident_str!("a")));
        assert_ok!(module_storage.fetch_deserialized_module(&AccountAddress::ZERO, ident_str!("c")));
        assert!(module_storage.matches(vec!["a", "c"], |e| { matches!(e, Deserialized(..)) }));
        assert!(module_storage.matches(vec![], |e| matches!(e, Verified(..))));

        let result = module_storage.fetch_verified_module(&AccountAddress::ZERO, ident_str!("d"));
        assert_ok!(result);
        assert!(module_storage.matches(vec!["a", "c"], |e| { matches!(e, Deserialized(..)) }));
        assert!(module_storage.matches(vec!["d", "e", "f", "g"], |e| { matches!(e, Verified(..)) }));

        let result = module_storage.fetch_verified_module(&AccountAddress::ZERO, ident_str!("a"));
        assert_ok!(result);
        assert!(module_storage.matches(vec![], |e| matches!(e, Deserialized(..))));
        assert!(
            module_storage.matches(vec!["a", "b", "c", "d", "e", "f", "g"], |e| matches!(
                e,
                Verified(..)
            ),)
        );
    }

    #[test]
    fn test_cyclic_dependencies_traversal_fails() {
        let mut module_bytes_storage = LocalModuleBytesStorage::empty();
        add_module_bytes(&mut module_bytes_storage, "a", vec!["b"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "b", vec!["c"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "c", vec!["a"], vec![]);

        let env = RuntimeEnvironment::test();
        let module_storage = module_bytes_storage.into_unsync_module_storage(&env);

        let result = module_storage.fetch_verified_module(&AccountAddress::ZERO, ident_str!("c"));
        assert_eq!(
            assert_err!(result).major_status(),
            StatusCode::CYCLIC_MODULE_DEPENDENCY
        );
    }

    #[test]
    fn test_cyclic_friends_are_allowed() {
        use ModuleStorageEntry::*;

        let mut module_bytes_storage = LocalModuleBytesStorage::empty();
        add_module_bytes(&mut module_bytes_storage, "a", vec![], vec!["b"]);
        add_module_bytes(&mut module_bytes_storage, "b", vec![], vec!["c"]);
        add_module_bytes(&mut module_bytes_storage, "c", vec![], vec!["a"]);

        let env = RuntimeEnvironment::test();
        let module_storage = module_bytes_storage.into_unsync_module_storage(&env);

        let result = module_storage.fetch_verified_module(&AccountAddress::ZERO, ident_str!("c"));
        assert_ok!(result);

        // Since `c` has no dependencies, only it gets deserialized and verified.
        assert!(module_storage.matches(vec![], |e| matches!(e, Deserialized(..))));
        assert!(module_storage.matches(vec!["c"], |e| matches!(e, Verified(..))));
    }

    #[test]
    fn test_transitive_friends_are_allowed_to_be_transitive_dependencies() {
        use ModuleStorageEntry::*;

        let mut module_bytes_storage = LocalModuleBytesStorage::empty();
        add_module_bytes(&mut module_bytes_storage, "a", vec!["b"], vec!["d"]);
        add_module_bytes(&mut module_bytes_storage, "b", vec!["c"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "c", vec![], vec![]);
        add_module_bytes(&mut module_bytes_storage, "d", vec![], vec!["c"]);

        let env = RuntimeEnvironment::test();
        let module_storage = module_bytes_storage.into_unsync_module_storage(&env);

        let result = module_storage.fetch_verified_module(&AccountAddress::ZERO, ident_str!("a"));
        assert_ok!(result);

        assert!(module_storage.matches(vec![], |e| matches!(e, Deserialized(..))));
        assert!(module_storage.matches(vec!["a", "b", "c"], |e| { matches!(e, Verified(..)) }));
    }
}
