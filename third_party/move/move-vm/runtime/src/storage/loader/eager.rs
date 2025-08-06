// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    check_dependencies_and_charge_gas, check_type_tag_dependencies_and_charge_gas,
    module_traversal::TraversalContext,
    storage::loader::traits::{
        FunctionDefinitionLoader, InstantiatedFunctionLoader, InstantiatedFunctionLoaderHelper,
        LegacyLoaderConfig, Loader, ModuleMetadataLoader, NativeModuleLoader,
        StructDefinitionLoader,
    },
    Function, LoadedFunction, Module, ModuleStorage, RuntimeEnvironment, WithRuntimeEnvironment,
};
use move_binary_format::errors::{Location, PartialVMError, PartialVMResult, VMResult};
use move_core_types::{
    identifier::IdentStr,
    language_storage::{ModuleId, TypeTag},
    metadata::Metadata,
    vm_status::StatusCode,
};
use move_vm_types::{
    gas::DependencyGasMeter,
    loaded_data::{
        runtime_types::{StructType, Type},
        struct_name_indexing::StructNameIndex,
    },
};
use std::sync::Arc;

/// Eager loader implementation used prior to lazy loading. It uses eager module verification by
/// loading and verifying the transitive closure of module's dependencies and friends. The gas is
/// metered at "entrypoints" (entry function or a script, dynamic dispatch) for the whole closure
/// at once.
pub struct EagerLoader<'a, T> {
    module_storage: &'a T,
}

impl<'a, T> EagerLoader<'a, T>
where
    T: ModuleStorage,
{
    /// Returns a new eager loader.
    pub fn new(module_storage: &'a T) -> Self {
        Self { module_storage }
    }
}

impl<'a, T> EagerLoader<'a, T>
where
    T: ModuleStorage,
{
    /// Converts a type tag into a runtime type. Can load struct definitions.
    fn unmetered_load_type(&self, tag: &TypeTag) -> PartialVMResult<Type> {
        self.runtime_environment()
            .vm_config()
            .ty_builder
            .create_ty(tag, |st| {
                self.module_storage.unmetered_get_struct_definition(
                    &st.address,
                    st.module.as_ident_str(),
                    st.name.as_ident_str(),
                )
            })
    }
}

impl<'a, T> WithRuntimeEnvironment for EagerLoader<'a, T>
where
    T: ModuleStorage,
{
    fn runtime_environment(&self) -> &RuntimeEnvironment {
        self.module_storage.runtime_environment()
    }
}

impl<'a, T> StructDefinitionLoader for EagerLoader<'a, T>
where
    T: ModuleStorage,
{
    fn is_lazy_loading_enabled(&self) -> bool {
        debug_assert!(!self.runtime_environment().vm_config().enable_lazy_loading);
        false
    }

    fn load_struct_definition(
        &self,
        _gas_meter: &mut impl DependencyGasMeter,
        _traversal_context: &mut TraversalContext,
        idx: &StructNameIndex,
    ) -> PartialVMResult<Arc<StructType>> {
        let struct_name = self
            .runtime_environment()
            .struct_name_index_map()
            .idx_to_struct_name_ref(*idx)?;

        self.module_storage.unmetered_get_struct_definition(
            struct_name.module.address(),
            struct_name.module.name(),
            struct_name.name.as_ident_str(),
        )
    }
}

impl<'a, T> FunctionDefinitionLoader for EagerLoader<'a, T>
where
    T: ModuleStorage,
{
    fn load_function_definition(
        &self,
        _gas_meter: &mut impl DependencyGasMeter,
        _traversal_context: &mut TraversalContext,
        module_id: &ModuleId,
        function_name: &IdentStr,
    ) -> VMResult<(Arc<Module>, Arc<Function>)> {
        self.module_storage
            .unmetered_get_function_definition(module_id.address(), module_id.name(), function_name)
            .map_err(|err| {
                // Note: legacy loader implementation used this error, so we need to remap.
                PartialVMError::new(StatusCode::FUNCTION_RESOLUTION_FAILURE)
                    .with_message(format!(
                        "Module or function do not exist for {}::{}::{}",
                        module_id.address(),
                        module_id.name(),
                        function_name
                    ))
                    .finish(err.location().clone())
            })
    }
}

impl<'a, T> NativeModuleLoader for EagerLoader<'a, T>
where
    T: ModuleStorage,
{
    fn charge_native_result_load_module(
        &self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        module_id: &ModuleId,
    ) -> PartialVMResult<()> {
        let arena_id = traversal_context
            .referenced_module_ids
            .alloc(module_id.clone());
        check_dependencies_and_charge_gas(self.module_storage, gas_meter, traversal_context, [(
            arena_id.address(),
            arena_id.name(),
        )])
        .map_err(|err| {
            err.to_partial().append_message_with_separator(
                '.',
                format!(
                    "Failed to charge transitive dependency for {}. Does this module exist?",
                    module_id
                ),
            )
        })?;
        Ok(())
    }
}

impl<'a, T> ModuleMetadataLoader for EagerLoader<'a, T>
where
    T: ModuleStorage,
{
    fn load_module_metadata(
        &self,
        _gas_meter: &mut impl DependencyGasMeter,
        _traversal_context: &mut TraversalContext,
        module_id: &ModuleId,
    ) -> PartialVMResult<Vec<Metadata>> {
        // Note:
        //   For backwards compatibility, metadata accesses were never metered.
        self.module_storage
            .unmetered_get_existing_module_metadata(module_id.address(), module_id.name())
            .map_err(|err| err.to_partial())
    }
}

impl<'a, T> InstantiatedFunctionLoaderHelper for EagerLoader<'a, T>
where
    T: ModuleStorage,
{
    fn load_ty_arg(
        &self,
        _gas_meter: &mut impl DependencyGasMeter,
        _traversal_context: &mut TraversalContext,
        ty_arg: &TypeTag,
    ) -> VMResult<Type> {
        self.unmetered_load_type(ty_arg)
            .map_err(|e| e.finish(Location::Undefined))
    }
}

impl<'a, T> InstantiatedFunctionLoader for EagerLoader<'a, T>
where
    T: ModuleStorage,
{
    fn load_instantiated_function(
        &self,
        config: &LegacyLoaderConfig,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        module_id: &ModuleId,
        function_name: &IdentStr,
        ty_args: &[TypeTag],
    ) -> VMResult<LoadedFunction> {
        if config.charge_for_dependencies {
            // Charge gas for function code loading.
            let arena_id = traversal_context
                .referenced_module_ids
                .alloc(module_id.clone());
            check_dependencies_and_charge_gas(
                self.module_storage,
                gas_meter,
                traversal_context,
                [(arena_id.address(), arena_id.name())],
            )?;
        }

        if config.charge_for_ty_tag_dependencies {
            // Charge gas for code loading of modules used by type arguments.
            check_type_tag_dependencies_and_charge_gas(
                self.module_storage,
                gas_meter,
                traversal_context,
                ty_args,
            )?;
        }

        let (module, function) = self.module_storage.unmetered_get_function_definition(
            module_id.address(),
            module_id.name(),
            function_name,
        )?;

        self.build_instantiated_function(gas_meter, traversal_context, module, function, ty_args)
    }
}

impl<'a, T> Loader for EagerLoader<'a, T>
where
    T: ModuleStorage,
{
    fn unmetered_module_storage(&self) -> &dyn ModuleStorage {
        self.module_storage
    }
}
