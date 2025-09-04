// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Implementation of native functions for value comparison.

use velor_gas_schedule::gas_params::natives::move_stdlib::{
    CMP_COMPARE_BASE, CMP_COMPARE_PER_ABS_VAL_UNIT,
};
use velor_native_interface::{
    RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError, SafeNativeResult,
};
use move_core_types::vm_status::StatusCode;
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    natives::function::PartialVMError,
    values::{Struct, Value},
};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

const ORDERING_LESS_THAN_VARIANT: u16 = 0;
const ORDERING_EQUAL_VARIANT: u16 = 1;
const ORDERING_GREATER_THAN_VARIANT: u16 = 2;

/***************************************************************************************************
 * native fun native_compare
 *
 *   gas cost: CMP_COMPARE_BASE + CMP_COMPARE_PER_ABS_VAL_UNIT * dereferenced_size_of_both_values
 *
 **************************************************************************************************/
fn native_compare(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(args.len() == 2);
    if args.len() != 2 {
        return Err(SafeNativeError::InvariantViolation(PartialVMError::new(
            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
        )));
    }

    let cost = CMP_COMPARE_BASE
        + CMP_COMPARE_PER_ABS_VAL_UNIT
            * (context.abs_val_size_dereferenced(&args[0])?
                + context.abs_val_size_dereferenced(&args[1])?);
    context.charge(cost)?;

    let ordering = args[0].compare(&args[1])?;
    let ordering_move_variant = match ordering {
        std::cmp::Ordering::Less => ORDERING_LESS_THAN_VARIANT,
        std::cmp::Ordering::Equal => ORDERING_EQUAL_VARIANT,
        std::cmp::Ordering::Greater => ORDERING_GREATER_THAN_VARIANT,
    };

    Ok(smallvec![Value::struct_(Struct::pack(vec![Value::u16(
        ordering_move_variant
    )]))])
}

/***************************************************************************************************
 * module
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [("compare", native_compare as RawSafeNative)];

    builder.make_named_natives(natives)
}
