// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::function_info::extract_function_info;
use velor_gas_schedule::gas_params::natives::velor_framework::DISPATCHABLE_AUTHENTICATE_DISPATCH_BASE;
use velor_native_interface::{
    RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError, SafeNativeResult,
};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::SmallVec;
use std::collections::VecDeque;

/***************************************************************************************************
 * native fun dispatchable_authenticate
 *
 *   Directs control flow based on the last argument. We use the same native function implementation
 *   for all dispatching native.
 *   gas cost: a flat fee because we charged the loading of those modules previously.
 *
 **************************************************************************************************/
pub(crate) fn native_dispatch(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let (module_name, func_name) = extract_function_info(&mut arguments)?;

    // Check that the module is already properly charged in this transaction.
    context
        .traversal_context()
        .check_is_special_or_visited(module_name.address(), module_name.name())
        .map_err(|_| SafeNativeError::Abort { abort_code: 4 })?;

    context.charge(DISPATCHABLE_AUTHENTICATE_DISPATCH_BASE)?;

    // Use Error to instruct the VM to perform a function call dispatch.
    Err(SafeNativeError::FunctionDispatch {
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
    let natives = [(
        "dispatchable_authenticate",
        native_dispatch as RawSafeNative,
    )];

    builder.make_named_natives(natives)
}
