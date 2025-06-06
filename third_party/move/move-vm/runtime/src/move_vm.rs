// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    data_cache::TransactionDataCache,
    interpreter::Interpreter,
    native_extensions::NativeContextExtensions,
    storage::ty_layout_converter::{LayoutConverter, StorageLayoutConverter},
    AsFunctionValueExtension, LoadedFunction, ModuleStorage,
};
use move_binary_format::{
    errors::{Location, PartialVMError, PartialVMResult, VMResult},
    file_format::LocalIndex,
};
use move_core_types::{value::MoveTypeLayout, vm_status::StatusCode};
use move_vm_metrics::{Timer, VM_TIMER};
use move_vm_types::{
    gas::GasMeter,
    loaded_data::runtime_types::Type,
    resolver::ResourceResolver,
    value_serde::ValueSerDeContext,
    values::{Locals, Reference, VMValueCast, Value},
};
use std::borrow::Borrow;

/// Return values from function execution in [MoveVm].
#[derive(Debug)]
pub struct SerializedReturnValues {
    /// The value of any arguments that were mutably borrowed. Non-mut borrowed values are not
    /// included.
    pub mutable_reference_outputs: Vec<(LocalIndex, Vec<u8>, MoveTypeLayout)>,
    /// The return values from the function.
    pub return_values: Vec<(Vec<u8>, MoveTypeLayout)>,
}

/// Move VM is completely stateless. It is used to execute a single loaded function with its type
/// arguments fully instantiated.
pub struct MoveVM;

impl MoveVM {
    /// Executes provided function with the specified arguments. The arguments are serialized, and
    /// are not checked by the VM. It is the responsibility of the caller of this function to
    /// verify that they are well-formed.
    ///
    /// During execution, [MoveVm] can modify values from the global storage. Modified values are
    /// cached in data store. Reads to the global values are also cached in the data store. The
    /// caller can decide what to do with the read and written global values after [MoveVm] is done
    /// executing the function. Additionally, modifications can be made to the native extensions.
    ///
    /// When execution finishes, the return values of the function are returned. Additionally, if
    /// there are any mutable references passed as arguments, these values are also returned.
    pub fn execute_loaded_function(
        function: LoadedFunction,
        serialized_args: Vec<impl Borrow<[u8]>>,
        data_cache: &mut TransactionDataCache,
        gas_meter: &mut impl GasMeter,
        extensions: &mut NativeContextExtensions,
        module_storage: &impl ModuleStorage,
        resource_resolver: &impl ResourceResolver,
    ) -> VMResult<SerializedReturnValues> {
        let vm_config = module_storage.runtime_environment().vm_config();
        let ty_builder = &vm_config.ty_builder;

        let create_ty_with_subst = |tys: &[Type]| -> VMResult<Vec<Type>> {
            tys.iter()
                .map(|ty| ty_builder.create_ty_with_subst(ty, function.ty_args()))
                .collect::<PartialVMResult<Vec<_>>>()
                .map_err(|err| err.finish(Location::Undefined))
        };

        let param_tys = create_ty_with_subst(function.param_tys())?;
        let (mut dummy_locals, deserialized_args) =
            deserialize_args(module_storage, &param_tys, serialized_args)
                .map_err(|e| e.finish(Location::Undefined))?;

        let return_tys = create_ty_with_subst(function.return_tys())?;

        let return_values = {
            let _timer = VM_TIMER.timer_with_label("Interpreter::entrypoint");
            Interpreter::entrypoint(
                function,
                deserialized_args,
                data_cache,
                module_storage,
                resource_resolver,
                gas_meter,
                extensions,
            )?
        };

        let return_values = serialize_return_values(module_storage, &return_tys, return_values)
            .map_err(|e| e.finish(Location::Undefined))?;
        let mutable_reference_outputs = param_tys
            .iter()
            .enumerate()
            .filter_map(|(idx, ty)| match ty {
                Type::MutableReference(inner_ty) => Some((idx, inner_ty.as_ref())),
                _ => None,
            })
            .map(|(idx, ty)| {
                // serialize return values first in the case that a value points into this local
                let local_val =
                    dummy_locals.move_loc(idx, vm_config.check_invariant_in_swap_loc)?;
                let (bytes, layout) = serialize_return_value(module_storage, ty, local_val)?;
                Ok((idx as LocalIndex, bytes, layout))
            })
            .collect::<PartialVMResult<_>>()
            .map_err(|e| e.finish(Location::Undefined))?;

        // locals should not be dropped until all return values are serialized
        drop(dummy_locals);

        Ok(SerializedReturnValues {
            mutable_reference_outputs,
            return_values,
        })
    }
}

fn deserialize_arg(
    module_storage: &impl ModuleStorage,
    ty: &Type,
    arg: impl Borrow<[u8]>,
) -> PartialVMResult<Value> {
    let (layout, has_identifier_mappings) = StorageLayoutConverter::new(module_storage)
        .type_to_type_layout_with_identifier_mappings(ty)
        .map_err(|_| {
            PartialVMError::new(StatusCode::INVALID_PARAM_TYPE_FOR_DESERIALIZATION)
                .with_message("[VM] failed to get layout from type".to_string())
        })?;

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
    ValueSerDeContext::new()
        .with_func_args_deserialization(&function_value_extension)
        .deserialize(arg.borrow(), &layout)
        .ok_or_else(deserialization_error)
}

fn deserialize_args(
    module_storage: &impl ModuleStorage,
    param_tys: &[Type],
    serialized_args: Vec<impl Borrow<[u8]>>,
) -> PartialVMResult<(Locals, Vec<Value>)> {
    if param_tys.len() != serialized_args.len() {
        return Err(
            PartialVMError::new(StatusCode::NUMBER_OF_ARGUMENTS_MISMATCH).with_message(format!(
                "argument length mismatch: expected {} got {}",
                param_tys.len(),
                serialized_args.len()
            )),
        );
    }

    // Create a list of dummy locals. Each value stored will be used be borrowed and passed
    // by reference to the invoked function
    let mut dummy_locals = Locals::new(param_tys.len());

    // Arguments for the invoked function. These can be owned values or references
    let vm_config = module_storage.runtime_environment().vm_config();
    let deserialized_args = param_tys
        .iter()
        .zip(serialized_args)
        .enumerate()
        .map(|(idx, (ty, arg_bytes))| match ty.get_ref_inner_ty() {
            Some(inner_ty) => {
                dummy_locals.store_loc(
                    idx,
                    deserialize_arg(module_storage, inner_ty, arg_bytes)?,
                    vm_config.check_invariant_in_swap_loc,
                )?;
                dummy_locals.borrow_loc(idx)
            },
            None => deserialize_arg(module_storage, ty, arg_bytes),
        })
        .collect::<PartialVMResult<Vec<_>>>()?;
    Ok((dummy_locals, deserialized_args))
}

fn serialize_return_value(
    module_storage: &impl ModuleStorage,
    ty: &Type,
    value: Value,
) -> PartialVMResult<(Vec<u8>, MoveTypeLayout)> {
    let (ty, value) = match ty.get_ref_inner_ty() {
        Some(inner_ty) => {
            let ref_value: Reference = value.cast()?;
            let inner_value = ref_value.read_ref()?;
            (inner_ty, inner_value)
        },
        None => (ty, value),
    };

    let (layout, has_identifier_mappings) = StorageLayoutConverter::new(module_storage)
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
    module_storage: &impl ModuleStorage,
    return_tys: &[Type],
    return_values: Vec<Value>,
) -> PartialVMResult<Vec<(Vec<u8>, MoveTypeLayout)>> {
    if return_tys.len() != return_values.len() {
        return Err(
            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message(
                format!(
                    "declared {} return types, but got {} return values",
                    return_tys.len(),
                    return_values.len()
                ),
            ),
        );
    }

    return_tys
        .iter()
        .zip(return_values)
        .map(|(ty, value)| serialize_return_value(module_storage, ty, value))
        .collect()
}
