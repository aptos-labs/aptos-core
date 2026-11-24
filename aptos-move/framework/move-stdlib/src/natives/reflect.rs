// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Implementation of native functions for reflection.

use crate::natives::result;
use aptos_gas_schedule::{
    gas_feature_versions::RELEASE_V1_51,
    gas_params::natives::aptos_framework::{
        REFLECT_RESOLVE_BASE, REFLECT_RESOLVE_NOT_FOUND, REFLECT_RESOLVE_PER_BYTE,
    },
};
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
use aptos_types::move_stdlib::reflect::{is_forbidden_to_reflect, parse_name, INVALID_IDENTIFIER};
use move_core_types::{
    account_address::AccountAddress, function::FunctionResolutionError, gas_algebra::NumBytes,
    language_storage::ModuleId,
};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    natives::function::PartialVMError,
    values::{Struct, StructRef, Value, VectorRef},
};
use smallvec::{smallvec, SmallVec};
use std::{collections::VecDeque, iter};

fn native_resolve(
    context: &mut SafeNativeContext,
    ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    // Charge base cost before anything else.
    context.charge(REFLECT_RESOLVE_BASE)?;

    // Process arguments
    debug_assert!(ty_args.len() == 1);
    let Some(fun_ty) = ty_args.first() else {
        return Err(SafeNativeError::InvariantViolation(
            PartialVMError::new_invariant_violation("wrong number of type arguments"),
        ));
    };

    debug_assert!(args.len() == 3);
    let fun_name_arg = string_bytes(safely_pop_arg!(args))?;
    let mod_name_arg = string_bytes(safely_pop_arg!(args))?;
    let addr = safely_pop_arg!(args, AccountAddress);

    let (fun_name, mod_name) = {
        let fun_name_bytes = fun_name_arg.as_bytes_ref()?;
        let mod_name_bytes = mod_name_arg.as_bytes_ref()?;
        let max_name_len = if context.gas_feature_version() >= RELEASE_V1_51 {
            // Charge for both names before reading them.
            context.charge(
                REFLECT_RESOLVE_PER_BYTE
                    * (NumBytes::new(fun_name_bytes.len() as u64)
                        + NumBytes::new(mod_name_bytes.len() as u64)),
            )?;
            context.get_feature_flags().get_max_identifier_size()
        } else {
            u64::MAX
        };
        let Some(fun_name) = parse_name(&fun_name_bytes, max_name_len) else {
            return Ok(smallvec![result::err_result(pack_err(INVALID_IDENTIFIER))]);
        };
        let Some(mod_name) = parse_name(&mod_name_bytes, max_name_len) else {
            return Ok(smallvec![result::err_result(pack_err(INVALID_IDENTIFIER))]);
        };
        (fun_name, mod_name)
    };
    let mod_id = ModuleId::new(addr, mod_name);

    if is_forbidden_to_reflect(mod_id.address(), mod_id.name(), &fun_name) {
        return Ok(smallvec![result::err_result(pack_err(
            FunctionResolutionError::FunctionNotAccessible as u16
        ))]);
    }

    // Resolve function and return closure. Notice the loader context function
    // takes care of gas metering and type checking.
    let resolved = context
        .loader_context()
        .resolve_function(&mod_id, &fun_name, fun_ty)?;
    match resolved {
        Ok(fun) => {
            // Return as a closure with no captured arguments
            Ok(smallvec![result::ok_result(Value::closure(
                fun,
                iter::empty()
            ))])
        },
        Err(err) => {
            let not_found = match err {
                FunctionResolutionError::FunctionNotFound => true,
                FunctionResolutionError::Reserved
                | FunctionResolutionError::FunctionNotAccessible
                | FunctionResolutionError::FunctionIncompatibleType
                | FunctionResolutionError::FunctionNotInstantiated => false,
            };
            if not_found && context.gas_feature_version() >= RELEASE_V1_51 {
                // The loader does not charge for a lookup that finds nothing.
                context.charge(REFLECT_RESOLVE_NOT_FOUND)?;
            }
            Ok(smallvec![result::err_result(pack_err(err as u16))])
        },
    }
}

/// Returns the bytes of a Move value of type `&String`.
fn string_bytes(value: Value) -> SafeNativeResult<VectorRef> {
    value
        .value_as::<StructRef>()
        .and_then(|string_ref| string_ref.borrow_field(0))
        .and_then(|bytes| bytes.value_as::<VectorRef>())
        .map_err(SafeNativeError::InvariantViolation)
}

fn pack_err(err: u16) -> Value {
    Value::struct_(Struct::pack_variant(err, vec![]))
}

pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [("native_resolve", native_resolve as RawSafeNative)];
    builder.make_named_natives(natives)
}
