// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    check_dependencies_and_charge_gas, check_type_tag_dependencies_and_charge_gas,
    loader::LazyLoadedFunction,
    module_traversal::TraversalContext,
    storage::ty_layout_converter::{
        LayoutConverter, MetredLazyLayoutConverter, UnmeteredLayoutConverter,
    },
    LazyMeteredCodeStorage, LoadedFunction, LoadedFunctionOwner, ModuleStorage, RuntimeEnvironment,
};
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{
    gas_algebra::NumBytes,
    identifier::IdentStr,
    language_storage::{ModuleId, TypeTag},
    metadata::Metadata,
    value::MoveTypeLayout,
    vm_status::StatusCode,
};
use move_vm_types::{
    gas::GasMeter,
    loaded_data::{
        runtime_types::{StructType, Type},
        struct_name_indexing::StructNameIndex,
    },
    values::AbstractFunction,
};
use std::sync::Arc;

pub trait MoveVmLoader {
    fn runtime_environment(&self) -> &RuntimeEnvironment;

    fn as_module_storage(&self) -> &impl ModuleStorage;

    fn load_struct_definition(
        &self,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        idx: &StructNameIndex,
    ) -> PartialVMResult<Arc<StructType>>;

    fn load_function_with_verified_ty_args(
        &self,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        module_id: &ModuleId,
        function_name: &IdentStr,
        verified_ty_args: Vec<Type>,
    ) -> PartialVMResult<LoadedFunction>;

    fn load_module_metadata(
        &self,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        module_id: &ModuleId,
    ) -> PartialVMResult<Vec<Metadata>>;

    fn load_ty_layout(
        &self,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        ty: &Type,
    ) -> PartialVMResult<MoveTypeLayout> {
        let (layout, _) =
            self.load_ty_layout_with_delayed_fields_check(gas_meter, traversal_context, ty)?;
        Ok(layout)
    }

    fn load_ty_layout_with_delayed_fields_check(
        &self,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        ty: &Type,
    ) -> PartialVMResult<(MoveTypeLayout, bool)>;

    fn charge_native_load_module(
        &self,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        module_id: ModuleId,
    ) -> PartialVMResult<()>;

    fn charge_before_resolve_closure(
        &self,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        function: &LazyLoadedFunction,
    ) -> PartialVMResult<ModuleId>;

    fn resolve_closure(
        &self,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        module_id: &ModuleId,
        function_name: &IdentStr,
        ty_args: &[TypeTag],
    ) -> PartialVMResult<LoadedFunction>;
}

pub struct LazyLoader<'a, T> {
    code_storage: &'a T,
}

impl<'a, T> LazyLoader<'a, T>
where
    T: ModuleStorage,
{
    pub fn new(code_storage: &'a T) -> Self {
        assert!(
            code_storage
                .runtime_environment()
                .vm_config()
                .use_lazy_loading
        );
        Self { code_storage }
    }

    pub(crate) fn charge_module(
        &self,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        module_id: ModuleId,
    ) -> PartialVMResult<()> {
        let module_id = traversal_context.referenced_module_ids.alloc(module_id);
        let addr = module_id.address();
        let name = module_id.name();

        if traversal_context.visit_if_not_special_address(addr, name) {
            let size = self
                .code_storage
                .unmetered_get_existing_module_size(addr, name)
                .map_err(|err| err.to_partial())?;
            gas_meter.charge_dependency(false, addr, name, NumBytes::new(size as u64))?;
        }
        Ok(())
    }
}

impl<'a, T> MoveVmLoader for LazyLoader<'a, T>
where
    T: ModuleStorage,
{
    fn runtime_environment(&self) -> &RuntimeEnvironment {
        self.code_storage.runtime_environment()
    }

    fn as_module_storage(&self) -> &impl ModuleStorage {
        self.code_storage
    }

    fn load_struct_definition(
        &self,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        idx: &StructNameIndex,
    ) -> PartialVMResult<Arc<StructType>> {
        let struct_name = self
            .code_storage
            .runtime_environment()
            .struct_name_index_map()
            .idx_to_struct_name_ref(*idx)?;

        // Charge gas before fetching the definition.
        self.charge_module(gas_meter, traversal_context, struct_name.module.clone())?;
        self.code_storage.unmetered_get_struct_definition(
            struct_name.module.address(),
            struct_name.module.name(),
            struct_name.name.as_ident_str(),
        )
    }

    fn load_function_with_verified_ty_args(
        &self,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        module_id: &ModuleId,
        function_name: &IdentStr,
        verified_ty_args: Vec<Type>,
    ) -> PartialVMResult<LoadedFunction> {
        self.charge_module(gas_meter, traversal_context, module_id.clone())?;
        let (module, function) = self
            .code_storage
            .unmetered_get_function_definition(module_id.address(), module_id.name(), function_name)
            .map_err(|err| err.to_partial())?;
        Ok(LoadedFunction {
            owner: LoadedFunctionOwner::Module(module),
            ty_args: verified_ty_args,
            function,
        })
    }

    fn load_module_metadata(
        &self,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        module_id: &ModuleId,
    ) -> PartialVMResult<Vec<Metadata>> {
        self.charge_module(gas_meter, traversal_context, module_id.clone())?;
        self.code_storage
            .unmetered_get_existing_module_metadata(module_id.address(), module_id.name())
            .map_err(|err| err.to_partial())
    }

    fn load_ty_layout_with_delayed_fields_check(
        &self,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        ty: &Type,
    ) -> PartialVMResult<(MoveTypeLayout, bool)> {
        MetredLazyLayoutConverter::new(gas_meter, traversal_context, self.code_storage)
            .type_to_type_layout_with_identifier_mappings(ty)
    }

    fn charge_native_load_module(
        &self,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        module_id: ModuleId,
    ) -> PartialVMResult<()> {
        self.charge_module(gas_meter, traversal_context, module_id)
    }

    fn charge_before_resolve_closure(
        &self,
        _gas_meter: &mut impl GasMeter,
        _traversal_context: &mut TraversalContext,
        function: &LazyLoadedFunction,
    ) -> PartialVMResult<ModuleId> {
        function.with_name_and_ty_args(|module_id, _, _| {
            let module_id = module_id.ok_or_else(|| {
                // TODO(#15664): currently we need the module id for gas charging
                //   of calls, so we can't proceed here without one. But we want
                //   to be able to let scripts use closures.
                PartialVMError::new_invariant_violation(format!(
                    "module id required to charge gas for function `{}`",
                    function.to_stable_string()
                ))
            })?;
            Ok(module_id.clone())
        })
    }

    fn resolve_closure(
        &self,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        module_id: &ModuleId,
        function_name: &IdentStr,
        ty_args: &[TypeTag],
    ) -> PartialVMResult<LoadedFunction> {
        LazyMeteredCodeStorage::new(self.code_storage)
            .metered_lazy_load_function(
                gas_meter,
                traversal_context,
                module_id,
                function_name,
                ty_args,
            )
            .map_err(|err| err.to_partial())
    }
}

pub struct LegacyLoader<'a, T> {
    code_storage: &'a T,
}

impl<'a, T> LegacyLoader<'a, T>
where
    T: ModuleStorage,
{
    pub fn new(code_storage: &'a T) -> Self {
        assert!(
            !code_storage
                .runtime_environment()
                .vm_config()
                .use_lazy_loading
        );
        Self { code_storage }
    }
}

impl<'a, T> MoveVmLoader for LegacyLoader<'a, T>
where
    T: ModuleStorage,
{
    fn runtime_environment(&self) -> &RuntimeEnvironment {
        self.code_storage.runtime_environment()
    }

    fn as_module_storage(&self) -> &impl ModuleStorage {
        self.code_storage
    }

    fn load_struct_definition(
        &self,
        _gas_meter: &mut impl GasMeter,
        _traversal_context: &mut TraversalContext,
        idx: &StructNameIndex,
    ) -> PartialVMResult<Arc<StructType>> {
        let struct_name = self
            .code_storage
            .runtime_environment()
            .struct_name_index_map()
            .idx_to_struct_name_ref(*idx)?;
        self.code_storage.unmetered_get_struct_definition(
            struct_name.module.address(),
            struct_name.module.name(),
            struct_name.name.as_ident_str(),
        )
    }

    fn load_function_with_verified_ty_args(
        &self,
        _gas_meter: &mut impl GasMeter,
        _traversal_context: &mut TraversalContext,
        module_id: &ModuleId,
        function_name: &IdentStr,
        verified_ty_args: Vec<Type>,
    ) -> PartialVMResult<LoadedFunction> {
        let (module, function) = self
            .code_storage
            .unmetered_get_function_definition(module_id.address(), module_id.name(), function_name)
            .map_err(|_| {
                // Note: legacy loader implementation used this error, so we need to remap.
                PartialVMError::new(StatusCode::FUNCTION_RESOLUTION_FAILURE).with_message(format!(
                    "Module or function do not exist for {}::{}::{}",
                    module_id.address(),
                    module_id.name(),
                    function_name
                ))
            })?;
        Ok(LoadedFunction {
            owner: LoadedFunctionOwner::Module(module),
            ty_args: verified_ty_args,
            function,
        })
    }

    fn load_module_metadata(
        &self,
        _gas_meter: &mut impl GasMeter,
        _traversal_context: &mut TraversalContext,
        module_id: &ModuleId,
    ) -> PartialVMResult<Vec<Metadata>> {
        self.code_storage
            .unmetered_get_existing_module_metadata(module_id.address(), module_id.name())
            .map_err(|err| err.to_partial())
    }

    fn load_ty_layout_with_delayed_fields_check(
        &self,
        _gas_meter: &mut impl GasMeter,
        _traversal_context: &mut TraversalContext,
        ty: &Type,
    ) -> PartialVMResult<(MoveTypeLayout, bool)> {
        UnmeteredLayoutConverter::new(self.code_storage)
            .type_to_type_layout_with_identifier_mappings(ty)
    }

    fn charge_native_load_module(
        &self,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        module_id: ModuleId,
    ) -> PartialVMResult<()> {
        let module_id = traversal_context.referenced_module_ids.alloc(module_id);
        check_dependencies_and_charge_gas(self.code_storage, gas_meter, traversal_context, [(
            module_id.address(),
            module_id.name(),
        )])
        .map_err(|err| {
            let msg = format!(
                "Failed to charge transitive dependency for {}. Does this module exists?",
                module_id
            );
            err.to_partial().append_message_with_separator('.', msg)
        })
    }

    fn charge_before_resolve_closure(
        &self,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        function: &LazyLoadedFunction,
    ) -> PartialVMResult<ModuleId> {
        function.with_name_and_ty_args(|module_id, _, ty_args| {
            let module_id = module_id.cloned().ok_or_else(|| {
                // TODO(#15664): currently we need the module id for gas charging
                //   of calls, so we can't proceed here without one. But we want
                //   to be able to let scripts use closures.
                PartialVMError::new_invariant_violation(format!(
                    "module id required to charge gas for function `{}`",
                    function.to_stable_string()
                ))
            })?;

            // Charge gas for function code loading.
            let arena_id = traversal_context
                .referenced_module_ids
                .alloc(module_id.clone());
            check_dependencies_and_charge_gas(self.code_storage, gas_meter, traversal_context, [(
                arena_id.address(),
                arena_id.name(),
            )])
            .map_err(|err| err.to_partial())?;

            // Charge gas for code loading of modules used by type arguments.
            check_type_tag_dependencies_and_charge_gas(
                self.code_storage,
                gas_meter,
                traversal_context,
                ty_args,
            )
            .map_err(|err| err.to_partial())?;
            Ok(module_id)
        })
    }

    fn resolve_closure(
        &self,
        _gas_meter: &mut impl GasMeter,
        _traversal_context: &mut TraversalContext,
        module_id: &ModuleId,
        function_name: &IdentStr,
        ty_args: &[TypeTag],
    ) -> PartialVMResult<LoadedFunction> {
        self.code_storage
            .unmetered_load_function(module_id, function_name, ty_args)
            .map_err(|err| err.to_partial())
    }
}
