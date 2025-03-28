// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    check_dependencies_and_charge_gas, check_type_tag_dependencies_and_charge_gas,
    config::VMConfig,
    loader::LazyLoadedFunction,
    module_traversal::{TraversalContext, TraversalStorage},
    CodeStorage, LoadedFunction, LoadedFunctionOwner, Module,
};
use move_binary_format::errors::{Location, PartialVMError, PartialVMResult, VMResult};
use move_core_types::{
    function::ClosureMask,
    identifier::IdentStr,
    language_storage::{ModuleId, TypeTag},
    value::MoveTypeLayout,
};
use move_vm_types::{
    gas::GasMeter,
    loaded_data::runtime_types::{StructType, Type},
    values::AbstractFunction,
};
use std::{rc::Rc, sync::Arc};

pub trait Loader {
    fn vm_config(&self) -> &VMConfig;

    /// Returns true if the module has been seen and charged gas. If the module has not been seen
    /// by execution, or does not exist (so also not seen), returns false.
    fn is_charged(&self, module_id: &ModuleId) -> bool;

    /// If module does not exist, returns an error. If module exits and is cached, returns it. If
    /// the module is not cached, also charges gas for it.
    fn load_module(
        &mut self,
        gas_meter: &mut impl GasMeter,
        module_id: &ModuleId,
    ) -> VMResult<Arc<Module>>;

    /// Loads a type based on the specified [TypeTag]. Returns an error if the module does not
    /// exist. Loading process may load modules and charge gas.
    fn load_ty(&mut self, gas_meter: &mut impl GasMeter, ty_tag: &TypeTag) -> VMResult<Type>;

    /// Loads the layout corresponding to the specified [Type]. Returns an error if any module that
    /// is needed for layout construction does not exist. Loading process may load modules and
    /// charge gas.
    /// In addition to layout, returns a boolean indicating weather there are any delayed fields
    /// in the layout.
    fn load_layout_with_delayed_fields_check(
        &mut self,
        gas_meter: &mut impl GasMeter,
        ty: &Type,
    ) -> PartialVMResult<(MoveTypeLayout, bool)>;

    // TODO(lazy): document
    fn load_and_check_depth_formula(
        &mut self,
        gas_meter: &mut impl GasMeter,
        ty: &Type,
    ) -> PartialVMResult<()>;

    fn load_and_resolve_function_from_closure(
        &mut self,
        gas_meter: &mut impl GasMeter,
        function_from_closure: &dyn AbstractFunction,
    ) -> PartialVMResult<(Rc<LoadedFunction>, ClosureMask)>;

    /// Loads a struct based on the specified name. Returns an error if the module does not exist.
    /// Loading process may load modules and charge gas.
    fn load_struct(
        &mut self,
        gas_meter: &mut impl GasMeter,
        module_id: &ModuleId,
        struct_name: &IdentStr,
    ) -> VMResult<Arc<StructType>> {
        let module = self.load_module(gas_meter, module_id)?;
        module.get_struct(struct_name)
    }

    fn load_script_entrypoint(
        &mut self,
        gas_meter: &mut impl GasMeter,
        serialized_script: &[u8],
        ty_args: &[TypeTag],
    ) -> VMResult<LoadedFunction>;

    fn load_function_entrypoint(
        &mut self,
        gas_meter: &mut impl GasMeter,
        module_id: &ModuleId,
        function_name: &IdentStr,
        ty_args: &[TypeTag],
    ) -> VMResult<LoadedFunction>;

    /// Loads a function based on the specified name and with the given arguments. Returns an error
    /// if the module does not exist. Loading process may load modules and charge gas.
    fn load_function(
        &mut self,
        gas_meter: &mut impl GasMeter,
        module_id: &ModuleId,
        function_name: &IdentStr,
        ty_args: Vec<Type>,
    ) -> VMResult<LoadedFunction> {
        let module = self.load_module(gas_meter, module_id)?;
        let function = module.get_function(function_name)?;

        Type::verify_ty_arg_abilities(function.ty_param_abilities(), &ty_args)
            .map_err(|e| e.finish(Location::Module(module_id.clone())))?;
        Ok(LoadedFunction {
            owner: LoadedFunctionOwner::Module(module),
            ty_args,
            function,
        })
    }
}

pub struct EagerLoader<'a, T> {
    traversal_context: TraversalContext<'a>,
    code_storage: &'a T,
    #[allow(dead_code)]
    check_dependencies_and_charge_gas: bool,
    #[allow(dead_code)]
    check_type_tag_dependencies_and_charge_gas: bool,
}

impl<'a, T> EagerLoader<'a, T>
where
    T: CodeStorage,
{
    pub fn new(traversal_storage: &'a TraversalStorage, code_storage: &'a T) -> Self {
        let traversal_context = TraversalContext::new(traversal_storage);
        Self {
            traversal_context,
            code_storage,
            check_dependencies_and_charge_gas: false,
            check_type_tag_dependencies_and_charge_gas: false,
        }
    }

    pub fn new_with(
        traversal_storage: &'a TraversalStorage,
        code_storage: &'a T,
        check_dependencies_and_charge_gas: bool,
        check_type_tag_dependencies_and_charge_gas: bool,
    ) -> Self {
        let traversal_context = TraversalContext::new(traversal_storage);
        Self {
            traversal_context,
            code_storage,
            check_dependencies_and_charge_gas,
            check_type_tag_dependencies_and_charge_gas,
        }
    }
}

impl<'a, T> Loader for EagerLoader<'a, T>
where
    T: CodeStorage,
{
    fn vm_config(&self) -> &VMConfig {
        self.code_storage.runtime_environment().vm_config()
    }

    fn is_charged(&self, module_id: &ModuleId) -> bool {
        self.traversal_context
            .visited
            .contains_key(&(module_id.address(), module_id.name()))
    }

    fn load_module(
        &mut self,
        _gas_meter: &mut impl GasMeter,
        _module_id: &ModuleId,
    ) -> VMResult<Arc<Module>> {
        todo!()
    }

    fn load_ty(&mut self, _gas_meter: &mut impl GasMeter, _ty_tag: &TypeTag) -> VMResult<Type> {
        todo!()
    }

    fn load_layout_with_delayed_fields_check(
        &mut self,
        _gas_meter: &mut impl GasMeter,
        _ty: &Type,
    ) -> PartialVMResult<(MoveTypeLayout, bool)> {
        todo!()
    }

    fn load_and_check_depth_formula(
        &mut self,
        _gas_meter: &mut impl GasMeter,
        _ty: &Type,
    ) -> PartialVMResult<()> {
        todo!()
    }

    fn load_and_resolve_function_from_closure(
        &mut self,
        gas_meter: &mut impl GasMeter,
        function_from_closure: &dyn AbstractFunction,
    ) -> PartialVMResult<(Rc<LoadedFunction>, ClosureMask)> {
        let lazy_function = LazyLoadedFunction::expect_this_impl(function_from_closure)?;
        let mask = lazy_function.closure_mask();

        // Before trying to resolve the function, charge gas for associated
        // module loading.
        lazy_function.with_name_and_ty_args(|module_id, _, ty_arg_tags| {
            let module_id = module_id.ok_or_else(|| {
                // TODO(#15664): currently we need the module id for gas charging
                //   of calls, so we can't proceed here without one. But we want
                //   to be able to let scripts use closures.
                let msg = format!(
                    "module id required to charge gas for function `{}`",
                    lazy_function.to_stable_string()
                );
                PartialVMError::new_invariant_violation(msg)
            })?;

            // Charge gas for function code loading.
            let arena_id = self
                .traversal_context
                .referenced_module_ids
                .alloc(module_id.clone());
            check_dependencies_and_charge_gas(
                self.code_storage,
                gas_meter,
                &mut self.traversal_context,
                [(arena_id.address(), arena_id.name())],
            )
            .map_err(|err| {
                // TODO(lazy): is it fine to remap back to partial?
                err.to_partial()
            })?;

            // Charge gas for code loading of modules used by type arguments.
            check_type_tag_dependencies_and_charge_gas(
                self.code_storage,
                gas_meter,
                &mut self.traversal_context,
                ty_arg_tags,
            )
            .map_err(|err| {
                // TODO(lazy): is it fine to remap back to partial?
                err.to_partial()
            })?;
            Ok(())
        })?;

        // Resolve the function. This may lead to loading the code related
        // to this function.
        let callee = lazy_function.with_resolved_function(self, gas_meter, |f| Ok(f.clone()))?;
        Ok((callee, mask))
    }

    fn load_script_entrypoint(
        &mut self,
        _gas_meter: &mut impl GasMeter,
        _serialized_script: &[u8],
        _ty_args: &[TypeTag],
    ) -> VMResult<LoadedFunction> {
        todo!()
    }

    fn load_function_entrypoint(
        &mut self,
        _gas_meter: &mut impl GasMeter,
        _module_id: &ModuleId,
        _function_name: &IdentStr,
        _ty_args: &[TypeTag],
    ) -> VMResult<LoadedFunction> {
        todo!()
    }
}
