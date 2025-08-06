// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    module_traversal::TraversalContext,
    storage::loader::traits::{
        FunctionDefinitionLoader, InstantiatedFunctionLoader, InstantiatedFunctionLoaderHelper,
        LegacyLoaderConfig, Loader, ModuleMetadataLoader, NativeModuleLoader,
        StructDefinitionLoader,
    },
    Function, LoadedFunction, Module, ModuleStorage, RuntimeEnvironment, WithRuntimeEnvironment,
};
use move_binary_format::errors::{Location, PartialVMResult, VMResult};
use move_core_types::{
    gas_algebra::NumBytes,
    identifier::IdentStr,
    language_storage::{ModuleId, TypeTag},
    metadata::Metadata,
};
use move_vm_types::{
    gas::DependencyGasMeter,
    loaded_data::{
        runtime_types::{StructType, Type},
        struct_name_indexing::StructNameIndex,
    },
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

    /// Converts a type tag into a runtime type, metering any loading of struct definitions.
    fn metered_load_type(
        &self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        tag: &TypeTag,
    ) -> PartialVMResult<Type> {
        self.runtime_environment()
            .vm_config()
            .ty_builder
            .create_ty(tag, |st| {
                self.charge_module(gas_meter, traversal_context, &st.module_id())?;
                self.module_storage.unmetered_get_struct_definition(
                    &st.address,
                    &st.module,
                    st.name.as_ident_str(),
                )
            })
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

impl<'a, T> FunctionDefinitionLoader for LazyLoader<'a, T>
where
    T: ModuleStorage,
{
    fn load_function_definition(
        &self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        module_id: &ModuleId,
        function_name: &IdentStr,
    ) -> VMResult<(Arc<Module>, Arc<Function>)> {
        self.charge_module(gas_meter, traversal_context, module_id)
            .map_err(|err| err.finish(Location::Module(module_id.clone())))?;
        self.module_storage.unmetered_get_function_definition(
            module_id.address(),
            module_id.name(),
            function_name,
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

impl<'a, T> InstantiatedFunctionLoaderHelper for LazyLoader<'a, T>
where
    T: ModuleStorage,
{
    fn load_ty_arg(
        &self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        ty_arg: &TypeTag,
    ) -> VMResult<Type> {
        self.metered_load_type(gas_meter, traversal_context, ty_arg)
            .map_err(|e| e.finish(Location::Undefined))
    }
}

impl<'a, T> InstantiatedFunctionLoader for LazyLoader<'a, T>
where
    T: ModuleStorage,
{
    fn load_instantiated_function(
        &self,
        // For lazy loading, not used!
        _config: &LegacyLoaderConfig,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        module_id: &ModuleId,
        function_name: &IdentStr,
        ty_args: &[TypeTag],
    ) -> VMResult<LoadedFunction> {
        let (module, function) =
            self.load_function_definition(gas_meter, traversal_context, module_id, function_name)?;

        self.build_instantiated_function(gas_meter, traversal_context, module, function, ty_args)
    }
}

impl<'a, T> Loader for LazyLoader<'a, T>
where
    T: ModuleStorage,
{
    fn unmetered_module_storage(&self) -> &dyn ModuleStorage {
        self.module_storage
    }
}
