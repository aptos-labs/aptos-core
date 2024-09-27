// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Implementation of native functions for utf8 strings.

use aptos_gas_schedule::gas_params::natives::move_stdlib::{
    MEM_SWAP_BASE, MEM_SWAP_PER_ABS_VAL_UNIT,
};
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
use move_core_types::vm_status::StatusCode;
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    natives::function::PartialVMError,
    values::{Reference, Value},
};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

/***************************************************************************************************
 * native fun native_swap
 *
 *   gas cost: MEM_SWAP_BASE + MEM_SWAP_PER_ABS_VAL_UNIT * abstract_size_of_arguments
 *
 **************************************************************************************************/
fn native_swap(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(args.len() == 2);

    if args.len() != 2 {
        return Err(SafeNativeError::InvariantViolation(PartialVMError::new(
            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
        )));
    }

    let cost = MEM_SWAP_BASE
        + MEM_SWAP_PER_ABS_VAL_UNIT
            * (context.abs_val_size(&args[0]) + context.abs_val_size(&args[1]));
    context.charge(cost)?;

    let ref1 = safely_pop_arg!(args, Reference);
    let ref0 = safely_pop_arg!(args, Reference);

    ref0.swap_ref(|value0| {
        let mut value1_opt = Option::None;
        ref1.swap_ref(|value1| {
            value1_opt = Option::Some(value1);
            Ok(value0)
        })?;
        Ok(value1_opt.unwrap())
    })?;

    Ok(smallvec![])
}

/***************************************************************************************************
 * module
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [("swap", native_swap as RawSafeNative)];

    builder.make_named_natives(natives)
}
