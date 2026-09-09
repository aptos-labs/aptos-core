// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Module storage and publishing checks for one transactional test.

use crate::{engine::build_natives, module_provider::InMemoryModuleProvider};
use bytes::Bytes;
use mono_move_core::{GasMeter, VMInternalError};
use mono_move_global_context::GlobalContext;
use mono_move_loader::{Loader, LoadingPolicy, LoweringPolicy, ModuleReadSet};
use move_binary_format::{compatibility::Compatibility, errors::VMError};
use move_core_types::{account_address::AccountAddress, language_storage::ModuleId};
use move_transactional_test_runner::vm_test_harness::create_runtime_environment;
use move_vm_runtime::{config::VMConfig, AsUnsyncModuleStorage, StagingModuleStorage};
use move_vm_test_utils::InMemoryStorage;
use thiserror::Error;

/// Published modules shared by V1's publishing checks and MonoVM's loader.
///
/// V1 stages, links, and checks compatibility against [`InMemoryStorage`].
/// MonoVM's [`InMemoryModuleProvider`] supplies the same module bytes to its
/// loader. [`commit`](Self::commit) updates both stores together.
///
/// Each bundle load uses one execution guard, then releases it and resets
/// the [`GlobalContext`]'s caches and arenas before returning. Subsequent
/// loads therefore read republished modules from storage.
pub struct TransactionalSession {
    ctx: GlobalContext,
    storage: InMemoryStorage,
    module_provider: InMemoryModuleProvider,
}

/// Failure in V1 publishing checks or MonoVM loading.
#[derive(Debug, Error)]
pub enum PublishError {
    /// V1's publishing checks rejected the bundle.
    #[error(transparent)]
    Staging(VMError),
    /// V1 accepted the bundle, but MonoVM failed to load a module.
    /// Loading includes deserialization, verification, and translation, but no lowering.
    #[error("Unable to load module '{module}' into MonoVM. Got error: {error}")]
    MonoLoad {
        module: ModuleId,
        error: VMInternalError,
    },
}

impl TransactionalSession {
    pub fn new(vm_config: &VMConfig) -> Self {
        Self {
            ctx: GlobalContext::with_num_execution_workers(1),
            storage: InMemoryStorage::new_with_runtime_environment(create_runtime_environment(
                vm_config.clone(),
            )),
            module_provider: InMemoryModuleProvider::new(),
        }
    }

    /// V1's view of the published code.
    pub fn storage(&self) -> &InMemoryStorage {
        &self.storage
    }

    /// Publishes `bundle` from `sender` after V1's checks under `compat` and
    /// MonoVM's loading checks. Modules are translated without lowering.
    /// Both stores remain unchanged if either check fails.
    pub fn publish(
        &mut self,
        sender: &AccountAddress,
        compat: Compatibility,
        bundle: Vec<Bytes>,
    ) -> Result<(), PublishError> {
        let verified = StagingModuleStorage::create_with_compat_config(
            sender,
            compat,
            &self.storage.as_unsync_module_storage(),
            bundle,
        )
        .map_err(PublishError::Staging)?
        .release_verified_module_bundle()
        .into_iter()
        .collect::<Vec<_>>();
        self.load_into_mono(&verified)?;
        self.commit(verified);
        Ok(())
    }

    /// Updates both stores with modules that have passed V1's publishing checks.
    pub(crate) fn commit(&mut self, modules: impl IntoIterator<Item = (ModuleId, Bytes)>) {
        for (id, bytes) in modules {
            self.storage
                .add_module_bytes(id.address(), id.name(), bytes.clone());
            self.module_provider
                .add_module_bytes(*id.address(), id.name().to_owned(), bytes);
        }
    }

    /// Deserializes, verifies, and translates each module without lowering.
    /// A temporary provider includes the candidate modules, so failed loads
    /// leave the stored modules unchanged.
    fn load_into_mono(&mut self, modules: &[(ModuleId, Bytes)]) -> Result<(), PublishError> {
        let mut provider = self.module_provider.clone();
        for (id, bytes) in modules {
            provider.add_module_bytes(*id.address(), id.name().to_owned(), bytes.clone());
        }
        let result = {
            let guard = self
                .ctx
                .try_execution_context(0)
                .expect("the session releases its guard after every operation");
            let loader = Loader::new_with_policy(
                &guard,
                &provider,
                LoadingPolicy::Lazy(LoweringPolicy::Lazy),
                build_natives(),
            );
            modules.iter().try_for_each(|(id, _)| {
                let mut read_set = ModuleReadSet::new();
                let mut gas_meter = GasMeter::with_max_budget();
                loader
                    .load_module(
                        &mut read_set,
                        &mut gas_meter,
                        guard.intern_address_name(id.address(), id.name()),
                    )
                    .map(|_| ())
                    .map_err(|error| PublishError::MonoLoad {
                        module: id.clone(),
                        error,
                    })
            })
        };
        self.ctx.maintenance_context().reset_arena_pool();
        result
    }
}
