// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{loader::Module, WithRuntimeEnvironment};
use ambassador::delegatable_trait;
use bytes::Bytes;
use hashbrown::HashSet;
use move_binary_format::{errors::VMResult, CompiledModule};
use move_core_types::{language_storage::ModuleId, metadata::Metadata};
use move_vm_types::{
    code::{ModuleCache, ModuleCode, ModuleCodeBuilder, WithBytes, WithHash, WithSize},
    module_cyclic_dependency_error, module_linker_error,
};
use std::sync::Arc;

/// Represents module storage backend, abstracting away any caching behaviour. The clients can
/// implement their own module storage to pass to the VM to resolve code.
#[delegatable_trait]
pub trait ModuleStorage: WithRuntimeEnvironment {
    /// Returns true if loader V2 implementation is enabled. Will be removed in the future, for now
    /// it is simply a convenient way to check the feature flag if module storage is available.
    // TODO(loader_v2): Remove this when loader V2 is enabled.
    fn is_enabled(&self) -> bool {
        self.runtime_environment().vm_config().use_loader_v2
    }

    /// Returns true if the module exists, and false otherwise. An error is returned if there is a
    /// storage error.
    fn check_module_exists(&self, module_id: &ModuleId) -> VMResult<bool>;

    /// Returns module bytes if module exists, or [None] otherwise. An error is returned if there
    /// is a storage error.
    fn fetch_module_bytes(&self, module_id: &ModuleId) -> VMResult<Option<Bytes>>;

    /// Returns the size of a module in bytes, or [None] otherwise. An error is returned if the
    /// there is a storage error.
    fn fetch_module_size_in_bytes(&self, module_id: &ModuleId) -> VMResult<Option<usize>>;

    /// Returns the metadata in the module, or [None] otherwise. An error is returned if there is
    /// a storage error or the module fails deserialization.
    fn fetch_module_metadata(&self, module_id: &ModuleId) -> VMResult<Option<Vec<Metadata>>>;

    /// Returns the metadata in the module. An error is returned if there is a storage error,
    /// module fails deserialization, or does not exist.
    fn fetch_existing_module_metadata(&self, module_id: &ModuleId) -> VMResult<Vec<Metadata>> {
        self.fetch_module_metadata(module_id)?
            .ok_or_else(|| module_linker_error!(module_id.address(), module_id.name()))
    }

    /// Returns the deserialized module, or [None] otherwise. An error is returned if:
    ///   1. the deserialization fails, or
    ///   2. there is an error from the underlying storage.
    fn fetch_deserialized_module(
        &self,
        module_id: &ModuleId,
    ) -> VMResult<Option<Arc<CompiledModule>>>;

    /// Returns the deserialized module. An error is returned if:
    ///   1. the deserialization fails,
    ///   2. there is an error from the underlying storage,
    ///   3. module does not exist.
    fn fetch_existing_deserialized_module(
        &self,
        module_id: &ModuleId,
    ) -> VMResult<Arc<CompiledModule>> {
        self.fetch_deserialized_module(module_id)?
            .ok_or_else(|| module_linker_error!(module_id.address(), module_id.name()))
    }

    /// Returns the verified module if it exists, or [None] otherwise. The existing module can be
    /// either in a cached state (it is then returned) or newly constructed. The error is returned
    /// if the storage fails to fetch the deserialized module and verify it.
    fn fetch_verified_module(&self, module_id: &ModuleId) -> VMResult<Option<Arc<Module>>>;
}

impl<T, E, V> ModuleStorage for T
where
    T: WithRuntimeEnvironment
        + ModuleCache<
            Key = ModuleId,
            Deserialized = CompiledModule,
            Verified = Module,
            Extension = E,
            Version = V,
        > + ModuleCodeBuilder<
            Key = ModuleId,
            Deserialized = CompiledModule,
            Verified = Module,
            Extension = E,
        >,
    E: WithBytes + WithSize + WithHash,
    V: Clone + Default + Ord,
{
    fn check_module_exists(&self, module_id: &ModuleId) -> VMResult<bool> {
        Ok(self.get_module_or_build_with(module_id, self)?.is_some())
    }

    fn fetch_module_bytes(&self, module_id: &ModuleId) -> VMResult<Option<Bytes>> {
        Ok(self
            .get_module_or_build_with(module_id, self)?
            .map(|(module, _)| module.extension().bytes().clone()))
    }

    fn fetch_module_size_in_bytes(&self, module_id: &ModuleId) -> VMResult<Option<usize>> {
        Ok(self
            .get_module_or_build_with(module_id, self)?
            .map(|(module, _)| module.extension().bytes().len()))
    }

    fn fetch_module_metadata(&self, module_id: &ModuleId) -> VMResult<Option<Vec<Metadata>>> {
        Ok(self
            .get_module_or_build_with(module_id, self)?
            .map(|(module, _)| module.code().deserialized().metadata.clone()))
    }

    fn fetch_deserialized_module(
        &self,
        module_id: &ModuleId,
    ) -> VMResult<Option<Arc<CompiledModule>>> {
        Ok(self
            .get_module_or_build_with(module_id, self)?
            .map(|(module, _)| module.code().deserialized().clone()))
    }

    fn fetch_verified_module(&self, module_id: &ModuleId) -> VMResult<Option<Arc<Module>>> {
        // Look up the verified module in cache, if it is not there, or if the module is not yet
        // verified, we need to load & verify its transitive dependencies.
        let (module, version) = match self.get_module_or_build_with(module_id, self)? {
            Some(module_and_version) => module_and_version,
            None => return Ok(None),
        };

        if module.code().is_verified() {
            return Ok(Some(module.code().verified().clone()));
        }

        let mut visited = HashSet::new();
        visited.insert(module_id.clone());
        Ok(Some(visit_dependencies_and_verify(
            module_id.clone(),
            module,
            version,
            &mut visited,
            self,
        )?))
    }
}

/// Visits the dependencies of the given module. If dependencies form a cycle (which should not be
/// the case as we check this when modules are added to the module cache), an error is returned.
///
/// Note:
///   This implementation **does not** load transitive friends. While it is possible to view
///   friends as `used-by` relation, it cannot be checked fully. For example, consider the case
///   when we have four modules A, B, C, D and let `X --> Y` be a dependency relation (Y is a
///   dependency of X) and `X ==> Y ` a friend relation (X declares Y a friend). Then consider the
///   case `A --> B <== C --> D <== A`. Here, if we opt for `used-by` semantics, there is a cycle.
///   But it cannot be checked, since, A only sees B and D, and C sees B and D, but both B and D do
///   not see any dependencies or friends. Hence, A cannot discover C and vice-versa, making
///   detection of such corner cases only possible if **all existing modules are checked**, which
///   is clearly infeasible.
fn visit_dependencies_and_verify<T, E, V>(
    module_id: ModuleId,
    module: Arc<ModuleCode<CompiledModule, Module, E>>,
    version: V,
    visited: &mut HashSet<ModuleId>,
    module_cache_with_context: &T,
) -> VMResult<Arc<Module>>
where
    T: WithRuntimeEnvironment
        + ModuleCache<
            Key = ModuleId,
            Deserialized = CompiledModule,
            Verified = Module,
            Extension = E,
            Version = V,
        > + ModuleCodeBuilder<
            Key = ModuleId,
            Deserialized = CompiledModule,
            Verified = Module,
            Extension = E,
        >,
    E: WithBytes + WithSize + WithHash,
    V: Clone + Default + Ord,
{
    let runtime_environment = module_cache_with_context.runtime_environment();

    // Step 1: Local verification.
    runtime_environment.paranoid_check_module_address_and_name(
        module.code().deserialized(),
        module_id.address(),
        module_id.name(),
    )?;
    let locally_verified_code = runtime_environment.build_locally_verified_module(
        module.code().deserialized().clone(),
        module.extension().size_in_bytes(),
        module.extension().hash(),
    )?;

    // Step 2: Traverse and collect all verified immediate dependencies so that we can verify
    // non-local properties of the module.
    let mut verified_dependencies = vec![];
    for (addr, name) in locally_verified_code.immediate_dependencies_iter() {
        let dependency_id = ModuleId::new(*addr, name.to_owned());

        let (dependency, dependency_version) = module_cache_with_context
            .get_module_or_build_with(&dependency_id, module_cache_with_context)?
            .ok_or_else(|| module_linker_error!(addr, name))?;

        // Dependency is already verified!
        if dependency.code().is_verified() {
            verified_dependencies.push(dependency.code().verified().clone());
            continue;
        }

        if visited.insert(dependency_id.clone()) {
            // Dependency is not verified, and we have not visited it yet.
            let verified_dependency = visit_dependencies_and_verify(
                dependency_id.clone(),
                dependency,
                dependency_version,
                visited,
                module_cache_with_context,
            )?;
            verified_dependencies.push(verified_dependency);
        } else {
            // We must have found a cycle otherwise.
            return Err(module_cyclic_dependency_error!(
                dependency_id.address(),
                dependency_id.name()
            ));
        }
    }

    let verified_code =
        runtime_environment.build_verified_module(locally_verified_code, &verified_dependencies)?;
    let module = module_cache_with_context.insert_verified_module(
        module_id,
        verified_code,
        module.extension().clone(),
        version,
    )?;
    Ok(module.code().verified().clone())
}
