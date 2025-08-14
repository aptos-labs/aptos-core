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
    loaded_data::runtime_types::Type, value_serde::ValueSerDeContext, values::{Reference, StructRef, Value, VectorRef},
};
use smallvec::{smallvec, SmallVec};
use std::{cmp::Ordering, collections::VecDeque};

/***************************************************************************************************
 * native fun binary_search_impl<K, V>(key: &K, entries: &vector<Entry<K, V>>, start: u64, end: u64): u64
 *
 *   gas cost: base_cost + unit_cost * bytes_len
 *
 **************************************************************************************************/
fn native_binary_search(
    _context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 4);


    let end = safely_pop_arg!(args, u64);
    let start = safely_pop_arg!(args, u64);
    let entries = safely_pop_arg!(args, VectorRef);
    // let key = safely_pop_arg!(args, Reference);

    let key = args.pop_back().unwrap();

    let mut l = start;
    let mut r = end;
    while l != r {
        let mid = l + ((r - l) >> 1);

        let mid_elem = entries.borrow_elem_unchecked(mid as usize)?.value_as::<StructRef>()?;

        let mid_key = mid_elem.borrow_field(0)?;


        let comparison = mid_key.compare(&key)?;
        match comparison {
            Ordering::Less => l = mid + 1,
            // std::cmp::Ordering::Equal => return Ok(smallvec![Value::u64(mid)]),
            Ordering::Equal | Ordering::Greater => r = mid,
        }


        // if (comparison.is_lt()) {
        //     l = mid + 1;
        // } else {
        //     r = mid;
        // };
    };


    Ok(smallvec![Value::u64(l)])




    // // TODO(Gas): charge for getting the layout
    // let layout = context.type_to_type_layout(&ty_args[0])?;

    // let bytes = safely_pop_arg!(args, Vec<u8>);
    // context.charge(
    //     UTIL_FROM_BYTES_BASE + UTIL_FROM_BYTES_PER_BYTE * NumBytes::new(bytes.len() as u64),
    // )?;

    // let function_value_extension = context.function_value_extension();
    // let max_value_nest_depth = context.max_value_nest_depth();
    // let val = match ValueSerDeContext::new(max_value_nest_depth)
    //     .with_legacy_signer()
    //     .with_func_args_deserialization(&function_value_extension)
    //     .deserialize(&bytes, &layout)
    // {
    //     Some(val) => val,
    //     None => {
    //         return Err(SafeNativeError::Abort {
    //             abort_code: EFROM_BYTES,
    //         })
    //     },
    // };

    // Ok(smallvec![val])
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [("binary_search_impl", native_binary_search as RawSafeNative),
    ("binary_search", native_binary_search)];

    builder.make_named_natives(natives)
}
