// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::VMConfig, data_cache::TransactionDataCache, native_extensions::NativeContextExtensions,
    runtime::VMRuntime, session::Session, RuntimeEnvironment,
};
use move_vm_types::resolver::MoveResolver;

pub struct MoveVM {
    pub(crate) runtime: VMRuntime,
}

impl MoveVM {
    /// Creates a new VM instance, using default configurations. Panics if there are duplicated
    /// natives.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let vm_config = VMConfig {
            // Keep the paranoid mode on as we most likely want this for tests.
            paranoid_type_checks: true,
            ..VMConfig::default()
        };
        Self::new_with_config(vm_config)
    }

    /// Creates a new VM instance, with provided VM configurations. Panics if there are duplicated
    /// natives.
    pub fn new_with_config(vm_config: VMConfig) -> Self {
        Self {
            runtime: VMRuntime::new(vm_config),
        }
    }

    pub fn new_with_runtime_environment(runtime_environment: &RuntimeEnvironment) -> Self {
        Self {
            runtime: VMRuntime::new(runtime_environment.vm_config().clone()),
        }
    }

    /// Returns VM configuration used to initialize the VM.
    pub fn vm_config(&self) -> &VMConfig {
        self.runtime.loader().vm_config()
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
            data_cache: TransactionDataCache::new(remote),
            native_extensions,
        }
    }
}
