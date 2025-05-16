// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::duplicated_attributes)]

use crate::{
    loader::{Function, LoadedFunction, Module, Script},
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
use move_binary_format::{
    errors::{PartialVMResult, VMResult},
    file_format::CompiledScript,
    CompiledModule,
};
use move_core_types::{
    account_address::AccountAddress,
    identifier::IdentStr,
    language_storage::{ModuleId, TypeTag},
    metadata::Metadata,
};
use move_vm_types::{
    code::{ambassador_impl_ScriptCache, Code, ModuleBytesStorage, ScriptCache, UnsyncScriptCache},
    loaded_data::runtime_types::{StructType, Type},
};
use std::sync::Arc;

/// Code storage that stores both modules and scripts (not thread-safe).
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
    #[cfg(any(test, feature = "testing"))]
    pub fn assert_cached_state<'b>(
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

pub trait AsUnsyncCodeStorage<'ctx, Ctx: ModuleBytesStorage + WithRuntimeEnvironment> {
    fn as_unsync_code_storage(&'ctx self) -> UnsyncCodeStorage<UnsyncModuleStorage<'ctx, Ctx>>;

    fn into_unsync_code_storage(self) -> UnsyncCodeStorage<UnsyncModuleStorage<'ctx, Ctx>>;
}

impl<'ctx, Ctx> AsUnsyncCodeStorage<'ctx, Ctx> for Ctx
where
    Ctx: ModuleBytesStorage + WithRuntimeEnvironment,
{
    fn as_unsync_code_storage(&'ctx self) -> UnsyncCodeStorage<UnsyncModuleStorage<'ctx, Ctx>> {
        UnsyncCodeStorage(UnsyncCodeStorageImpl::new(self.as_unsync_module_storage()))
    }

    fn into_unsync_code_storage(self) -> UnsyncCodeStorage<UnsyncModuleStorage<'ctx, Ctx>> {
        UnsyncCodeStorage(UnsyncCodeStorageImpl::new(
            self.into_unsync_module_storage(),
        ))
    }
}
