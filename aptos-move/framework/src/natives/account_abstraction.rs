// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::function_info::extract_function_info;
use aptos_gas_schedule::gas_params::natives::aptos_framework::DISPATCHABLE_AUTHENTICATE_DISPATCH_BASE;
use aptos_native_interface::{
    RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError, SafeNativeResult,
};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{ty_interner::TypeId, values::Value};
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
    ty_args: &[TypeId],
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
        ty_args: ty_args.to_vec(),
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
