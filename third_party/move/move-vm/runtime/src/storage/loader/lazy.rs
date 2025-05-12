// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    module_traversal::TraversalContext,
    storage::loader::traits::{
        Loader, ModuleMetadataLoader, NativeModuleLoader, StructDefinitionLoader,
    },
    ModuleStorage, RuntimeEnvironment, WithRuntimeEnvironment,
};
use move_binary_format::errors::PartialVMResult;
use move_core_types::{gas_algebra::NumBytes, language_storage::ModuleId, metadata::Metadata};
use move_vm_types::{
    gas::DependencyGasMeter,
    loaded_data::{runtime_types::StructType, struct_name_indexing::StructNameIndex},
    module_linker_error,
};
use std::sync::Arc;

/// Loader implementation used after lazy loading is enabled. Every module access is metered
/// dynamically (if it is first access to a module with the current [TraversalContext], then gas is
/// charged). Module verification is lazy: there is no loading of transitive closure of module's
/// dependencies and friends when accessing a verified module, a function definition or a struct
/// definition.
pub struct LazyLoader<'a, T> {
    module_storage: &'a T,
}

impl<'a, T> LazyLoader<'a, T>
where
    T: ModuleStorage,
{
    /// Returns a new lazy loader.
    pub fn new(module_storage: &'a T) -> Self {
        Self { module_storage }
    }

    /// Charges gas for the module load if the module has not been loaded already.
    fn charge_module(
        &self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        module_id: &ModuleId,
    ) -> PartialVMResult<()> {
        if traversal_context.visit_if_not_special_module_id(module_id) {
            let addr = module_id.address();
            let name = module_id.name();

            let size = self
                .module_storage
                .unmetered_get_module_size(addr, name)
                .map_err(|err| err.to_partial())?
                .ok_or_else(|| module_linker_error!(addr, name).to_partial())?;
            gas_meter.charge_dependency(false, addr, name, NumBytes::new(size as u64))?;
        }
        Ok(())
    }
}

impl<'a, T> WithRuntimeEnvironment for LazyLoader<'a, T>
where
    T: ModuleStorage,
{
    fn runtime_environment(&self) -> &RuntimeEnvironment {
        self.module_storage.runtime_environment()
    }
}

impl<'a, T> StructDefinitionLoader for LazyLoader<'a, T>
where
    T: ModuleStorage,
{
    fn is_lazy_loading_enabled(&self) -> bool {
        debug_assert!(self.runtime_environment().vm_config().enable_lazy_loading);
        true
    }

    fn load_struct_definition(
        &self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        idx: &StructNameIndex,
    ) -> PartialVMResult<Arc<StructType>> {
        let struct_name = self
            .runtime_environment()
            .struct_name_index_map()
            .idx_to_struct_name_ref(*idx)?;

        self.charge_module(gas_meter, traversal_context, &struct_name.module)?;
        self.module_storage.unmetered_get_struct_definition(
            struct_name.module.address(),
            struct_name.module.name(),
            struct_name.name.as_ident_str(),
        )
    }
}

impl<'a, T> NativeModuleLoader for LazyLoader<'a, T>
where
    T: ModuleStorage,
{
    fn charge_native_result_load_module(
        &self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        module_id: &ModuleId,
    ) -> PartialVMResult<()> {
        self.charge_module(gas_meter, traversal_context, module_id)?;
        Ok(())
    }
}

impl<'a, T> ModuleMetadataLoader for LazyLoader<'a, T>
where
    T: ModuleStorage,
{
    fn load_module_metadata(
        &self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        module_id: &ModuleId,
    ) -> PartialVMResult<Vec<Metadata>> {
        self.charge_module(gas_meter, traversal_context, module_id)?;
        self.module_storage
            .unmetered_get_existing_module_metadata(module_id.address(), module_id.name())
            .map_err(|err| err.to_partial())
    }
}

impl<'a, T> Loader for LazyLoader<'a, T> where T: ModuleStorage {}
