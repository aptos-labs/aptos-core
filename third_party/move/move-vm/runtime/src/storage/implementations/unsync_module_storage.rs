// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    storage::{
        environment::{RuntimeEnvironment, WithRuntimeEnvironment},
        module_storage::ModuleStorage,
    },
    Module,
};
use bytes::Bytes;
#[cfg(test)]
use claims::assert_some;
use move_binary_format::{
    errors::{Location, VMResult},
    CompiledModule,
};
use move_core_types::{
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
    language_storage::ModuleId,
    metadata::Metadata,
};
use move_vm_types::{
    code_storage::ModuleBytesStorage, module_cyclic_dependency_error, module_linker_error,
    panic_error,
};
use std::{
    borrow::Borrow,
    cell::RefCell,
    collections::{btree_map, BTreeMap, BTreeSet},
    ops::Deref,
    sync::Arc,
};

/// An entry in [UnsyncModuleStorage]'s module cache. As modules are accessed, entries can change
/// from deserialized representation into the verified one.
#[derive(Debug, Clone)]
pub(crate) enum ModuleCacheEntry {
    Deserialized {
        module: Arc<CompiledModule>,
        module_size: usize,
        module_hash: [u8; 32],
    },
    Verified {
        module: Arc<Module>,
    },
}

impl ModuleCacheEntry {
    /// Returns the verified module if the entry is verified, and [None] otherwise.
    fn into_verified(self) -> Option<Arc<Module>> {
        match self {
            Self::Deserialized { .. } => None,
            Self::Verified { module } => Some(module),
        }
    }
}

/// Implementation of (not thread-safe) module storage used for Move unit tests, and externally.
pub struct UnsyncModuleStorage<'a, S> {
    /// Environment where this module storage is defined in.
    runtime_environment: &'a RuntimeEnvironment,
    /// Cache with deserialized or verified modules.
    module_cache: RefCell<BTreeMap<AccountAddress, BTreeMap<Identifier, ModuleCacheEntry>>>,

    /// Immutable baseline storage from which one can fetch raw module bytes.
    base_storage: BorrowedOrOwned<'a, S>,
}

pub trait AsUnsyncModuleStorage<'a, S> {
    fn as_unsync_module_storage(
        &'a self,
        env: &'a RuntimeEnvironment,
    ) -> UnsyncModuleStorage<'a, S>;

    fn into_unsync_module_storage(self, env: &'a RuntimeEnvironment) -> UnsyncModuleStorage<'a, S>;
}

impl<'a, S: ModuleBytesStorage> AsUnsyncModuleStorage<'a, S> for S {
    fn as_unsync_module_storage(
        &'a self,
        env: &'a RuntimeEnvironment,
    ) -> UnsyncModuleStorage<'a, S> {
        UnsyncModuleStorage::from_borrowed(env, self)
    }

    fn into_unsync_module_storage(self, env: &'a RuntimeEnvironment) -> UnsyncModuleStorage<'a, S> {
        UnsyncModuleStorage::from_owned(env, self)
    }
}

impl<'a, S: ModuleBytesStorage> UnsyncModuleStorage<'a, S> {
    /// Private constructor from borrowed byte storage. Creates empty module storage cache.
    fn from_borrowed(runtime_environment: &'a RuntimeEnvironment, storage: &'a S) -> Self {
        Self {
            runtime_environment,
            module_cache: RefCell::new(BTreeMap::new()),
            base_storage: BorrowedOrOwned::Borrowed(storage),
        }
    }

    /// Private constructor that captures provided byte storage by value. Creates empty module
    /// storage cache.
    fn from_owned(runtime_environment: &'a RuntimeEnvironment, storage: S) -> Self {
        Self {
            runtime_environment,
            module_cache: RefCell::new(BTreeMap::new()),
            base_storage: BorrowedOrOwned::Owned(storage),
        }
    }

    /// Returns true if the module is cached.
    fn is_module_cached(&self, address: &AccountAddress, module_name: &IdentStr) -> bool {
        let module_storage = self.module_cache.borrow();
        module_storage
            .get(address)
            .is_some_and(|account_module_storage| account_module_storage.contains_key(module_name))
    }

    /// If the module does not exist, returns true, and false otherwise. For modules that exist, if
    /// the module is not yet cached in module cache, fetches it from the baseline storage and
    /// caches as a deserialized entry.
    fn module_does_not_exist(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<bool> {
        use btree_map::Entry::*;
        use ModuleCacheEntry::*;

        if !self.is_module_cached(address, module_name) {
            let bytes = match self.fetch_module_bytes(address, module_name)? {
                Some(bytes) => bytes,
                None => return Ok(true),
            };

            let (module, module_size, module_hash) = self
                .runtime_environment
                .deserialize_into_compiled_module(&bytes)?;
            self.runtime_environment
                .paranoid_check_module_address_and_name(&module, address, module_name)?;

            let mut module_cache = self.module_cache.borrow_mut();
            let account_module_cache = match module_cache.entry(*address) {
                Occupied(entry) => entry.into_mut(),
                Vacant(entry) => entry.insert(BTreeMap::new()),
            };
            account_module_cache.insert(module_name.to_owned(), Deserialized {
                module: Arc::new(module),
                module_size,
                module_hash,
            });
        }
        Ok(false)
    }

    /// Returns the entry in module storage (deserialized or verified) and an error if it does not
    /// exist. This API clones the underlying entry pointers.
    fn fetch_existing_module_cache_entry(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<ModuleCacheEntry> {
        if self.module_does_not_exist(address, module_name)? {
            return Err(module_linker_error!(address, module_name));
        }

        // Module cache must contain a deserialized entry, because the function above puts it there
        //  if it was not cached already.
        let module_cache = self.module_cache.borrow();
        Ok(get_module_entry(&module_cache, address, module_name)?.clone())
    }

    /// Visits the dependencies of the given module. If dependencies form a cycle (which should not
    /// be the case as we check this when modules are added to the module cache), an error is
    /// returned.
    ///
    /// Important: this implementation **does not** load transitive friends. While it is possible
    /// to view friends as `used-by` relation, it cannot be checked fully. For example, consider
    /// the case when we have four modules A, B, C, D and let `X --> Y` be a dependency relation
    /// (Y is a dependency of X) and `X ==> Y ` a friend relation (X declares Y a friend). Then
    /// consider the case `A --> B <== C --> D <== A`. Here, if we opt for `used-by` semantics,
    /// there is a cycle. But it cannot be checked, since, A only sees B and D, and C sees B and D,
    /// but both B and D do not see any dependencies or friends. Hence, A cannot discover C and
    /// vice-versa, making detection of such corner cases only possible if **all existing modules
    /// are checked**, which is clearly infeasible.
    fn fetch_verified_module_and_visit_all_transitive_dependencies(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
        visited: &mut BTreeSet<ModuleId>,
    ) -> VMResult<Arc<Module>> {
        use ModuleCacheEntry::*;

        // Get the module, and in case it is verified, return early.
        let entry = self.fetch_existing_module_cache_entry(address, module_name)?;
        let (module, module_size, module_hash) = match entry {
            Deserialized {
                module,
                module_size,
                module_hash,
            } => (module, module_size, module_hash),
            Verified { module } => return Ok(module),
        };

        // Step 1: verify compiled module locally.
        let locally_verified_module = self.runtime_environment.build_locally_verified_module(
            module,
            module_size,
            &module_hash,
        )?;

        // Step 2: visit all dependencies and collect them for later verification.
        let mut verified_immediate_dependencies = vec![];
        for (addr, name) in locally_verified_module.immediate_dependencies_iter() {
            // Check if the module has been already visited and verified.
            let dep_entry = self.fetch_existing_module_cache_entry(addr, name)?;
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
        let module = Arc::new(
            self.runtime_environment
                .build_verified_module(locally_verified_module, &verified_immediate_dependencies)?,
        );

        // Step 4: update cached representation to fully verified one.
        let mut module_cache = self.module_cache.borrow_mut();
        let entry = get_module_entry_mut(&mut module_cache, address, module_name)?;
        *entry = Verified {
            module: module.clone(),
        };
        Ok(module)
    }

    /// The reference to the baseline byte storage used by this module storage.
    pub fn byte_storage(&self) -> &S {
        &self.base_storage
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
        // Cached modules in module storage are a subset of modules in byte storage, so it is
        // sufficient to check existence based on it.
        Ok(self
            .base_storage
            .fetch_module_bytes(address, module_name)?
            .is_some())
    }

    fn fetch_module_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Bytes>> {
        self.base_storage.fetch_module_bytes(address, module_name)
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
    ) -> VMResult<Option<Vec<Metadata>>> {
        Ok(self
            .fetch_deserialized_module(address, module_name)?
            .map(|module| module.metadata.clone()))
    }

    fn fetch_deserialized_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Arc<CompiledModule>>> {
        use ModuleCacheEntry::*;

        if self.module_does_not_exist(address, module_name)? {
            return Ok(None);
        }

        // At this point module storage contains a deserialized entry, because the function above
        // puts it there if it existed and was not cached already.
        let module_cache = self.module_cache.borrow();
        let entry = get_module_entry(&module_cache, address, module_name)?;

        Ok(Some(match entry {
            Deserialized { module, .. } => module.clone(),
            Verified { module } => module.as_compiled_module(),
        }))
    }

    fn fetch_verified_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Arc<Module>>> {
        if !self.check_module_exists(address, module_name)? {
            return Ok(None);
        }

        let mut visited = BTreeSet::new();
        let module = self.fetch_verified_module_and_visit_all_transitive_dependencies(
            address,
            module_name,
            &mut visited,
        )?;
        Ok(Some(module))
    }
}

fn get_module_entry<'a>(
    module_cache: &'a BTreeMap<AccountAddress, BTreeMap<Identifier, ModuleCacheEntry>>,
    address: &AccountAddress,
    module_name: &IdentStr,
) -> VMResult<&'a ModuleCacheEntry> {
    let unreachable = || {
        let msg = format!("Entry for {}::{} is not cached", address, module_name);
        panic_error!(msg).finish(Location::Undefined)
    };

    module_cache
        .get(address)
        .ok_or_else(unreachable)?
        .get(module_name)
        .ok_or_else(unreachable)
}

fn get_module_entry_mut<'a>(
    module_cache: &'a mut BTreeMap<AccountAddress, BTreeMap<Identifier, ModuleCacheEntry>>,
    address: &AccountAddress,
    module_name: &IdentStr,
) -> VMResult<&'a mut ModuleCacheEntry> {
    let unreachable = || {
        let msg = format!("Entry for {}::{} is not cached", address, module_name);
        panic_error!(msg).finish(Location::Undefined)
    };

    module_cache
        .get_mut(address)
        .ok_or_else(unreachable)?
        .get_mut(module_name)
        .ok_or_else(unreachable)
}

/// Represents owned or borrowed types, similar to [std::borrow::Cow] but without enforcing
/// [ToOwned] trait bound on types it stores. We use it to be able to construct different storages
/// that capture or borrow underlying byte storage.
enum BorrowedOrOwned<'a, T> {
    Borrowed(&'a T),
    Owned(T),
}

impl<'a, T> Deref for BorrowedOrOwned<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match *self {
            Self::Borrowed(x) => x,
            Self::Owned(ref x) => x.borrow(),
        }
    }
}

#[cfg(test)]
impl<'e, B: ModuleBytesStorage> UnsyncModuleStorage<'e, B> {
    pub(crate) fn does_not_have_cached_modules(&self) -> bool {
        let module_cache = self.module_cache.borrow();
        module_cache.get(&AccountAddress::ZERO).is_none()
    }

    pub(crate) fn matches<P: Fn(&ModuleCacheEntry) -> bool>(
        &self,
        module_names: impl IntoIterator<Item = &'e str>,
        predicate: P,
    ) -> bool {
        let module_cache = self.module_cache.borrow();
        let module_names_in_cache = assert_some!(module_cache.get(&AccountAddress::ZERO))
            .iter()
            .filter_map(|(name, entry)| predicate(entry).then_some(name.as_str()))
            .collect::<BTreeSet<_>>();
        let module_names = module_names.into_iter().collect::<BTreeSet<_>>();
        module_names.is_subset(&module_names_in_cache)
            && module_names_in_cache.is_subset(&module_names)
    }
}

#[cfg(test)]
pub(crate) mod test {
    use super::*;
    use claims::{assert_err, assert_none, assert_ok};
    use move_binary_format::{
        file_format::empty_module_with_dependencies_and_friends,
        file_format_common::VERSION_DEFAULT,
    };
    use move_core_types::{ident_str, vm_status::StatusCode};
    use move_vm_test_utils::InMemoryStorage;

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
        module_bytes_storage: &mut InMemoryStorage,
        module_name: &'a str,
        dependencies: impl IntoIterator<Item = &'a str>,
        friends: impl IntoIterator<Item = &'a str>,
    ) {
        let (module, bytes) = module(module_name, dependencies, friends);
        module_bytes_storage.add_module_bytes(module.self_addr(), module.self_name(), bytes);
    }

    #[test]
    fn test_module_does_not_exist() {
        let runtime_environment = RuntimeEnvironment::new(vec![]);
        let module_storage =
            InMemoryStorage::new().into_unsync_module_storage(&runtime_environment);

        let result = module_storage.check_module_exists(&AccountAddress::ZERO, ident_str!("a"));
        assert!(!assert_ok!(result));

        let result =
            module_storage.fetch_module_size_in_bytes(&AccountAddress::ZERO, ident_str!("a"));
        assert_none!(assert_ok!(result));

        let result = module_storage.fetch_module_metadata(&AccountAddress::ZERO, ident_str!("a"));
        assert_none!(assert_ok!(result));

        let result =
            module_storage.fetch_deserialized_module(&AccountAddress::ZERO, ident_str!("a"));
        assert_none!(assert_ok!(result));

        let result = module_storage.fetch_verified_module(&AccountAddress::ZERO, ident_str!("a"));
        assert_none!(assert_ok!(result));
    }

    #[test]
    fn test_module_exists() {
        let mut module_bytes_storage = InMemoryStorage::new();
        add_module_bytes(&mut module_bytes_storage, "a", vec![], vec![]);

        let runtime_environment = RuntimeEnvironment::new(vec![]);
        let module_storage = module_bytes_storage.into_unsync_module_storage(&runtime_environment);

        let result = module_storage.check_module_exists(&AccountAddress::ZERO, ident_str!("a"));
        assert!(assert_ok!(result));
        assert!(module_storage.does_not_have_cached_modules());
    }

    #[test]
    fn test_deserialized_caching() {
        use ModuleCacheEntry::*;

        let mut module_bytes_storage = InMemoryStorage::new();
        add_module_bytes(&mut module_bytes_storage, "a", vec!["b", "c"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "b", vec![], vec![]);
        add_module_bytes(&mut module_bytes_storage, "c", vec!["d", "e"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "d", vec![], vec![]);
        add_module_bytes(&mut module_bytes_storage, "e", vec![], vec![]);

        let runtime_environment = RuntimeEnvironment::new(vec![]);
        let module_storage = module_bytes_storage.into_unsync_module_storage(&runtime_environment);

        let result = module_storage.fetch_module_metadata(&AccountAddress::ZERO, ident_str!("a"));
        assert_eq!(
            assert_some!(assert_ok!(result)),
            module("a", vec!["b", "c"], vec![]).0.metadata
        );

        assert!(module_storage.matches(vec!["a"], |e| { matches!(e, Deserialized { .. }) }));
        assert!(module_storage.matches(vec![], |e| matches!(e, Verified { .. })));

        let result =
            module_storage.fetch_deserialized_module(&AccountAddress::ZERO, ident_str!("c"));
        assert_eq!(
            assert_some!(assert_ok!(result)).as_ref(),
            &module("c", vec!["d", "e"], vec![]).0
        );

        assert!(module_storage.matches(vec!["a", "c"], |e| { matches!(e, Deserialized { .. }) }));
        assert!(module_storage.matches(vec![], |e| matches!(e, Verified { .. })));
    }

    #[test]
    fn test_dependency_tree_traversal() {
        use ModuleCacheEntry::*;

        let mut module_bytes_storage = InMemoryStorage::new();
        add_module_bytes(&mut module_bytes_storage, "a", vec!["b", "c"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "b", vec![], vec![]);
        add_module_bytes(&mut module_bytes_storage, "c", vec!["d", "e"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "d", vec![], vec![]);
        add_module_bytes(&mut module_bytes_storage, "e", vec![], vec![]);

        let runtime_environment = RuntimeEnvironment::new(vec![]);
        let module_storage = module_bytes_storage.into_unsync_module_storage(&runtime_environment);

        let result = module_storage.fetch_verified_module(&AccountAddress::ZERO, ident_str!("c"));
        assert_ok!(result);
        assert!(module_storage.matches(vec![], |e| matches!(e, Deserialized { .. })));
        assert!(module_storage.matches(vec!["c", "d", "e"], |e| { matches!(e, Verified { .. }) }));

        let result = module_storage.fetch_verified_module(&AccountAddress::ZERO, ident_str!("a"));
        assert_ok!(result);
        assert!(module_storage.matches(vec!["a", "b", "c", "d", "e"], |e| {
            matches!(e, Verified { .. })
        }));

        let result = module_storage.fetch_verified_module(&AccountAddress::ZERO, ident_str!("a"));
        assert_ok!(result);
    }

    #[test]
    fn test_dependency_dag_traversal() {
        use ModuleCacheEntry::*;

        let mut module_bytes_storage = InMemoryStorage::new();
        add_module_bytes(&mut module_bytes_storage, "a", vec!["b", "c"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "b", vec!["d"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "c", vec!["d"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "d", vec!["e", "f"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "e", vec!["g"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "f", vec!["g"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "g", vec![], vec![]);

        let runtime_environment = RuntimeEnvironment::new(vec![]);
        let module_storage = module_bytes_storage.into_unsync_module_storage(&runtime_environment);

        assert_ok!(module_storage.fetch_deserialized_module(&AccountAddress::ZERO, ident_str!("a")));
        assert_ok!(module_storage.fetch_deserialized_module(&AccountAddress::ZERO, ident_str!("c")));
        assert!(module_storage.matches(vec!["a", "c"], |e| { matches!(e, Deserialized { .. }) }));
        assert!(module_storage.matches(vec![], |e| matches!(e, Verified { .. })));

        let result = module_storage.fetch_verified_module(&AccountAddress::ZERO, ident_str!("d"));
        assert_ok!(result);
        assert!(module_storage.matches(vec!["a", "c"], |e| { matches!(e, Deserialized { .. }) }));
        assert!(module_storage.matches(vec!["d", "e", "f", "g"], |e| {
            matches!(e, Verified { .. })
        }));

        let result = module_storage.fetch_verified_module(&AccountAddress::ZERO, ident_str!("a"));
        assert_ok!(result);
        assert!(module_storage.matches(vec![], |e| matches!(e, Deserialized { .. })));
        assert!(
            module_storage.matches(vec!["a", "b", "c", "d", "e", "f", "g"], |e| matches!(
                e,
                Verified { .. }
            ),)
        );
    }

    #[test]
    fn test_cyclic_dependencies_traversal_fails() {
        let mut module_bytes_storage = InMemoryStorage::new();
        add_module_bytes(&mut module_bytes_storage, "a", vec!["b"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "b", vec!["c"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "c", vec!["a"], vec![]);

        let runtime_environment = RuntimeEnvironment::new(vec![]);
        let module_storage = module_bytes_storage.into_unsync_module_storage(&runtime_environment);

        let result = module_storage.fetch_verified_module(&AccountAddress::ZERO, ident_str!("c"));
        assert_eq!(
            assert_err!(result).major_status(),
            StatusCode::CYCLIC_MODULE_DEPENDENCY
        );
    }

    #[test]
    fn test_cyclic_friends_are_allowed() {
        use ModuleCacheEntry::*;

        let mut module_bytes_storage = InMemoryStorage::new();
        add_module_bytes(&mut module_bytes_storage, "a", vec![], vec!["b"]);
        add_module_bytes(&mut module_bytes_storage, "b", vec![], vec!["c"]);
        add_module_bytes(&mut module_bytes_storage, "c", vec![], vec!["a"]);

        let runtime_environment = RuntimeEnvironment::new(vec![]);
        let module_storage = module_bytes_storage.into_unsync_module_storage(&runtime_environment);

        let result = module_storage.fetch_verified_module(&AccountAddress::ZERO, ident_str!("c"));
        assert_ok!(result);

        // Since `c` has no dependencies, only it gets deserialized and verified.
        assert!(module_storage.matches(vec![], |e| matches!(e, Deserialized { .. })));
        assert!(module_storage.matches(vec!["c"], |e| matches!(e, Verified { .. })));
    }

    #[test]
    fn test_transitive_friends_are_allowed_to_be_transitive_dependencies() {
        use ModuleCacheEntry::*;

        let mut module_bytes_storage = InMemoryStorage::new();
        add_module_bytes(&mut module_bytes_storage, "a", vec!["b"], vec!["d"]);
        add_module_bytes(&mut module_bytes_storage, "b", vec!["c"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "c", vec![], vec![]);
        add_module_bytes(&mut module_bytes_storage, "d", vec![], vec!["c"]);

        let runtime_environment = RuntimeEnvironment::new(vec![]);
        let module_storage = module_bytes_storage.into_unsync_module_storage(&runtime_environment);

        let result = module_storage.fetch_verified_module(&AccountAddress::ZERO, ident_str!("a"));
        assert_ok!(result);

        assert!(module_storage.matches(vec![], |e| matches!(e, Deserialized { .. })));
        assert!(module_storage.matches(vec!["a", "b", "c"], |e| { matches!(e, Verified { .. }) }));
    }
}
