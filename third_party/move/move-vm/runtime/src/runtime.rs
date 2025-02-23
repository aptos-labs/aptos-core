// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    data_cache::TransactionDataCache,
    interpreter::Interpreter,
    loader::LoadedFunction,
    module_traversal::TraversalContext,
    native_extensions::NativeContextExtensions,
    session::SerializedReturnValues,
    storage::{
        code_storage::CodeStorage, module_storage::ModuleStorage,
        ty_layout_converter::LoaderLayoutConverter,
    },
    AsFunctionValueExtension, LayoutConverter,
};
use move_binary_format::{
    errors::{Location, PartialVMError, PartialVMResult, VMResult},
    file_format::LocalIndex,
};
use move_core_types::{language_storage::TypeTag, value::MoveTypeLayout, vm_status::StatusCode};
use move_vm_metrics::{Timer, VM_TIMER};
use move_vm_types::{
    gas::GasMeter,
    loaded_data::runtime_types::Type,
    value_serde::ValueSerDeContext,
    values::{Locals, Reference, VMValueCast, Value},
};
use std::borrow::Borrow;

/// An instantiation of the MoveVM.
pub(crate) struct VMRuntime {}

impl VMRuntime {
    /// Creates a new runtime instance with provided environment.
    pub(crate) fn new() -> Self {
        Self {}
    }

    fn deserialize_arg(
        &self,
        module_storage: &impl ModuleStorage,
        ty: &Type,
        arg: impl Borrow<[u8]>,
    ) -> PartialVMResult<Value> {
        let (layout, has_identifier_mappings) = match LoaderLayoutConverter::new(module_storage)
            .type_to_type_layout_with_identifier_mappings(ty)
        {
            Ok(layout) => layout,
            Err(_err) => {
                return Err(PartialVMError::new(
                    StatusCode::INVALID_PARAM_TYPE_FOR_DESERIALIZATION,
                )
                .with_message("[VM] failed to get layout from type".to_string()));
            },
        };

        let deserialization_error = || -> PartialVMError {
            PartialVMError::new(StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT)
                .with_message("[VM] failed to deserialize argument".to_string())
        };

        // Make sure we do not construct values which might have identifiers
        // inside. This should be guaranteed by transaction argument validation
        // but because it does not use layouts we double-check here.
        if has_identifier_mappings {
            return Err(deserialization_error());
        }

        let function_value_extension = module_storage.as_function_value_extension();
        match ValueSerDeContext::new()
            .with_func_args_deserialization(&function_value_extension)
            .deserialize(arg.borrow(), &layout)
        {
            Some(val) => Ok(val),
            None => Err(deserialization_error()),
        }
    }

    fn deserialize_args(
        &self,
        module_storage: &impl ModuleStorage,
        param_tys: Vec<Type>,
        serialized_args: Vec<impl Borrow<[u8]>>,
    ) -> PartialVMResult<(Locals, Vec<Value>)> {
        if param_tys.len() != serialized_args.len() {
            return Err(
                PartialVMError::new(StatusCode::NUMBER_OF_ARGUMENTS_MISMATCH).with_message(
                    format!(
                        "argument length mismatch: expected {} got {}",
                        param_tys.len(),
                        serialized_args.len()
                    ),
                ),
            );
        }

        // Create a list of dummy locals. Each value stored will be used be borrowed and passed
        // by reference to the invoked function
        let mut dummy_locals = Locals::new(param_tys.len());
        // Arguments for the invoked function. These can be owned values or references
        let deserialized_args = param_tys
            .into_iter()
            .zip(serialized_args)
            .enumerate()
            .map(|(idx, (ty, arg_bytes))| match &ty {
                Type::MutableReference(inner_t) | Type::Reference(inner_t) => {
                    dummy_locals.store_loc(
                        idx,
                        self.deserialize_arg(module_storage, inner_t, arg_bytes)?,
                        module_storage.vm_config().check_invariant_in_swap_loc,
                    )?;
                    dummy_locals.borrow_loc(idx)
                },
                _ => self.deserialize_arg(module_storage, &ty, arg_bytes),
            })
            .collect::<PartialVMResult<Vec<_>>>()?;
        Ok((dummy_locals, deserialized_args))
    }

    fn serialize_return_value(
        &self,
        module_storage: &impl ModuleStorage,
        ty: &Type,
        value: Value,
    ) -> PartialVMResult<(Vec<u8>, MoveTypeLayout)> {
        let (ty, value) = match ty {
            Type::Reference(inner) | Type::MutableReference(inner) => {
                let ref_value: Reference = value.cast()?;
                let inner_value = ref_value.read_ref()?;
                (&**inner, inner_value)
            },
            _ => (ty, value),
        };

        let (layout, has_identifier_mappings) = LoaderLayoutConverter::new(module_storage)
            .type_to_type_layout_with_identifier_mappings(ty)
            .map_err(|_err| {
                // TODO: Should we use `err` instead of mapping?
                PartialVMError::new(StatusCode::VERIFICATION_ERROR).with_message(
                    "entry point functions cannot have non-serializable return types".to_string(),
                )
            })?;

        let serialization_error = || -> PartialVMError {
            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                .with_message("failed to serialize return values".to_string())
        };

        // Disallow native values to escape through return values of a function.
        if has_identifier_mappings {
            return Err(serialization_error());
        }

        let function_value_extension = module_storage.as_function_value_extension();
        let bytes = ValueSerDeContext::new()
            .with_func_args_deserialization(&function_value_extension)
            .serialize(&value, &layout)?
            .ok_or_else(serialization_error)?;
        Ok((bytes, layout))
    }

    fn serialize_return_values(
        &self,
        module_storage: &impl ModuleStorage,
        return_types: &[Type],
        return_values: Vec<Value>,
    ) -> PartialVMResult<Vec<(Vec<u8>, MoveTypeLayout)>> {
        if return_types.len() != return_values.len() {
            return Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message(
                    format!(
                        "declared {} return types, but got {} return values",
                        return_types.len(),
                        return_values.len()
                    ),
                ),
            );
        }

        return_types
            .iter()
            .zip(return_values)
            .map(|(ty, value)| self.serialize_return_value(module_storage, ty, value))
            .collect()
    }

    fn execute_function_impl(
        &self,
        function: LoadedFunction,
        serialized_args: Vec<impl Borrow<[u8]>>,
        data_store: &mut TransactionDataCache,
        module_storage: &impl ModuleStorage,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        extensions: &mut NativeContextExtensions,
    ) -> VMResult<SerializedReturnValues> {
        let ty_builder = module_storage.ty_builder();
        let ty_args = function.ty_args();

        let param_tys = function
            .param_tys()
            .iter()
            .map(|ty| ty_builder.create_ty_with_subst(ty, ty_args))
            .collect::<PartialVMResult<Vec<_>>>()
            .map_err(|err| err.finish(Location::Undefined))?;
        let mut_ref_args = param_tys
            .iter()
            .enumerate()
            .filter_map(|(idx, ty)| match ty {
                Type::MutableReference(inner) => Some((idx, inner.clone())),
                _ => None,
            })
            .collect::<Vec<_>>();
        let (mut dummy_locals, deserialized_args) = self
            .deserialize_args(module_storage, param_tys, serialized_args)
            .map_err(|e| e.finish(Location::Undefined))?;
        let return_tys = function
            .return_tys()
            .iter()
            .map(|ty| ty_builder.create_ty_with_subst(ty, ty_args))
            .collect::<PartialVMResult<Vec<_>>>()
            .map_err(|err| err.finish(Location::Undefined))?;

        let timer = VM_TIMER.timer_with_label("Interpreter::entrypoint");
        let return_values = Interpreter::entrypoint(
            function,
            deserialized_args,
            data_store,
            module_storage,
            gas_meter,
            traversal_context,
            extensions,
        )?;
        drop(timer);

        let serialized_return_values = self
            .serialize_return_values(module_storage, &return_tys, return_values)
            .map_err(|e| e.finish(Location::Undefined))?;
        let serialized_mut_ref_outputs = mut_ref_args
            .into_iter()
            .map(|(idx, ty)| {
                // serialize return values first in the case that a value points into this local
                let local_val = dummy_locals
                    .move_loc(idx, module_storage.vm_config().check_invariant_in_swap_loc)?;
                let (bytes, layout) =
                    self.serialize_return_value(module_storage, &ty, local_val)?;
                Ok((idx as LocalIndex, bytes, layout))
            })
            .collect::<PartialVMResult<_>>()
            .map_err(|e| e.finish(Location::Undefined))?;

        // locals should not be dropped until all return values are serialized
        drop(dummy_locals);

        Ok(SerializedReturnValues {
            mutable_reference_outputs: serialized_mut_ref_outputs,
            return_values: serialized_return_values,
        })
    }

    pub(crate) fn execute_function_instantiation(
        &self,
        func: LoadedFunction,
        serialized_args: Vec<impl Borrow<[u8]>>,
        data_store: &mut TransactionDataCache,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        extensions: &mut NativeContextExtensions,
        module_storage: &impl ModuleStorage,
    ) -> VMResult<SerializedReturnValues> {
        self.execute_function_impl(
            func,
            serialized_args,
            data_store,
            module_storage,
            gas_meter,
            traversal_context,
            extensions,
        )
    }

    pub(crate) fn execute_script(
        &self,
        script: impl Borrow<[u8]>,
        ty_args: Vec<TypeTag>,
        serialized_args: Vec<impl Borrow<[u8]>>,
        data_store: &mut TransactionDataCache,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        extensions: &mut NativeContextExtensions,
        code_storage: &impl CodeStorage,
    ) -> VMResult<()> {
        // Load the script first, verify it, and then execute the entry-point main function.
        let main = code_storage.load_script(script.borrow(), &ty_args)?;

        self.execute_function_impl(
            main,
            serialized_args,
            data_store,
            code_storage,
            gas_meter,
            traversal_context,
            extensions,
        )?;
        Ok(())
    }
}
