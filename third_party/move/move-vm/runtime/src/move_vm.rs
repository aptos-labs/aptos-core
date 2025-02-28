// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    data_cache::TransactionDataCache,
    loader::{LegacyModuleStorage, LegacyModuleStorageAdapter},
    native_extensions::NativeContextExtensions,
    runtime::VMRuntime,
    session::Session,
    RuntimeEnvironment,
};
use move_core_types::{language_storage::ModuleId, metadata::Metadata};
use move_vm_types::resolver::MoveResolver;

pub struct MoveVM {
    pub(crate) runtime: VMRuntime,
}

impl MoveVM {
    /// Creates a new VM instance for the given [RuntimeEnvironment].
    pub fn new_with_runtime_environment(runtime_environment: &RuntimeEnvironment) -> Self {
        Self {
            runtime: VMRuntime::new(runtime_environment),
        }
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
    pub fn new_session<'r>(&self, remote: &'r impl MoveResolver) -> Session<'r, '_> {
        self.new_session_with_extensions(remote, NativeContextExtensions::default())
    }

    /// Create a new session, as in `new_session`, but provide native context extensions.
    pub fn new_session_with_extensions<'r>(
        &self,
        remote: &'r impl MoveResolver,
        native_extensions: NativeContextExtensions<'r>,
    ) -> Session<'r, '_> {
        Session {
            move_vm: self,
            data_cache: TransactionDataCache::new(
                self.runtime
                    .loader()
                    .vm_config()
                    .deserializer_config
                    .clone(),
                remote,
            ),
            module_store: LegacyModuleStorageAdapter::new(self.runtime.module_storage_v1()),
            native_extensions,
        }
    }

    /// Allows the adapter to announce to the VM that the code loading cache should be considered
    /// outdated. This can happen if the adapter executed a particular code publishing transaction
    /// but decided to not commit the result to the data store. Because the code cache currently
    /// does not support deletion, the cache will, incorrectly, still contain this module.
    #[deprecated]
    pub fn mark_loader_cache_as_invalid(&self) {
        #[allow(deprecated)]
        self.runtime.loader().mark_v1_as_invalid()
    }

    /// Returns true if the loader cache has been invalidated (either by explicit call above
    /// or by the runtime)
    #[deprecated]
    pub fn is_loader_cache_invalidated(&self) -> bool {
        #[allow(deprecated)]
        self.runtime.loader().is_v1_invalidated()
    }

    /// If the loader cache has been invalidated (either by the above call or by internal logic)
    /// flush it so it is valid again. Notice that should only be called if there are no
    /// outstanding sessions created from this VM.
    #[deprecated]
    pub fn flush_loader_cache_if_invalidated(&self) {
        // Flush the module cache inside the VMRuntime. This code is there for a legacy reason:
        // - In the old session api that we provide, MoveVM will hold a cache for loaded module and the session will be created against that cache.
        //   Thus if an module invalidation event happens (e.g, by upgrade request), we will need to flush this internal cache as well.
        // - If we can deprecate this session api, we will be able to get rid of this internal loaded cache and make the MoveVM "stateless" and
        //   invulnerable to module invalidation.
        #[allow(deprecated)]
        if self.runtime.loader().is_v1_invalidated() {
            self.runtime.module_cache.flush();
        };
        #[allow(deprecated)]
        self.runtime.loader().flush_v1_if_invalidated()
    }

    /// DO NOT USE THIS API!
    ///
    /// Currently, metadata is owned by module which is owned by the VM. In the new loader
    /// V2 design, clients can fetch metadata and apply this function directly!
    #[deprecated]
    pub fn with_module_metadata<T, F>(&self, module: &ModuleId, f: F) -> Option<T>
    where
        F: FnOnce(&[Metadata]) -> Option<T>,
    {
        f(&self.runtime.module_cache.fetch_module(module)?.metadata)
    }
}
