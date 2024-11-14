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

impl<'a, T> Deref for BorrowedOrOwned<'a, T> {
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
#[derive(Delegate)]
#[delegate(
    ModuleCache,
    target = "module_cache",
    where = "S: ModuleBytesStorage + WithRuntimeEnvironment"
)]
struct UnsyncModuleStorageImpl<'s, S> {
    /// Module cache with deserialized or verified modules.
    module_cache: UnsyncModuleCache<ModuleId, CompiledModule, Module, BytesWithHash, NoVersion>,

    /// Immutable baseline storage from which one can fetch raw module bytes.
    base_storage: BorrowedOrOwned<'s, S>,
}

impl<'s, S> WithRuntimeEnvironment for UnsyncModuleStorageImpl<'s, S>
where
    S: ModuleBytesStorage + WithRuntimeEnvironment,
{
    fn runtime_environment(&self) -> &RuntimeEnvironment {
        self.base_storage.runtime_environment()
    }
}

impl<'s, S: ModuleBytesStorage + WithRuntimeEnvironment> UnsyncModuleStorageImpl<'s, S> {
    /// Private constructor from borrowed byte storage. Creates empty module storage cache.
    fn from_borrowed(storage: &'s S) -> Self {
        Self {
            module_cache: UnsyncModuleCache::empty(),
            base_storage: BorrowedOrOwned::Borrowed(storage),
        }
    }

    /// Private constructor that captures provided byte storage by value. Creates empty module
    /// storage cache.
    fn from_owned(storage: S) -> Self {
        Self {
            module_cache: UnsyncModuleCache::empty(),
            base_storage: BorrowedOrOwned::Owned(storage),
        }
    }
}

impl<'s, S: ModuleBytesStorage + WithRuntimeEnvironment> ModuleCodeBuilder
    for UnsyncModuleStorageImpl<'s, S>
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
            .base_storage
            .runtime_environment()
            .deserialize_into_compiled_module(&bytes)?;
        let hash = sha3_256(&bytes);
        let extension = Arc::new(BytesWithHash::new(bytes, hash));
        let module = ModuleCode::from_deserialized(compiled_module, extension);
        Ok(Some(module))
    }
}

/// Implementation of (not thread-safe) module storage used for Move unit tests, and externally.
#[derive(Delegate)]
#[delegate(
    WithRuntimeEnvironment,
    where = "S: ModuleBytesStorage + WithRuntimeEnvironment"
)]
#[delegate(
    ModuleStorage,
    where = "S: ModuleBytesStorage + WithRuntimeEnvironment"
)]
pub struct UnsyncModuleStorage<'s, S>(UnsyncModuleStorageImpl<'s, S>);

impl<'s, S: ModuleBytesStorage + WithRuntimeEnvironment> UnsyncModuleStorage<'s, S> {
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
    #[cfg(any(test, feature = "testing"))]
    pub fn assert_cached_state<'b>(
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

pub trait AsUnsyncModuleStorage<'s, S> {
    fn as_unsync_module_storage(&'s self) -> UnsyncModuleStorage<'s, S>;

    fn into_unsync_module_storage(self) -> UnsyncModuleStorage<'s, S>;
}

impl<'s, S: ModuleBytesStorage + WithRuntimeEnvironment> AsUnsyncModuleStorage<'s, S> for S {
    fn as_unsync_module_storage(&'s self) -> UnsyncModuleStorage<'s, S> {
        UnsyncModuleStorage(UnsyncModuleStorageImpl::from_borrowed(self))
    }

    fn into_unsync_module_storage(self) -> UnsyncModuleStorage<'s, S> {
        UnsyncModuleStorage(UnsyncModuleStorageImpl::from_owned(self))
    }
}
