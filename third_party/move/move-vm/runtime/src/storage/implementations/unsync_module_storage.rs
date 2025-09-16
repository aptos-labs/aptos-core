// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::duplicated_attributes)]

use crate::{
    ambassador_impl_ModuleStorage, ambassador_impl_WithRuntimeEnvironment,
    loader::Module,
    storage::environment::{RuntimeEnvironment, WithRuntimeEnvironment},
    LayoutCache, LayoutCacheEntry, LayoutCacheHit, ModuleStorage,
};
use ambassador::Delegate;
use bytes::Bytes;
use move_binary_format::{
    errors::{PartialVMResult, VMResult},
    CompiledModule,
};
use move_core_types::{
    account_address::AccountAddress, identifier::IdentStr, language_storage::ModuleId,
    metadata::Metadata,
};
use move_vm_types::{
    code::{
        ambassador_impl_ModuleCache, ModuleBytesStorage, ModuleCache, ModuleCode,
        ModuleCodeBuilder, UnsyncModuleCache, WithBytes, WithHash,
    },
    loaded_data::struct_name_indexing::StructNameIndex,
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
#[derive(Delegate)]
#[delegate(
    ModuleCache,
    target = "module_cache",
    where = "Ctx: ModuleBytesStorage + WithRuntimeEnvironment"
)]
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

impl<Ctx> WithRuntimeEnvironment for UnsyncModuleStorageImpl<'_, Ctx>
where
    Ctx: ModuleBytesStorage + WithRuntimeEnvironment,
{
    fn runtime_environment(&self) -> &RuntimeEnvironment {
        self.ctx.runtime_environment()
    }
}

impl<Ctx> ModuleCodeBuilder for UnsyncModuleStorageImpl<'_, Ctx>
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

impl<Ctx> LayoutCache for UnsyncModuleStorageImpl<'_, Ctx> {
    fn get_non_generic_struct_layout(&self, _idx: &StructNameIndex) -> Option<LayoutCacheHit> {
        None
    }

    fn store_non_generic_struct_layout(
        &self,
        _idx: &StructNameIndex,
        _entry: LayoutCacheEntry,
    ) -> PartialVMResult<()> {
        Ok(())
    }
}

/// Implementation of (not thread-safe) module storage used for Move unit tests, and externally.
#[derive(Delegate)]
#[delegate(
    WithRuntimeEnvironment,
    where = "Ctx: ModuleBytesStorage + WithRuntimeEnvironment"
)]
#[delegate(
    ModuleStorage,
    where = "Ctx: ModuleBytesStorage + WithRuntimeEnvironment"
)]
pub struct UnsyncModuleStorage<'ctx, Ctx>(UnsyncModuleStorageImpl<'ctx, Ctx>);

impl<Ctx> LayoutCache for UnsyncModuleStorage<'_, Ctx> {
    fn get_non_generic_struct_layout(&self, _idx: &StructNameIndex) -> Option<LayoutCacheHit> {
        None
    }

    fn store_non_generic_struct_layout(
        &self,
        _idx: &StructNameIndex,
        _entry: LayoutCacheEntry,
    ) -> PartialVMResult<()> {
        Ok(())
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
        impl Iterator<Item = (ModuleId, Arc<Module>)> + use<Ctx>,
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
