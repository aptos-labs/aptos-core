// Copyright © Supra
// SPDX-License-Identifier: Apache-2.0

use aptos_gas_schedule::gas_params::natives::aptos_framework::VECTOR_UTILS_PER_SWAP;
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeResult,
};
use move_core_types::gas_algebra::NumBytes;
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

/***************************************************************************************************
* native fun sort vector by key in ascending order.

* accepts 2 vectors one is the key of u64 type based on which sorting is done, the other one is the values
* in the original vector to be sorted.
*
**************************************************************************************************/
fn native_sort_vector_u64_by_key(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 2);

    let keys: Vec<u64> = safely_pop_arg!(args, Vec<u64>);
    let values: Vec<u64> = safely_pop_arg!(args, Vec<u64>);

    let complexity = (((values.len() as f64).log2() + 1.0) as usize) * values.len();
    context.charge(VECTOR_UTILS_PER_SWAP * NumBytes::new(complexity as u64))?;

    let mut pairs = keys.into_iter().zip(values).collect::<Vec<_>>();
    pairs.sort_by_key(|(k, _v)| *k);
    let result = pairs.into_iter().map(|(_, v)| v).collect::<Vec<_>>();

    let result = Value::vector_u64(result);
    Ok(smallvec![result])
}

/***************************************************************************************************
* native fun sort vector of u64 in ascending order.

* accepts a vector of u64 values and returns sorted version in ascending order.
*
**************************************************************************************************/
fn native_sort_vector_u64(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 1);
    let mut values: Vec<u64> = safely_pop_arg!(args, Vec<u64>);
    let complexity = (((values.len() as f64).log2() + 1.0) as usize) * values.len();
    context.charge(VECTOR_UTILS_PER_SWAP * NumBytes::new(complexity as u64))?;
    values.sort();
    let result = Value::vector_u64(values);
    Ok(smallvec![result])
}
/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [
        (
            "native_sort_vector_u64_by_key",
            native_sort_vector_u64_by_key as RawSafeNative,
        ),
        (
            "native_sort_vector_u64",
            native_sort_vector_u64 as RawSafeNative,
        ),
    ];

    builder.make_named_natives(natives)
}
