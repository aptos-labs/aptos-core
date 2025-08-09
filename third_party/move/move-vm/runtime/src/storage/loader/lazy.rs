// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    module_traversal::TraversalContext,
    storage::loader::traits::{
        FunctionDefinitionLoader, InstantiatedFunctionLoader, InstantiatedFunctionLoaderHelper,
        LegacyLoaderConfig, Loader, ModuleMetadataLoader, NativeModuleLoader, ScriptLoader,
        StructDefinitionLoader,
    },
    Function, LoadedFunction, Module, ModuleStorage, RuntimeEnvironment, Script,
    WithRuntimeEnvironment,
};
use move_binary_format::{
    errors::{Location, PartialVMResult, VMResult},
    file_format::CompiledScript,
};
use move_core_types::{
    gas_algebra::NumBytes,
    identifier::IdentStr,
    language_storage::{ModuleId, TypeTag},
    metadata::Metadata,
};
use move_vm_types::{
    code::{Code, ScriptCache},
    gas::{DependencyGasMeter, DependencyKind},
    loaded_data::{
        runtime_types::{StructType, Type},
        struct_name_indexing::StructNameIndex,
    },
    sha3_256,
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
                .unmetered_get_existing_module_size(addr, name)
                .map_err(|err| err.to_partial())?;
            gas_meter.charge_dependency(
                DependencyKind::Existing,
                addr,
                name,
                NumBytes::new(size as u64),
            )?;
        }
        Ok(())
    }

    /// Loads a module, metering it and performing lazy verification (no loading of transitive
    /// dependencies).
    fn metered_load_module(
        &self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        module_id: &ModuleId,
    ) -> VMResult<Arc<Module>> {
        self.charge_module(gas_meter, traversal_context, module_id)
            .map_err(|err| err.finish(Location::Undefined))?;
        self.module_storage
            .unmetered_get_existing_lazily_verified_module(module_id)
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
                self.metered_load_module(
                    gas_meter,
                    traversal_context,
                    &ModuleId::new(st.address, st.module.to_owned()),
                )
                .and_then(|module| module.get_struct(&st.name))
                .map_err(|err| err.to_partial())
            })
    }
}

impl<'a, T> LazyLoader<'a, T>
where
    T: ModuleStorage
        + ScriptCache<Key = [u8; 32], Deserialized = CompiledScript, Verified = Script>,
{
    fn metered_verify_and_cache_script(
        &self,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        serialized_script: &[u8],
    ) -> VMResult<Arc<Script>> {
        use Code::*;

        let hash = sha3_256(serialized_script);
        let deserialized_script = match self.module_storage.get_script(&hash) {
            Some(Verified(script)) => return Ok(script),
            Some(Deserialized(deserialized_script)) => deserialized_script,
            None => self
                .runtime_environment()
                .deserialize_into_script(serialized_script)
                .map(Arc::new)?,
        };

        let locally_verified_script = self
            .runtime_environment()
            .build_locally_verified_script(deserialized_script)?;

        let immediate_dependencies = locally_verified_script
            .immediate_dependencies_iter()
            .map(|(addr, name)| {
                let module_id = ModuleId::new(*addr, name.to_owned());
                self.metered_load_module(gas_meter, traversal_context, &module_id)
            })
            .collect::<VMResult<Vec<_>>>()?;

        let verified_script = self
            .runtime_environment()
            .build_verified_script(locally_verified_script, &immediate_dependencies)?;

        Ok(self
            .module_storage
            .insert_verified_script(hash, verified_script))
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

        self.metered_load_module(gas_meter, traversal_context, &struct_name.module)
            .and_then(|module| module.get_struct(&struct_name.name))
            .map_err(|err| err.to_partial())
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
        let module = self.metered_load_module(gas_meter, traversal_context, module_id)?;
        let function = module.get_function(function_name)?;
        Ok((module, function))
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
    ) -> PartialVMResult<Type> {
        self.metered_load_type(gas_meter, traversal_context, ty_arg)
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

impl<'a, T> ScriptLoader for LazyLoader<'a, T>
where
    T: ModuleStorage
        + ScriptCache<Key = [u8; 32], Deserialized = CompiledScript, Verified = Script>,
{
    fn load_script(
        &self,
        // For lazy loading, config is a no-op.
        _config: &LegacyLoaderConfig,
        gas_meter: &mut impl DependencyGasMeter,
        traversal_context: &mut TraversalContext,
        serialized_script: &[u8],
        ty_args: &[TypeTag],
    ) -> VMResult<LoadedFunction> {
        let script =
            self.metered_verify_and_cache_script(gas_meter, traversal_context, serialized_script)?;
        self.build_instantiated_script(gas_meter, traversal_context, script, ty_args)
    }
}
