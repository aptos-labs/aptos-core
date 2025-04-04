// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use aptos_types::transaction::Script;
use move_binary_format::errors::{Location, PartialVMError, VMResult};
use move_core_types::{
    identifier::IdentStr,
    language_storage::{ModuleId, TypeTag},
    vm_status::StatusCode,
};
use move_vm_runtime::{
    check_dependencies_and_charge_gas, check_script_dependencies_and_check_gas,
    check_type_tag_dependencies_and_charge_gas, module_traversal::TraversalContext, CodeStorage,
    LazyMeteredCodeStorage, LoadedFunction, ModuleStorage, RuntimeEnvironment,
};
use move_vm_types::gas::GasMeter;

pub(crate) trait AptosVmLoader {
    #[allow(dead_code)]
    fn runtime_environment(&self) -> &RuntimeEnvironment;

    fn load_function(
        &self,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        module_id: &ModuleId,
        function_name: &IdentStr,
        ty_args: &[TypeTag],
    ) -> VMResult<LoadedFunction>;
}

pub(crate) trait AptosVmScriptLoader {
    fn load_script(
        &self,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        script: &Script,
    ) -> VMResult<LoadedFunction>;
}

pub(crate) struct LazyLoader<'a, T> {
    code_storage: LazyMeteredCodeStorage<'a, T>,
}

impl<'a, T> LazyLoader<'a, T>
where
    T: ModuleStorage,
{
    pub(crate) fn new(code_storage: &'a T) -> VMResult<Self> {
        if code_storage
            .runtime_environment()
            .vm_config()
            .use_lazy_loading
        {
            Ok(Self {
                code_storage: LazyMeteredCodeStorage::new(code_storage),
            })
        } else {
            let err = PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                .with_message(
                    "Lazy loader cannot be used if lazy loading is not enabled".to_string(),
                )
                .finish(Location::Undefined);
            Err(err)
        }
    }
}

impl<'a, T> AptosVmLoader for LazyLoader<'a, T>
where
    T: ModuleStorage,
{
    fn runtime_environment(&self) -> &RuntimeEnvironment {
        self.code_storage.runtime_environment()
    }

    fn load_function(
        &self,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        module_id: &ModuleId,
        function_name: &IdentStr,
        ty_args: &[TypeTag],
    ) -> VMResult<LoadedFunction> {
        self.code_storage.metered_lazy_load_function(
            gas_meter,
            traversal_context,
            module_id,
            function_name,
            ty_args,
        )
    }
}

impl<'a, T> AptosVmScriptLoader for LazyLoader<'a, T>
where
    T: CodeStorage,
{
    fn load_script(
        &self,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        script: &Script,
    ) -> VMResult<LoadedFunction> {
        self.code_storage.metered_lazy_load_script(
            gas_meter,
            traversal_context,
            script.code(),
            script.ty_args(),
        )
    }
}

pub(crate) struct LegacyLoader<'a, T> {
    meter_dependencies_and_friends: bool,
    meter_ty_tags: bool,
    code_storage: &'a T,
}

impl<'a, T> LegacyLoader<'a, T>
where
    T: ModuleStorage,
{
    pub(crate) fn new(
        meter_dependencies_and_friends: bool,
        meter_ty_tags: bool,
        code_storage: &'a T,
    ) -> VMResult<Self> {
        if code_storage
            .runtime_environment()
            .vm_config()
            .use_lazy_loading
        {
            let err = PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                .with_message("Legacy loader cannot be used if lazy loading is enabled".to_string())
                .finish(Location::Undefined);
            Err(err)
        } else {
            Ok(Self {
                meter_dependencies_and_friends,
                meter_ty_tags,
                code_storage,
            })
        }
    }
}

impl<'a, T> AptosVmLoader for LegacyLoader<'a, T>
where
    T: ModuleStorage,
{
    fn runtime_environment(&self) -> &RuntimeEnvironment {
        self.code_storage.runtime_environment()
    }

    fn load_function(
        &self,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        module_id: &ModuleId,
        function_name: &IdentStr,
        ty_args: &[TypeTag],
    ) -> VMResult<LoadedFunction> {
        if self.meter_dependencies_and_friends {
            let module_id = traversal_context
                .referenced_module_ids
                .alloc(module_id.clone());
            check_dependencies_and_charge_gas(self.code_storage, gas_meter, traversal_context, [
                (module_id.address(), module_id.name()),
            ])?;
        }

        if self.meter_ty_tags {
            check_type_tag_dependencies_and_charge_gas(
                self.code_storage,
                gas_meter,
                traversal_context,
                ty_args,
            )?;
        }

        self.code_storage
            .unmetered_load_function(module_id, function_name, ty_args)
    }
}

impl<'a, T> AptosVmScriptLoader for LegacyLoader<'a, T>
where
    T: CodeStorage,
{
    fn load_script(
        &self,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        script: &Script,
    ) -> VMResult<LoadedFunction> {
        if self.meter_dependencies_and_friends {
            check_script_dependencies_and_check_gas(
                self.code_storage,
                gas_meter,
                traversal_context,
                script.code(),
            )?;
        }

        if self.meter_ty_tags {
            check_type_tag_dependencies_and_charge_gas(
                self.code_storage,
                gas_meter,
                traversal_context,
                script.ty_args(),
            )?;
        }

        self.code_storage
            .unmetered_load_script(script.code(), script.ty_args())
    }
}
