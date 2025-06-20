// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
use move_binary_format::errors::PartialVMError;
use move_core_types::{
    account_address::AccountAddress, gas_algebra::NumBytes, identifier::Identifier,
    language_storage::ModuleId, vm_status::StatusCode,
};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Reference, StructRef, Value, VectorRef},
};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

// Extract Identifer from a move value of type &String
fn identifier_from_ref(v: Value) -> SafeNativeResult<Identifier> {
    let bytes = v
        .value_as::<StructRef>()
        .and_then(|s| s.borrow_field(0))
        .and_then(|v| v.value_as::<VectorRef>())
        .map_err(SafeNativeError::InvariantViolation)?
        .as_bytes_ref()
        .to_vec();
    Identifier::from_utf8(bytes).map_err(|_| SafeNativeError::Abort { abort_code: 1 })
}

pub(crate) fn extract_function_info(
    arguments: &mut VecDeque<Value>,
) -> SafeNativeResult<(ModuleId, Identifier)> {
    match arguments.pop_back() {
        Some(val) => match val.value_as::<StructRef>() {
            Ok(v) => {
                let module_address = v
                    .borrow_field(0)
                    .and_then(|v| v.value_as::<Reference>())
                    .and_then(|v| v.read_ref())
                    .and_then(|v| v.value_as::<AccountAddress>())
                    .map_err(SafeNativeError::InvariantViolation)?;

                let module_name = identifier_from_ref(
                    v.borrow_field(1)
                        .map_err(SafeNativeError::InvariantViolation)?,
                )?;

                let func_name = identifier_from_ref(
                    v.borrow_field(2)
                        .map_err(SafeNativeError::InvariantViolation)?,
                )?;
                Ok((ModuleId::new(module_address, module_name), func_name))
            },
            Err(e) => Err(SafeNativeError::InvariantViolation(e)),
        },
        None => Err(SafeNativeError::InvariantViolation(PartialVMError::new(
            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
        ))),
    }
}

/***************************************************************************************************
 * native fun check_dispatch_type_compatibility_impl
 *
 *   Returns true if the function argument types of rhs is the same as (arguments type of lhs || &FunctionInfo)
 *   gas cost: base_cost + unit_cost * type_size
 *
 **************************************************************************************************/
fn native_check_dispatch_type_compatibility_impl(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(arguments.len() == 2);

    // TODO: Figure out the correct gas charging schema here.
    //
    // We need to load the modules from lhs and rhs, and cloning the bytes for module id and function name.
    context.charge(FUNCTION_INFO_CHECK_DISPATCH_TYPE_COMPATIBILITY_IMPL_BASE)?;

    let (rhs, rhs_id) = {
        let (module, func) = extract_function_info(&mut arguments)?;
        let is_err = if context.get_feature_flags().is_account_abstraction_enabled()
            || context
                .get_feature_flags()
                .is_derivable_account_abstraction_enabled()
        {
            !module.address().is_special()
                && !context
                    .traversal_context()
                    .visited
                    .contains_key(&(module.address(), module.name()))
        } else {
            !context
                .traversal_context()
                .visited
                .contains_key(&(module.address(), module.name()))
        };
        if is_err {
            return Err(SafeNativeError::Abort { abort_code: 2 });
        }
        (
            context
                .load_function(&module, &func)
                .map_err(|_| SafeNativeError::Abort { abort_code: 2 })?,
            module,
        )
    };
    let (lhs, lhs_id) = {
        let (module, func) = extract_function_info(&mut arguments)?;
        (
            context
                .load_function(&module, &func)
                .map_err(|_| SafeNativeError::Abort { abort_code: 2 })?,
            module,
        )
    };

    if lhs.param_tys().is_empty() {
        return Err(SafeNativeError::Abort { abort_code: 2 });
    }

    Ok(smallvec![Value::bool(
        rhs.ty_param_abilities() == lhs.ty_param_abilities()
            && rhs.return_tys() == lhs.return_tys()
            && &lhs.param_tys()[0..lhs.param_count() - 1] == rhs.param_tys()
            && !rhs.is_friend_or_private()
            && !rhs.is_native()
            && lhs_id != rhs_id
    )])
}

/***************************************************************************************************
 * native fun is_identifier
 *
 *   Returns true if the string passed in is a valid Move identifier
 *   gas cost: base_cost + unit_cost * num_of_bytes
 *
 **************************************************************************************************/
fn native_is_identifier(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(arguments.len() == 1);

    let s_arg = safely_pop_arg!(arguments, VectorRef);
    let s_ref = s_arg.as_bytes_ref();

    context.charge(
        FUNCTION_INFO_CHECK_IS_IDENTIFIER_BASE
            + FUNCTION_INFO_CHECK_IS_IDENTIFIER_PER_BYTE
                * NumBytes::new(s_ref.as_slice().len() as u64),
    )?;

    let result = if let Ok(str) = std::str::from_utf8(&s_ref) {
        Identifier::is_valid(str)
    } else {
        false
    };

    Ok(smallvec![Value::bool(result)])
}

/***************************************************************************************************
 * native fun load_function_impl
 *
 *   Load up a module related to the function and charge gas.
 *   gas cost: base_cost + transitive deps size of the function.
 *
 **************************************************************************************************/
fn native_load_function_impl(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(arguments.len() == 1);

    context.charge(FUNCTION_INFO_LOAD_FUNCTION_BASE)?;
    let (module_name, _) = extract_function_info(&mut arguments)?;

    Err(SafeNativeError::LoadModule { module_name })
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [
        (
            "check_dispatch_type_compatibility_impl",
            native_check_dispatch_type_compatibility_impl as RawSafeNative,
        ),
        ("is_identifier", native_is_identifier),
        ("load_function_impl", native_load_function_impl),
    ];

    builder.make_named_natives(natives)
}
