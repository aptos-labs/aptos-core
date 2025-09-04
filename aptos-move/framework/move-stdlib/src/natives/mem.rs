// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Implementation of native functions for memory manipulation.

use aptos_gas_schedule::gas_params::natives::move_stdlib::MEM_SWAP_BASE;
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
use aptos_types::error;
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Reference, Value},
};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

/// The feature is not enabled.
pub const EFEATURE_NOT_ENABLED: u64 = 1;

/***************************************************************************************************
 * native fun native_swap
 *
 *   gas cost: MEM_SWAP_BASE
 *
 **************************************************************************************************/
fn native_swap(
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

    debug_assert!(args.len() == 2);

    context.charge(MEM_SWAP_BASE)?;

    let left = safely_pop_arg!(args, Reference);
    let right = safely_pop_arg!(args, Reference);

    // TODO: this does not provide very strong invariants, revisit.
    context.data_cache().copy_on_write(&left)?;
    context.data_cache().copy_on_write(&right)?;
    left.swap_values(right)?;

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
