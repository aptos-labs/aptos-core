// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Implementation of native functions (non-bytecode instructions) for vector.

use aptos_gas_schedule::gas_params::natives::move_stdlib::{
    VECTOR_MOVE_RANGE_BASE, VECTOR_MOVE_RANGE_PER_INDEX_MOVED,
};
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
use aptos_types::error;
use move_core_types::gas_algebra::NumArgs;
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Value, VectorRef},
};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

/// Given input positions/lengths are outside of vector boundaries.
pub const EINDEX_OUT_OF_BOUNDS: u64 = 1;

/// The feature is not enabled.
pub const EFEATURE_NOT_ENABLED: u64 = 2;

/***************************************************************************************************
 * native fun move_range<T>(from: &mut vector<T>, removal_position: u64, length: u64, to: &mut vector<T>, insert_position: u64)
 *
 *   gas cost: VECTOR_MOVE_RANGE_BASE + VECTOR_MOVE_RANGE_PER_INDEX_MOVED * num_elements_to_move
 *
 **************************************************************************************************/
fn native_move_range(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    if !context
        .get_feature_flags()
        .is_native_memory_operations_enabled()
    {
        return Err(SafeNativeError::Abort {
            abort_code: error::unavailable(EFEATURE_NOT_ENABLED),
        });
    }

    context.charge(VECTOR_MOVE_RANGE_BASE)?;

    let map_err = |_| SafeNativeError::Abort {
        abort_code: error::invalid_argument(EINDEX_OUT_OF_BOUNDS),
    };
    let insert_position = usize::try_from(safely_pop_arg!(args, u64)).map_err(map_err)?;
    let to = safely_pop_arg!(args, VectorRef);
    let length = usize::try_from(safely_pop_arg!(args, u64)).map_err(map_err)?;
    let removal_position = usize::try_from(safely_pop_arg!(args, u64)).map_err(map_err)?;
    let from = safely_pop_arg!(args, VectorRef);

    // We need to charge before executing, so fetching and checking sizes here.
    // We repeat fetching and checking of the sizes inside VectorRef::move_range call as well.
    // Not sure if possible to combine (as we are never doing charging there).
    let to_len = to.length_as_usize()?;
    let from_len = from.length_as_usize()?;

    if removal_position
        .checked_add(length)
        .is_none_or(|end| end > from_len)
        || insert_position > to_len
    {
        return Err(SafeNativeError::Abort {
            abort_code: EINDEX_OUT_OF_BOUNDS,
        });
    }

    // We are moving all elements in the range, all elements after range, and all elements after insertion point.
    // We are counting "length" of moving block twice, as it both gets moved out and moved in.
    // From calibration testing, this seems to be a reasonable approximation of the cost of the operation.
    context.charge(
        VECTOR_MOVE_RANGE_PER_INDEX_MOVED
            * NumArgs::new(
                (from_len - removal_position)
                    .checked_add(to_len - insert_position)
                    .and_then(|v| v.checked_add(length))
                    .ok_or_else(|| SafeNativeError::Abort {
                        abort_code: EINDEX_OUT_OF_BOUNDS,
                    })? as u64,
            ),
    )?;

    VectorRef::move_range(
        &from,
        removal_position,
        length,
        &to,
        insert_position,
        // &ty_args[0],
    )?;

    Ok(smallvec![])
}

/***************************************************************************************************
 * module
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [("move_range", native_move_range as RawSafeNative)];

    builder.make_named_natives(natives)
}
