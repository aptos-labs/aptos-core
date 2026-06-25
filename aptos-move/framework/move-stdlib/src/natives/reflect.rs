// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Implementation of native functions for reflection.

use crate::natives::result;
use aptos_gas_schedule::gas_params::natives::aptos_framework::REFLECT_RESOLVE_BASE;
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
use move_core_types::{
    account_address::AccountAddress,
    identifier::{IdentStr, Identifier},
    language_storage::ModuleId,
};
use move_vm_runtime::native_functions::{FunctionResolutionError, NativeFunction};
use move_vm_types::{
    loaded_data::runtime_types::Type,
    natives::function::PartialVMError,
    values::{Struct, StructRef, Value, VectorRef},
};
use smallvec::{smallvec, SmallVec};
use std::{collections::VecDeque, iter};

const INVALID_IDENTIFIER: u16 = 0;

/// Functions that reflection refuses to resolve, identified by `(module_name, function_name)` at the
/// framework address `0x1`. A function is forbidden when its call-site rules are enforced by the
/// bytecode verifier and therefore cannot be upheld for a dynamically-resolved function value.
const FORBIDDEN_FRAMEWORK_FUNCTIONS: &[(&str, &str)] = &[("event", "emit")];

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
    let Some(fun_name) = identifier_from_string(safely_pop_arg!(args))? else {
        return Ok(smallvec![result::err_result(pack_err(INVALID_IDENTIFIER))]);
    };
    let Some(mod_name) = identifier_from_string(safely_pop_arg!(args))? else {
        return Ok(smallvec![result::err_result(pack_err(INVALID_IDENTIFIER))]);
    };
    let addr = safely_pop_arg!(args, AccountAddress);
    let mod_id = ModuleId::new(addr, mod_name);

    if is_forbidden_to_reflect(&mod_id, &fun_name) {
        return Ok(smallvec![result::err_result(pack_err(
            FunctionResolutionError::FunctionNotAccessible as u16
        ))]);
    }

    // Resolve function and return closure. Notice the loader context function
    // takes care of gas metering and type checking.
    match context
        .loader_context()
        .resolve_function(&mod_id, &fun_name, fun_ty)?
    {
        Ok(fun) => {
            // Return as a closure with no captured arguments
            Ok(smallvec![result::ok_result(Value::closure(
                fun,
                iter::empty()
            ))])
        },
        Err(e) => Ok(smallvec![result::err_result(pack_err(e as u16))]),
    }
}

/// Returns true if reflection must refuse to resolve `mod_id::fun_name`, i.e., it is one of the
/// `FORBIDDEN_FRAMEWORK_FUNCTIONS` at the framework address `0x1`.
fn is_forbidden_to_reflect(mod_id: &ModuleId, fun_name: &IdentStr) -> bool {
    mod_id.address() == &AccountAddress::ONE
        && FORBIDDEN_FRAMEWORK_FUNCTIONS
            .iter()
            .any(|&(module, function)| {
                mod_id.name().as_str() == module && fun_name.as_str() == function
            })
}

/// Extract Identifier from a move value of type &String
fn identifier_from_string(v: Value) -> SafeNativeResult<Option<Identifier>> {
    let bytes = v
        .value_as::<StructRef>()
        .and_then(|s| s.borrow_field(0))
        .and_then(|v| v.value_as::<VectorRef>())
        .map_err(SafeNativeError::InvariantViolation)?
        .as_bytes_ref()?
        .to_vec();
    Ok(Identifier::from_utf8(bytes).ok())
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
