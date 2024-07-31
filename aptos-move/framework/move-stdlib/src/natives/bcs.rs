// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use aptos_gas_schedule::gas_params::natives::move_stdlib::*;
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
use move_core_types::{
    gas_algebra::NumBytes, value::MoveTypeLayout,
    vm_status::sub_status::NFE_BCS_SERIALIZATION_FAILURE,
};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    value_serde::serialized_size_allowing_delayed_values,
    values::{values_impl::Reference, Value},
};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

// Processes arguments passed to the BCS functions, e.g., to_bytes or serialized_size.
// Returns the value to serialize, and its layout.
fn process_bcs_args(
    context: &mut SafeNativeContext,
    mut ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<(Value, MoveTypeLayout)> {
    debug_assert!(ty_args.len() == 1);
    debug_assert!(args.len() == 1);

    let ref_to_val = safely_pop_arg!(args, Reference);
    let arg_type = ty_args.pop().unwrap();

    let layout = match context.type_to_type_layout(&arg_type) {
        Ok(layout) => layout,
        Err(_) => {
            context.charge(BCS_TO_BYTES_FAILURE)?;
            return Err(SafeNativeError::Abort {
                abort_code: NFE_BCS_SERIALIZATION_FAILURE,
            });
        },
    };

    let val = ref_to_val.read_ref()?;
    Ok((val, layout))
}

/***************************************************************************************************
 * native fun to_bytes
 *
 *   gas cost: size_of(val_type) * input_unit_cost +        | get type layout
 *             size_of(val) * input_unit_cost +             | serialize value
 *             max(size_of(output), 1) * output_unit_cost
 *
 *             If any of the first two steps fails, a partial cost + an additional failure_cost
 *             will be charged.
 *
 **************************************************************************************************/
/// Rust implementation of Move's `native public fun to_bytes<T>(&T): vector<u8>`
#[inline]
fn native_to_bytes(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let (val, layout) = process_bcs_args(context, ty_args, args)?;
    let serialized_value = match val.simple_serialize(&layout) {
        Some(serialized_value) => serialized_value,
        None => {
            context.charge(BCS_TO_BYTES_FAILURE)?;
            return Err(SafeNativeError::Abort {
                abort_code: NFE_BCS_SERIALIZATION_FAILURE,
            });
        },
    };
    context
        .charge(BCS_TO_BYTES_PER_BYTE_SERIALIZED * NumBytes::new(serialized_value.len() as u64))?;

    Ok(smallvec![Value::vector_u8(serialized_value)])
}

/***************************************************************************************************
 * native fun serialized_size
 *
 *   gas cost: size_of(output)
 *
 *   If the getting the type layout or serialization results in error, a special failure
 *   cost is charged.
 *
 **************************************************************************************************/
fn native_serialized_size(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let (val, layout) = process_bcs_args(context, ty_args, args)?;

    // Note:
    // This function reuses the same gas costs as bcs::to_bytes. We might want to
    // consider to charge less if
    //   1) calculation of serialized size changes, or
    //   2) the final gas cost (it is u64, not vector like in bcs::to_bytes).
    // Charging for failure during layout construction can stay the same.
    let serialized_size = match serialized_size_allowing_delayed_values(&val, &layout) {
        Some(serialized_size) => serialized_size as u64,
        None => {
            context.charge(BCS_TO_BYTES_FAILURE)?;
            return Err(SafeNativeError::Abort {
                abort_code: NFE_BCS_SERIALIZATION_FAILURE,
            });
        },
    };
    context.charge(BCS_TO_BYTES_PER_BYTE_SERIALIZED * NumBytes::new(serialized_size))?;

    Ok(smallvec![Value::u64(serialized_size)])
}

/***************************************************************************************************
 * module
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let funcs = [
        ("to_bytes", native_to_bytes as RawSafeNative),
        ("serialized_size", native_serialized_size as RawSafeNative),
    ];

    builder.make_named_natives(funcs)
}
