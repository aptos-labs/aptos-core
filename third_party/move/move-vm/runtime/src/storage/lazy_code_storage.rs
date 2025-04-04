// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    module_traversal::TraversalContext, CodeStorage, LoadedFunction, LoadedFunctionOwner,
    ModuleStorage,
};
use move_binary_format::errors::{Location, PartialVMResult, VMResult};
use move_core_types::{
    gas_algebra::NumBytes,
    identifier::IdentStr,
    language_storage::{ModuleId, TypeTag},
    vm_status::{sub_status::type_resolution_failure, StatusCode},
};
use move_vm_metrics::{Timer, VM_TIMER};
use move_vm_types::{gas::GasMeter, loaded_data::runtime_types::Type};

pub struct LazyMeteredCodeStorage<'a, T> {
    code_storage: &'a T,
}

impl<'a, T> LazyMeteredCodeStorage<'a, T>
where
    T: ModuleStorage,
{
    pub fn new(code_storage: &'a T) -> Self {
        debug_assert!(
            code_storage
                .runtime_environment()
                .vm_config()
                .use_lazy_loading
        );
        Self { code_storage }
    }

    fn metered_lazy_load_ty_args(
        &self,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        ty_args: &[TypeTag],
    ) -> PartialVMResult<Vec<Type>> {
        let ty_builder = &self
            .code_storage
            .runtime_environment()
            .vm_config()
            .ty_builder;
        ty_args
            .iter()
            .map(|tag| {
                ty_builder.create_ty(tag, |st| {
                    let module_id = traversal_context
                        .referenced_module_ids
                        .alloc(st.module_id());
                    let addr = module_id.address();
                    let name = module_id.name();

                    if traversal_context.visit_if_not_special_address(addr, name) {
                        let size = self
                            .code_storage
                            .unmetered_get_existing_module_size(addr, name)
                            .map_err(|err| err.to_partial())?;
                        gas_meter.charge_dependency(
                            false,
                            module_id.address(),
                            module_id.name(),
                            NumBytes::new(size as u64),
                        )?;
                    }

                    self.code_storage.unmetered_get_struct_definition(
                        &st.address,
                        st.module.as_ident_str(),
                        st.name.as_ident_str(),
                    )
                })
            })
            .collect::<PartialVMResult<Vec<_>>>()
            .map_err(|err| {
                // User provided type argument failed to load. Set extra sub status to distinguish
                // from internal type loading error.
                if StatusCode::TYPE_RESOLUTION_FAILURE == err.major_status() {
                    err.with_sub_status(type_resolution_failure::EUSER_TYPE_LOADING_FAILURE)
                } else {
                    err
                }
            })
    }

    pub fn metered_lazy_load_function(
        &self,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        module_id: &ModuleId,
        function_name: &IdentStr,
        ty_args: &[TypeTag],
    ) -> VMResult<LoadedFunction> {
        let _timer = VM_TIMER.timer_with_label("Loader::load_function");

        let module_id = traversal_context
            .referenced_module_ids
            .alloc(module_id.clone());
        let addr = module_id.address();
        let name = module_id.name();

        if traversal_context.visit_if_not_special_address(addr, name) {
            let size = self
                .code_storage
                .unmetered_get_existing_module_size(addr, name)?;
            gas_meter
                .charge_dependency(false, addr, name, NumBytes::new(size as u64))
                .map_err(|err| err.finish(Location::Undefined))?;
        }

        let (module, function) =
            self.code_storage
                .unmetered_get_function_definition(addr, name, function_name)?;

        let ty_args = self
            .metered_lazy_load_ty_args(gas_meter, traversal_context, ty_args)
            .map_err(|err| err.finish(Location::Undefined))?;
        Type::verify_ty_arg_abilities(function.ty_param_abilities(), &ty_args)
            .map_err(|err| err.finish(Location::Module(module_id.clone())))?;

        Ok(LoadedFunction {
            owner: LoadedFunctionOwner::Module(module),
            ty_args,
            function,
        })
    }
}

impl<'a, T> LazyMeteredCodeStorage<'a, T>
where
    T: CodeStorage,
{
    pub fn metered_lazy_load_script(
        &self,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        serialized_script: &[u8],
        ty_args: &[TypeTag],
    ) -> VMResult<LoadedFunction> {
        let compiled_script = self
            .code_storage
            .deserialize_and_cache_script(serialized_script)?;

        let locally_verified_script = self
            .code_storage
            .runtime_environment()
            .build_locally_verified_script(compiled_script)?;

        // For scripts, make sure we charging gas for immediate dependencies first.
        for (addr, name) in locally_verified_script.immediate_dependencies_iter() {
            let module_id = traversal_context
                .referenced_module_ids
                .alloc(ModuleId::new(*addr, name.to_owned()));
            let addr = module_id.address();
            let name = module_id.name();

            if traversal_context.visit_if_not_special_address(addr, name) {
                let size = self
                    .code_storage
                    .unmetered_get_existing_module_size(addr, name)?;
                gas_meter
                    .charge_dependency(false, addr, name, NumBytes::new(size as u64))
                    .map_err(|err| err.finish(Location::Undefined))?;
            }
        }

        // At this point, we charged gas for immediate dependencies, and verified the script.
        let script = self
            .code_storage
            .verify_and_cache_script(serialized_script)?;
        let main = script.entry_point();

        let ty_args = self
            .metered_lazy_load_ty_args(gas_meter, traversal_context, ty_args)
            .map_err(|err| err.finish(Location::Script))?;
        Type::verify_ty_arg_abilities(main.ty_param_abilities(), &ty_args)
            .map_err(|err| err.finish(Location::Script))?;

        Ok(LoadedFunction {
            owner: LoadedFunctionOwner::Script(script),
            ty_args,
            function: main,
        })
    }
}
