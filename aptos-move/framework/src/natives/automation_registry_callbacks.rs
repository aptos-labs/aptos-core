// Copyright (c) 2025 Supra.
// SPDX-License-Identifier: Apache-2.0
use aptos_native_interface::{
    SafeNativeBuilder, SafeNativeContext, SafeNativeResult,
};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Value},
};
use smallvec::{smallvec, SmallVec};
use std::{collections::VecDeque};


/***************************************************************************************************
 * native fun to check whether the binary has been updated which supports automation registry
 * lifecycle management based on the cycle
 *
 * dummy function
 *
 * gas cost: 0
 *
 **************************************************************************************************/
fn native_automation_cycle_management_support(
    _context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    _arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    Ok(smallvec![Value::bool(true)])
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [
        ("native_automation_cycle_management_support", native_automation_cycle_management_support),
    ];

    builder.make_named_natives(natives)
}
