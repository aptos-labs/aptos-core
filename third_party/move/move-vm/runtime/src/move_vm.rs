// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::VMConfig,
    data_cache::TransactionDataCache,
    loader::{ModuleStorage, ModuleStorageAdapter},
    native_extensions::NativeContextExtensions,
    native_functions::NativeFunction,
    runtime::VMRuntime,
    session::Session,
};
use move_binary_format::{
    errors::{Location, PartialVMError, VMResult},
    CompiledModule,
};
use move_core_types::{
    account_address::AccountAddress, identifier::Identifier, language_storage::ModuleId,
    metadata::Metadata, resolver::MoveResolver,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct MoveVM {
    pub(crate) runtime: VMRuntime,
}

impl MoveVM {
    pub fn new(
        natives: impl IntoIterator<Item = (AccountAddress, Identifier, Identifier, NativeFunction)>,
    ) -> VMResult<Self> {
        Self::new_with_config(natives, VMConfig::default())
    }

    pub fn new_with_config(
        natives: impl IntoIterator<Item = (AccountAddress, Identifier, Identifier, NativeFunction)>,
        vm_config: VMConfig,
    ) -> VMResult<Self> {
        Ok(Self {
            runtime: VMRuntime::new(natives, vm_config)
                .map_err(|err| err.finish(Location::Undefined))?,
        })
    }

    /// Create a new Session backed by the given storage.
    ///
    /// Right now it is the caller's responsibility to ensure cache coherence of the Move VM Loader
    ///   - When a module gets published in a Move VM Session, and then gets used by another
    ///     transaction, it will be loaded into the code cache and stay there even if the resulted
    ///     effects do not get committed back to the storage when the Session ends.
    ///   - As a result, if one wants to have multiple sessions at a time, one needs to make sure
    ///     none of them will try to publish a module. In other words, if there is a module publishing
    ///     Session it must be the only Session existing.
    ///   - In general, a new Move VM needs to be created whenever the storage gets modified by an
    ///     outer environment, or otherwise the states may be out of sync. There are a few exceptional
    ///     cases where this may not be necessary, with the most notable one being the common module
    ///     publishing flow: you can keep using the same Move VM if you publish some modules in a Session
    ///     and apply the effects to the storage when the Session ends.
    pub fn new_session<'r>(
        &self,
        remote: &'r impl MoveResolver<PartialVMError>,
    ) -> Session<'r, '_> {
        self.new_session_with_extensions(remote, NativeContextExtensions::default())
    }

    /// Create a new session, as in `new_session`, but provide native context extensions.
    pub fn new_session_with_extensions<'r>(
        &self,
        remote: &'r impl MoveResolver<PartialVMError>,
        native_extensions: NativeContextExtensions<'r>,
    ) -> Session<'r, '_> {
        Session {
            move_vm: self,
            data_cache: TransactionDataCache::new(remote),
            module_store: ModuleStorageAdapter::new(self.runtime.module_storage()),
            native_extensions,
        }
    }

    /// Create a new session, as in `new_session`, but provide native context extensions and custome storage for resolved modules.
    pub fn new_session_with_extensions_and_modules<'r>(
        &self,
        remote: &'r impl MoveResolver<PartialVMError>,
        module_storage: Arc<dyn ModuleStorage>,
        native_extensions: NativeContextExtensions<'r>,
    ) -> Session<'r, '_> {
        Session {
            move_vm: self,
            data_cache: TransactionDataCache::new(remote),
            module_store: ModuleStorageAdapter::new(module_storage),
            native_extensions,
        }
    }

    /// Load a module into VM's code cache
    pub fn load_module(
        &self,
        module_id: &ModuleId,
        remote: &impl MoveResolver<PartialVMError>,
    ) -> VMResult<Arc<CompiledModule>> {
        self.runtime
            .loader()
            .load_module(
                module_id,
                &TransactionDataCache::new(remote),
                &ModuleStorageAdapter::new(self.runtime.module_storage()),
            )
            .map(|arc_module| arc_module.arc_module())
    }

    /// Allows the adapter to announce to the VM that the code loading cache should be considered
    /// outdated. This can happen if the adapter executed a particular code publishing transaction
    /// but decided to not commit the result to the data store. Because the code cache currently
    /// does not support deletion, the cache will, incorrectly, still contain this module.
    /// TODO: new loader architecture
    pub fn mark_loader_cache_as_invalid(&self) {
        self.runtime.loader().mark_as_invalid()
    }

    /// Returns true if the loader cache has been invalidated (either by explicit call above
    /// or by the runtime)
    pub fn is_loader_cache_invalidated(&self) -> bool {
        self.runtime.loader().is_invalidated()
    }

    /// If the loader cache has been invalidated (either by the above call or by internal logic)
    /// flush it so it is valid again. Notice that should only be called if there are no
    /// outstanding sessions created from this VM.
    pub fn flush_loader_cache_if_invalidated(&self) {
        // Flush the module cache inside the VMRuntime. This code is there for a legacy reason:
        // - In the old session api that we provide, MoveVM will hold a cache for loaded module and the session will be created against that cache.
        //   Thus if an module invalidation event happens (e.g, by upgrade request), we will need to flush this internal cache as well.
        // - If we can deprecate this session api, we will be able to get rid of this internal loaded cache and make the MoveVM "stateless" and
        //   invulnerable to module invalidation.
        if self.runtime.loader().is_invalidated() {
            self.runtime.module_cache.flush();
        };
        self.runtime.loader().flush_if_invalidated()
    }

    /// Attempts to discover metadata in a given module with given key. Availability
    /// of this data may depend on multiple aspects. In general, no hard assumptions of
    /// availability should be made, but typically, one can expect that
    /// the modules which have been involved in the execution of the last session are available.
    ///
    /// This is called by an adapter to extract, for example, debug information out of
    /// the metadata section of the code for post mortem analysis. Notice that because
    /// of ownership of the underlying binary representation of modules hidden behind an rwlock,
    /// this actually has to hand back a copy of the associated metadata, so metadata should
    /// be organized keeping this in mind.
    ///
    /// TODO: in the new loader architecture, as the loader is visible to the adapter, one would
    ///   call this directly via the loader instead of the VM.
    pub fn with_module_metadata<T, F>(&self, module: &ModuleId, f: F) -> Option<T>
    where
        F: FnOnce(&[Metadata]) -> Option<T>,
    {
        f(&self
            .runtime
            .module_cache
            .fetch_module(module)?
            .module()
            .metadata)
    }
}
