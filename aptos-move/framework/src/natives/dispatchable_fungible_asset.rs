use super::function_info::extract_function_info;
use aptos_gas_algebra::InternalGas;
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use aptos_native_interface::{
    RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError, SafeNativeResult,
};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::SmallVec;
use std::collections::VecDeque;

/***************************************************************************************************
 * native fun dispatchable_withdraw
 *
 *   Directs control flow based on the last argument.
 *   gas cost: TBD
 *
 **************************************************************************************************/
pub(crate) fn native_dispatch(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let (module_name, func_name) = extract_function_info(&mut arguments)?;
    if !context.traversal_context().visited.contains_key(&(module_name.address(), module_name.name())) {
        return Err(SafeNativeError::Abort { abort_code: 4 });
    }
    Err(SafeNativeError::FunctionDispatch {
        cost: InternalGas::zero(),
        module_name,
        func_name,
        ty_args,
        args: arguments.into_iter().collect(),
    })
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [
        ("dispatchable_withdraw", native_dispatch as RawSafeNative),
        ("dispatchable_deposit", native_dispatch),
        ("dispatchable_derived_balance", native_dispatch),
    ];

    builder.make_named_natives(natives)
}
