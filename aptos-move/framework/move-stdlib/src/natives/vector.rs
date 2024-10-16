// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Implementation of native functions for utf8 strings.

use aptos_gas_schedule::gas_params::natives::move_stdlib::{
    VECTOR_RANGE_MOVE_BASE, VECTOR_RANGE_MOVE_PER_INDEX_MOVED,
};
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
use move_core_types::gas_algebra::NumArgs;
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Value, VectorRef},
};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

use super::mem::get_feature_not_available_error;

/// The generic type supplied to aggregator snapshots is not supported.
pub const EINDEX_OUT_OF_BOUNDS: u64 = 0x03_0001;

/***************************************************************************************************
 * native fun range_move<T>(from: &mut vector<T>, removal_position: u64, length: u64, to: &mut vector<T>, insert_position: u64)
 *
 *   gas cost: VECTOR_RANGE_MOVE_BASE + VECTOR_RANGE_MOVE_PER_INDEX_MOVED * num_elements_to_move
 *
 **************************************************************************************************/
fn native_range_move(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    if !context.get_feature_flags().is_native_memory_operations_enabled() {
        return Err(get_feature_not_available_error());
    }

    context.charge(VECTOR_RANGE_MOVE_BASE)?;

    let map_err = |_| SafeNativeError::Abort {
        abort_code: EINDEX_OUT_OF_BOUNDS,
    };
    let insert_position = usize::try_from(safely_pop_arg!(args, u64)).map_err(map_err)?;
    let to = safely_pop_arg!(args, VectorRef);
    let length = usize::try_from(safely_pop_arg!(args, u64)).map_err(map_err)?;
    let removal_position = usize::try_from(safely_pop_arg!(args, u64)).map_err(map_err)?;
    let from = safely_pop_arg!(args, VectorRef);

    // need to fetch sizes here to charge upfront, and later in move_range, not sure if possible to combine
    let to_len = to.len_usize_raw(&ty_args[0])?;
    let from_len = from.len_usize_raw(&ty_args[0])?;

    if removal_position
        .checked_add(length)
        .map_or(true, |end| end > from_len)
        || insert_position > to_len
    {
        return Err(SafeNativeError::Abort {
            abort_code: EINDEX_OUT_OF_BOUNDS,
        });
    }

    // We are moving all elements in the range, all elements after range, and all elements after insertion point.
    // We are counting "length" of moving block twice, as it both gets moved out and moved in.
    context.charge(
        VECTOR_RANGE_MOVE_PER_INDEX_MOVED
            * NumArgs::new(
                (from_len - removal_position)
                    .checked_add(to_len - insert_position)
                    .and_then(|v| v.checked_add(length))
                    .ok_or_else(|| SafeNativeError::Abort {
                        abort_code: EINDEX_OUT_OF_BOUNDS,
                    })? as u64,
            ),
    )?;

    from.move_range(removal_position, length, &to, insert_position, &ty_args[0])?;

    Ok(smallvec![])
}

/***************************************************************************************************
 * module
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [("range_move", native_range_move as RawSafeNative)];

    builder.make_named_natives(natives)
}
