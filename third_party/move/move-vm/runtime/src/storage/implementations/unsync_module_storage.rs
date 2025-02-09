// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    loader::Module,
    storage::environment::{RuntimeEnvironment, WithRuntimeEnvironment},
    ModuleStorage,
};
use bytes::Bytes;
use move_binary_format::{errors::VMResult, CompiledModule};
use move_core_types::{
    account_address::AccountAddress, identifier::IdentStr, language_storage::ModuleId,
    metadata::Metadata,
};
use move_vm_types::{
    code::{
        ModuleBytesStorage, ModuleCache, ModuleCode, ModuleCodeBuilder, UnsyncModuleCache,
        WithBytes, WithHash,
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
struct UnsyncModuleStorageImpl<'ctx, Ctx> {
    /// Module cache with deserialized or verified modules.
    module_cache: UnsyncModuleCache<ModuleId, CompiledModule, Module, BytesWithHash, NoVersion>,
    /// External context with data and configs.
    ctx: BorrowedOrOwned<'ctx, Ctx>,
}

impl<'ctx, Ctx> UnsyncModuleStorageImpl<'ctx, Ctx>
where
    Ctx: ModuleBytesStorage + WithRuntimeEnvironment,
{
    /// Private constructor from borrowed context. Creates empty module cache.
    fn from_borrowed(ctx: &'ctx Ctx) -> Self {
        Self {
            module_cache: UnsyncModuleCache::empty(),
            ctx: BorrowedOrOwned::Borrowed(ctx),
        }
    }

    /// Private constructor that captures the context by value. Creates empty module cache.
    fn from_owned(ctx: Ctx) -> Self {
        Self {
            module_cache: UnsyncModuleCache::empty(),
            ctx: BorrowedOrOwned::Owned(ctx),
        }
    }
}

impl<'ctx, Ctx> WithRuntimeEnvironment for UnsyncModuleStorageImpl<'ctx, Ctx>
where
    Ctx: ModuleBytesStorage + WithRuntimeEnvironment,
{
    fn runtime_environment(&self) -> &RuntimeEnvironment {
        self.ctx.runtime_environment()
    }
}

impl<'ctx, Ctx> ModuleCache for UnsyncModuleStorageImpl<'ctx, Ctx>
where
    Ctx: ModuleBytesStorage + WithRuntimeEnvironment,
{
    type Deserialized = CompiledModule;
    type Extension = BytesWithHash;
    type Key = ModuleId;
    type Verified = Module;
    type Version = NoVersion;

    fn insert_deserialized_module(
        &self,
        key: Self::Key,
        deserialized_code: Self::Deserialized,
        extension: Arc<Self::Extension>,
        version: Self::Version,
    ) -> VMResult<Arc<ModuleCode<Self::Deserialized, Self::Verified, Self::Extension>>> {
        self.module_cache
            .insert_deserialized_module(key, deserialized_code, extension, version)
    }

    fn insert_verified_module(
        &self,
        key: Self::Key,
        verified_code: Self::Verified,
        extension: Arc<Self::Extension>,
        version: Self::Version,
    ) -> VMResult<Arc<ModuleCode<Self::Deserialized, Self::Verified, Self::Extension>>> {
        self.module_cache
            .insert_verified_module(key, verified_code, extension, version)
    }

    fn get_module_or_build_with(
        &self,
        key: &Self::Key,
        builder: &dyn ModuleCodeBuilder<
            Key = Self::Key,
            Deserialized = Self::Deserialized,
            Verified = Self::Verified,
            Extension = Self::Extension,
        >,
    ) -> VMResult<
        Option<(
            Arc<ModuleCode<Self::Deserialized, Self::Verified, Self::Extension>>,
            Self::Version,
        )>,
    > {
        self.module_cache.get_module_or_build_with(key, builder)
    }

    fn num_modules(&self) -> usize {
        self.module_cache.num_modules()
    }
}

impl<'ctx, Ctx> ModuleCodeBuilder for UnsyncModuleStorageImpl<'ctx, Ctx>
where
    Ctx: ModuleBytesStorage + WithRuntimeEnvironment,
{
    type Deserialized = CompiledModule;
    type Extension = BytesWithHash;
    type Key = ModuleId;
    type Verified = Module;

    fn build(
        &self,
        key: &Self::Key,
    ) -> VMResult<Option<ModuleCode<Self::Deserialized, Self::Verified, Self::Extension>>> {
        let bytes = match self.ctx.fetch_module_bytes(key.address(), key.name())? {
            Some(bytes) => bytes,
            None => return Ok(None),
        };
        let compiled_module = self
            .ctx
            .runtime_environment()
            .deserialize_into_compiled_module(&bytes)?;
        let hash = sha3_256(&bytes);
        let extension = Arc::new(BytesWithHash::new(bytes, hash));
        let module = ModuleCode::from_deserialized(compiled_module, extension);
        Ok(Some(module))
    }
}

/// Implementation of (not thread-safe) module storage used for Move unit tests, and externally.
pub struct UnsyncModuleStorage<'ctx, Ctx>(UnsyncModuleStorageImpl<'ctx, Ctx>);

impl<'ctx, Ctx> ModuleStorage for UnsyncModuleStorage<'ctx, Ctx>
where
    Ctx: ModuleBytesStorage + WithRuntimeEnvironment,
{
    fn check_module_exists(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<bool> {
        self.0.check_module_exists(address, module_name)
    }

    fn fetch_module_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Bytes>> {
        self.0.fetch_module_bytes(address, module_name)
    }

    fn fetch_module_size_in_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<usize>> {
        self.0.fetch_module_size_in_bytes(address, module_name)
    }

    fn fetch_module_metadata(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Vec<Metadata>>> {
        self.0.fetch_module_metadata(address, module_name)
    }

    fn fetch_deserialized_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Arc<CompiledModule>>> {
        self.0.fetch_deserialized_module(address, module_name)
    }

    fn fetch_verified_module(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Arc<Module>>> {
        self.0.fetch_verified_module(address, module_name)
    }
}

impl<'ctx, Ctx> WithRuntimeEnvironment for UnsyncModuleStorage<'ctx, Ctx>
where
    Ctx: ModuleBytesStorage + WithRuntimeEnvironment,
{
    fn runtime_environment(&self) -> &RuntimeEnvironment {
        self.0.runtime_environment()
    }
}

impl<'ctx, Ctx> UnsyncModuleStorage<'ctx, Ctx>
where
    Ctx: ModuleBytesStorage + WithRuntimeEnvironment,
{
    /// The reference to the baseline byte storage used by this module storage.
    pub fn byte_storage(&self) -> &Ctx {
        &self.0.ctx
    }

    /// Returns an iterator of all modules that have been cached and verified.
    pub fn unpack_into_verified_modules_iter(
        self,
    ) -> (
        BorrowedOrOwned<'ctx, Ctx>,
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
        (self.0.ctx, verified_modules_iter)
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

pub trait AsUnsyncModuleStorage<'ctx, Ctx> {
    fn as_unsync_module_storage(&'ctx self) -> UnsyncModuleStorage<'ctx, Ctx>;

    fn into_unsync_module_storage(self) -> UnsyncModuleStorage<'ctx, Ctx>;
}

impl<'ctx, Ctx> AsUnsyncModuleStorage<'ctx, Ctx> for Ctx
where
    Ctx: ModuleBytesStorage + WithRuntimeEnvironment,
{
    fn as_unsync_module_storage(&'ctx self) -> UnsyncModuleStorage<'ctx, Ctx> {
        UnsyncModuleStorage(UnsyncModuleStorageImpl::from_borrowed(self))
    }

    fn into_unsync_module_storage(self) -> UnsyncModuleStorage<'ctx, Ctx> {
        UnsyncModuleStorage(UnsyncModuleStorageImpl::from_owned(self))
    }
}
