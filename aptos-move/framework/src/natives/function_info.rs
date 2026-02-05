// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

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

mod abort_codes {
    /// String is not a valid Move identifier
    pub const EINVALID_IDENTIFIER: u64 = 1;
    /// Function specified in the FunctionInfo doesn't exist on chain
    pub const EINVALID_FUNCTION: u64 = 2;
}

// Extract Identifier from a move value of type &String
fn identifier_from_ref(v: Value) -> SafeNativeResult<Identifier> {
    let bytes = v
        .value_as::<StructRef>()
        .and_then(|s| s.borrow_field(0))
        .and_then(|v| v.value_as::<VectorRef>())
        .map_err(SafeNativeError::InvariantViolation)?
        .as_bytes_ref()
        .to_vec();
    Identifier::from_utf8(bytes).map_err(|_| {
        SafeNativeError::abort_with_message(
            abort_codes::EINVALID_IDENTIFIER,
            "String is not a valid Move identifier",
        )
    })
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
    _ty_args: &[Type],
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(arguments.len() == 2);

    // TODO: Figure out the correct gas charging schema here.
    //
    // We need to load the modules from lhs and rhs, and cloning the bytes for module id and function name.
    context.charge(FUNCTION_INFO_CHECK_DISPATCH_TYPE_COMPATIBILITY_IMPL_BASE)?;

    let (rhs, rhs_id) = {
        let (module, func) = extract_function_info(&mut arguments)?;

        let check_visited = |a, n| {
            let special_addresses_considered_visited =
                context.get_feature_flags().is_account_abstraction_enabled()
                    || context
                        .get_feature_flags()
                        .is_derivable_account_abstraction_enabled();
            if special_addresses_considered_visited {
                context
                    .traversal_context()
                    .check_is_special_or_visited(a, n)
            } else {
                context.traversal_context().legacy_check_visited(a, n)
            }
        };

        check_visited(module.address(), module.name()).map_err(|_| {
            SafeNativeError::abort_with_message(
                abort_codes::EINVALID_FUNCTION,
                format!(
                    "Module {}::{} is not loaded prior to native dispatch",
                    module.address(),
                    module.name()
                ),
            )
        })?;

        let function = context.load_function(&module, &func).map_err(|_| {
            SafeNativeError::abort_with_message(
                abort_codes::EINVALID_FUNCTION,
                format!(
                    "Cannot load RHS function: {}::{}::{}",
                    module.address(),
                    module.name(),
                    func
                ),
            )
        })?;

        (function, module)
    };

    let (lhs, lhs_id, lhs_func_name) = {
        let (module, func) = extract_function_info(&mut arguments)?;

        let function = context.load_function(&module, &func).map_err(|_| {
            SafeNativeError::abort_with_message(
                abort_codes::EINVALID_FUNCTION,
                format!(
                    "Cannot load LHS function: {}::{}::{}",
                    module.address(),
                    module.name(),
                    func
                ),
            )
        })?;

        (function, module, func)
    };

    if lhs.param_tys().is_empty() {
        return Err(SafeNativeError::abort_with_message(
            abort_codes::EINVALID_FUNCTION,
            format!(
                "Expected LHS function {}::{}::{} to have 1 or more parameters",
                lhs_id.address(),
                lhs_id.name(),
                lhs_func_name
            ),
        ));
    }

    Ok(smallvec![Value::bool(
        rhs.ty_param_abilities() == lhs.ty_param_abilities()
            && rhs.return_tys() == lhs.return_tys()
            && &lhs.param_tys()[0..lhs.param_count() - 1] == rhs.param_tys()
            && rhs.is_public()
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
    _ty_args: &[Type],
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
    _ty_args: &[Type],
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(arguments.len() == 1);

    context.charge(FUNCTION_INFO_LOAD_FUNCTION_BASE)?;
    let (module_name, _) = extract_function_info(&mut arguments)?;

    if context.has_direct_gas_meter_access_in_native_context() {
        context.charge_gas_for_dependencies(module_name)?;
        Ok(smallvec![])
    } else {
        // Legacy flow, VM will charge gas for module loading.
        Err(SafeNativeError::LoadModule { module_name })
    }
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
