// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    loader::Module,
    storage::{
        environment::{
            ambassador_impl_WithRuntimeEnvironment, RuntimeEnvironment, WithRuntimeEnvironment,
        },
        module_storage::{ambassador_impl_ModuleStorage, ModuleStorage},
    },
};
use ambassador::Delegate;
use bytes::Bytes;
use move_binary_format::{errors::VMResult, CompiledModule};
use move_core_types::{
    account_address::AccountAddress, identifier::IdentStr, language_storage::ModuleId,
    metadata::Metadata,
};
use move_vm_types::{
    code::{
        ambassador_impl_ModuleCache, ModuleBytesStorage, ModuleCache, ModuleCode,
        ModuleCodeBuilder, UnsyncModuleCache, WithBytes, WithHash,
    },
    sha3_256,
};
use std::{borrow::Borrow, ops::Deref, sync::Arc};

/// Represents owned or borrowed types, similar to [std::borrow::Cow] but without enforcing
/// [ToOwned] trait bound on types it stores. We use it to be able to construct different storages
/// that capture or borrow underlying byte storage.
pub enum BorrowedOrOwned<'a, T> {
    Borrowed(&'a T),
    Owned(T),
}

impl<T> Deref for BorrowedOrOwned<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match *self {
            Self::Borrowed(x) => x,
            Self::Owned(ref x) => x.borrow(),
        }
    }
}

/// Extension for modules stored in [UnsyncModuleStorage] to also capture information about bytes
/// and hash.
struct BytesWithHash {
    /// Bytes of the module.
    bytes: Bytes,
    /// Hash of the module.
    hash: [u8; 32],
}

impl BytesWithHash {
    /// Returns new extension containing bytes and hash.
    pub fn new(bytes: Bytes, hash: [u8; 32]) -> Self {
        Self { bytes, hash }
    }
}

impl WithBytes for BytesWithHash {
    fn bytes(&self) -> &Bytes {
        &self.bytes
    }
}

impl WithHash for BytesWithHash {
    fn hash(&self) -> &[u8; 32] {
        &self.hash
    }
}

/// Placeholder for module versioning since we do not allow to mutate [UnsyncModuleStorage].
#[derive(Clone, Default, Eq, PartialEq, Ord, PartialOrd)]
struct NoVersion;

/// Private implementation of module storage based on non-[Sync] module cache and the baseline
/// storage.
#[allow(clippy::duplicated_attributes)]
#[derive(Delegate)]
#[delegate(
    WithRuntimeEnvironment,
    target = "runtime_environment",
    where = "S: ModuleBytesStorage, E: WithRuntimeEnvironment"
)]
#[delegate(
    ModuleCache,
    target = "module_cache",
    where = "S: ModuleBytesStorage, E: WithRuntimeEnvironment"
)]
struct UnsyncModuleStorageImpl<'s, S, E> {
    /// Environment where this module storage is defined in.
    runtime_environment: E,
    /// Module cache with deserialized or verified modules.
    module_cache: UnsyncModuleCache<ModuleId, CompiledModule, Module, BytesWithHash, NoVersion>,

    /// Immutable baseline storage from which one can fetch raw module bytes.
    base_storage: BorrowedOrOwned<'s, S>,
}

impl<'s, S: ModuleBytesStorage, E: WithRuntimeEnvironment> UnsyncModuleStorageImpl<'s, S, E> {
    /// Private constructor from borrowed byte storage. Creates empty module storage cache.
    fn from_borrowed(runtime_environment: E, storage: &'s S) -> Self {
        Self {
            runtime_environment,
            module_cache: UnsyncModuleCache::empty(),
            base_storage: BorrowedOrOwned::Borrowed(storage),
        }
    }

    /// Private constructor that captures provided byte storage by value. Creates empty module
    /// storage cache.
    fn from_owned(runtime_environment: E, storage: S) -> Self {
        Self {
            runtime_environment,
            module_cache: UnsyncModuleCache::empty(),
            base_storage: BorrowedOrOwned::Owned(storage),
        }
    }
}

impl<S: ModuleBytesStorage, E: WithRuntimeEnvironment> ModuleCodeBuilder
    for UnsyncModuleStorageImpl<'_, S, E>
{
    type Deserialized = CompiledModule;
    type Extension = BytesWithHash;
    type Key = ModuleId;
    type Verified = Module;

    fn build(
        &self,
        key: &Self::Key,
    ) -> VMResult<Option<ModuleCode<Self::Deserialized, Self::Verified, Self::Extension>>> {
        let bytes = match self
            .base_storage
            .fetch_module_bytes(key.address(), key.name())?
        {
            Some(bytes) => bytes,
            None => return Ok(None),
        };
        let compiled_module = self
            .runtime_environment()
            .deserialize_into_compiled_module(&bytes)?;
        let hash = sha3_256(&bytes);
        let extension = Arc::new(BytesWithHash::new(bytes, hash));
        let module = ModuleCode::from_deserialized(compiled_module, extension);
        Ok(Some(module))
    }
}

/// Implementation of (not thread-safe) module storage used for Move unit tests, and externally.
#[allow(clippy::duplicated_attributes)]
#[derive(Delegate)]
#[delegate(
    WithRuntimeEnvironment,
    where = "S: ModuleBytesStorage, E: WithRuntimeEnvironment"
)]
#[delegate(
    ModuleStorage,
    where = "S: ModuleBytesStorage, E: WithRuntimeEnvironment"
)]
pub struct UnsyncModuleStorage<'s, S, E>(UnsyncModuleStorageImpl<'s, S, E>);

impl<'s, S: ModuleBytesStorage, E: WithRuntimeEnvironment> UnsyncModuleStorage<'s, S, E> {
    /// The reference to the baseline byte storage used by this module storage.
    pub fn byte_storage(&self) -> &S {
        &self.0.base_storage
    }

    /// Returns an iterator of all modules that have been cached and verified.
    pub fn unpack_into_verified_modules_iter(
        self,
    ) -> (
        BorrowedOrOwned<'s, S>,
        impl Iterator<Item = (ModuleId, Arc<Module>)>,
    ) {
        let verified_modules_iter =
            self.0
                .module_cache
                .into_modules_iter()
                .flat_map(|(key, module)| {
                    module.code().is_verified().then(|| {
                        // TODO(loader_v2):
                        //   We should be able to take ownership here, instead of clones.
                        (key, module.code().verified().clone())
                    })
                });
        (self.0.base_storage, verified_modules_iter)
    }

    /// Test-only method that checks the state of the module cache.
    #[cfg(test)]
    pub(crate) fn assert_cached_state<'b>(
        &self,
        deserialized: Vec<&'b ModuleId>,
        verified: Vec<&'b ModuleId>,
    ) {
        use claims::*;

        assert_eq!(self.0.num_modules(), deserialized.len() + verified.len());
        for id in deserialized {
            let result = self.0.get_module_or_build_with(id, &self.0);
            let module = assert_some!(assert_ok!(result)).0;
            assert!(!module.code().is_verified())
        }
        for id in verified {
            let result = self.0.get_module_or_build_with(id, &self.0);
            let module = assert_some!(assert_ok!(result)).0;
            assert!(module.code().is_verified())
        }
    }
}

pub trait AsUnsyncModuleStorage<'s, S, E> {
    fn as_unsync_module_storage(&'s self, runtime_environment: E) -> UnsyncModuleStorage<'s, S, E>;

    fn into_unsync_module_storage(self, runtime_environment: E) -> UnsyncModuleStorage<'s, S, E>;
}

impl<'s, S: ModuleBytesStorage, E: WithRuntimeEnvironment> AsUnsyncModuleStorage<'s, S, E> for S {
    fn as_unsync_module_storage(&'s self, runtime_environment: E) -> UnsyncModuleStorage<'s, S, E> {
        UnsyncModuleStorage(UnsyncModuleStorageImpl::from_borrowed(
            runtime_environment,
            self,
        ))
    }

    fn into_unsync_module_storage(self, runtime_environment: E) -> UnsyncModuleStorage<'s, S, E> {
        UnsyncModuleStorage(UnsyncModuleStorageImpl::from_owned(
            runtime_environment,
            self,
        ))
    }
}

#[cfg(test)]
pub(crate) mod test {
    use super::*;
    use crate::storage::module_storage::ModuleStorage;
    use claims::{assert_err, assert_none, assert_ok, assert_some};
    use move_binary_format::{
        file_format::empty_module_with_dependencies_and_friends,
        file_format_common::VERSION_DEFAULT,
    };
    use move_core_types::{
        account_address::AccountAddress, ident_str, identifier::Identifier, vm_status::StatusCode,
    };
    use move_vm_test_utils::InMemoryStorage;

    fn make_module<'a>(
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
        let (module, bytes) = make_module(module_name, dependencies, friends);
        module_bytes_storage.add_module_bytes(module.self_addr(), module.self_name(), bytes);
    }

    #[test]
    fn test_module_does_not_exist() {
        let runtime_environment = RuntimeEnvironment::new(vec![]);
        let module_storage = InMemoryStorage::new().into_unsync_module_storage(runtime_environment);

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
        let id = ModuleId::new(AccountAddress::ZERO, Identifier::new("a").unwrap());

        let runtime_environment = RuntimeEnvironment::new(vec![]);
        let module_storage = module_bytes_storage.into_unsync_module_storage(runtime_environment);

        assert!(assert_ok!(
            module_storage.check_module_exists(id.address(), id.name())
        ));
        module_storage.assert_cached_state(vec![&id], vec![]);
    }

    #[test]
    fn test_deserialized_caching() {
        let mut module_bytes_storage = InMemoryStorage::new();

        let a_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("a").unwrap());
        let c_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("c").unwrap());

        add_module_bytes(&mut module_bytes_storage, "a", vec!["b", "c"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "b", vec![], vec![]);
        add_module_bytes(&mut module_bytes_storage, "c", vec!["d", "e"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "d", vec![], vec![]);
        add_module_bytes(&mut module_bytes_storage, "e", vec![], vec![]);

        let runtime_environment = RuntimeEnvironment::new(vec![]);
        let module_storage = module_bytes_storage.into_unsync_module_storage(runtime_environment);

        let result = module_storage.fetch_module_metadata(a_id.address(), a_id.name());
        let expected = make_module("a", vec!["b", "c"], vec![]).0.metadata;
        assert_eq!(assert_some!(assert_ok!(result)), expected);
        module_storage.assert_cached_state(vec![&a_id], vec![]);

        let result = module_storage.fetch_deserialized_module(c_id.address(), c_id.name());
        let expected = make_module("c", vec!["d", "e"], vec![]).0;
        assert_eq!(assert_some!(assert_ok!(result)).as_ref(), &expected);
        module_storage.assert_cached_state(vec![&a_id, &c_id], vec![]);
    }

    #[test]
    fn test_dependency_tree_traversal() {
        let mut module_bytes_storage = InMemoryStorage::new();

        let a_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("a").unwrap());
        let b_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("b").unwrap());
        let c_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("c").unwrap());
        let d_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("d").unwrap());
        let e_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("e").unwrap());

        add_module_bytes(&mut module_bytes_storage, "a", vec!["b", "c"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "b", vec![], vec![]);
        add_module_bytes(&mut module_bytes_storage, "c", vec!["d", "e"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "d", vec![], vec![]);
        add_module_bytes(&mut module_bytes_storage, "e", vec![], vec![]);

        let runtime_environment = RuntimeEnvironment::new(vec![]);
        let module_storage = module_bytes_storage.into_unsync_module_storage(runtime_environment);

        assert_ok!(module_storage.fetch_verified_module(c_id.address(), c_id.name()));
        module_storage.assert_cached_state(vec![], vec![&c_id, &d_id, &e_id]);

        assert_ok!(module_storage.fetch_verified_module(a_id.address(), a_id.name()));
        module_storage.assert_cached_state(vec![], vec![&a_id, &b_id, &c_id, &d_id, &e_id]);

        assert_ok!(module_storage.fetch_verified_module(a_id.address(), a_id.name()));
    }

    #[test]
    fn test_dependency_dag_traversal() {
        let mut module_bytes_storage = InMemoryStorage::new();

        let a_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("a").unwrap());
        let b_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("b").unwrap());
        let c_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("c").unwrap());
        let d_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("d").unwrap());
        let e_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("e").unwrap());
        let f_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("f").unwrap());
        let g_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("g").unwrap());

        add_module_bytes(&mut module_bytes_storage, "a", vec!["b", "c"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "b", vec!["d"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "c", vec!["d"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "d", vec!["e", "f"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "e", vec!["g"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "f", vec!["g"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "g", vec![], vec![]);

        let runtime_environment = RuntimeEnvironment::new(vec![]);
        let module_storage = module_bytes_storage.into_unsync_module_storage(runtime_environment);

        assert_ok!(module_storage.fetch_deserialized_module(a_id.address(), a_id.name()));
        assert_ok!(module_storage.fetch_deserialized_module(c_id.address(), c_id.name()));
        module_storage.assert_cached_state(vec![&a_id, &c_id], vec![]);

        assert_ok!(module_storage.fetch_verified_module(d_id.address(), d_id.name()));
        module_storage.assert_cached_state(vec![&a_id, &c_id], vec![&d_id, &e_id, &f_id, &g_id]);

        assert_ok!(module_storage.fetch_verified_module(a_id.address(), a_id.name()));
        module_storage.assert_cached_state(vec![], vec![
            &a_id, &b_id, &c_id, &d_id, &e_id, &f_id, &g_id,
        ]);
    }

    #[test]
    fn test_cyclic_dependencies_traversal_fails() {
        let mut module_bytes_storage = InMemoryStorage::new();

        let c_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("c").unwrap());

        add_module_bytes(&mut module_bytes_storage, "a", vec!["b"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "b", vec!["c"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "c", vec!["a"], vec![]);

        let runtime_environment = RuntimeEnvironment::new(vec![]);
        let module_storage = module_bytes_storage.into_unsync_module_storage(runtime_environment);

        let result = module_storage.fetch_verified_module(c_id.address(), c_id.name());
        assert_eq!(
            assert_err!(result).major_status(),
            StatusCode::CYCLIC_MODULE_DEPENDENCY
        );
    }

    #[test]
    fn test_cyclic_friends_are_allowed() {
        let mut module_bytes_storage = InMemoryStorage::new();

        let c_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("c").unwrap());

        add_module_bytes(&mut module_bytes_storage, "a", vec![], vec!["b"]);
        add_module_bytes(&mut module_bytes_storage, "b", vec![], vec!["c"]);
        add_module_bytes(&mut module_bytes_storage, "c", vec![], vec!["a"]);

        let runtime_environment = RuntimeEnvironment::new(vec![]);
        let module_storage = module_bytes_storage.into_unsync_module_storage(runtime_environment);

        let result = module_storage.fetch_verified_module(c_id.address(), c_id.name());
        assert_ok!(result);

        // Since `c` has no dependencies, only it gets deserialized and verified.
        module_storage.assert_cached_state(vec![], vec![&c_id]);
    }

    #[test]
    fn test_transitive_friends_are_allowed_to_be_transitive_dependencies() {
        let mut module_bytes_storage = InMemoryStorage::new();

        let a_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("a").unwrap());
        let b_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("b").unwrap());
        let c_id = ModuleId::new(AccountAddress::ZERO, Identifier::new("c").unwrap());

        add_module_bytes(&mut module_bytes_storage, "a", vec!["b"], vec!["d"]);
        add_module_bytes(&mut module_bytes_storage, "b", vec!["c"], vec![]);
        add_module_bytes(&mut module_bytes_storage, "c", vec![], vec![]);
        add_module_bytes(&mut module_bytes_storage, "d", vec![], vec!["c"]);

        let runtime_environment = RuntimeEnvironment::new(vec![]);
        let module_storage = module_bytes_storage.into_unsync_module_storage(runtime_environment);

        assert_ok!(module_storage.fetch_verified_module(a_id.address(), a_id.name()));
        module_storage.assert_cached_state(vec![], vec![&a_id, &b_id, &c_id]);
    }
}
