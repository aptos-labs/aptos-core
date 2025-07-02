// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
use move_core_types::gas_algebra::NumBytes;
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type, value_serde::ValueSerDeContext, values::Value,
};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

// !!!! NOTE !!!!
// This file is intended for natives from the util module in the framework.
// DO NOT PUT HELPER FUNCTIONS HERE!

/// Abort code when from_bytes fails (0x01 == INVALID_ARGUMENT)
const EFROM_BYTES: u64 = 0x01_0001;

/***************************************************************************************************
 * native fun from_bytes
 *
 *   gas cost: base_cost + unit_cost * bytes_len
 *
 **************************************************************************************************/
fn native_from_bytes(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(ty_args.len(), 1);
    debug_assert_eq!(args.len(), 1);

    // TODO(Gas): charge for getting the layout
    let layout = context
        .type_to_type_layout_with_delayed_fields(&ty_args[0])?
        .unpack()
        .0;
    let bytes = safely_pop_arg!(args, Vec<u8>);
    context.charge(
        UTIL_FROM_BYTES_BASE + UTIL_FROM_BYTES_PER_BYTE * NumBytes::new(bytes.len() as u64),
    )?;

    let function_value_extension = context.function_value_extension();
    let val = match ValueSerDeContext::new()
        .with_legacy_signer()
        .with_func_args_deserialization(&function_value_extension)
        .deserialize(&bytes, &layout)
    {
        Some(val) => val,
        None => {
            return Err(SafeNativeError::Abort {
                abort_code: EFROM_BYTES,
            })
        },
    };

    Ok(smallvec![val])
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [("from_bytes", native_from_bytes as RawSafeNative)];

    builder.make_named_natives(natives)
}
