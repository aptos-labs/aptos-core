// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Implementation of native functions for utf8 strings.

use aptos_gas_schedule::gas_params::natives::move_stdlib::*;
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeResult,
};
use move_core_types::gas_algebra::NumBytes;
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Value, VectorRef},
};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

// The implementation approach delegates all utf8 handling to Rust.
// This is possible without copying of bytes because (a) we can
// get a `std::cell::Ref<Vec<u8>>` from a `vector<u8>` and in turn a `&[u8]`
// from that (b) assuming that `vector<u8>` embedded in a string
// is already valid utf8, we can use `str::from_utf8_unchecked` to
// create a `&str` view on the bytes without a copy. Once we have this
// view, we can call ut8 functions like length, substring, etc.

/***************************************************************************************************
 * native fun internal_check_utf8
 *
 *   gas cost: base_cost + unit_cost * length_in_bytes
 *
 **************************************************************************************************/
fn native_check_utf8(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(args.len() == 1);
    let s_arg = safely_pop_arg!(args, VectorRef);
    let s_ref = s_arg.as_bytes_ref();

    context.charge(
        STRING_CHECK_UTF8_BASE + STRING_CHECK_UTF8_PER_BYTE * NumBytes::new(s_ref.len() as u64),
    )?;

    let ok = std::str::from_utf8(s_ref).is_ok();
    // TODO: extensible native cost tables

    Ok(smallvec![Value::bool(ok)])
}

/***************************************************************************************************
 * native fun internal_is_char_boundary
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
fn native_is_char_boundary(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(args.len() == 2);

    context.charge(STRING_IS_CHAR_BOUNDARY_BASE)?;

    let i = safely_pop_arg!(args, u64);
    let s_arg = safely_pop_arg!(args, VectorRef);
    let s_ref = s_arg.as_bytes_ref();
    let ok = unsafe {
        // This is safe because we guarantee the bytes to be utf8.
        std::str::from_utf8_unchecked(s_ref).is_char_boundary(i as usize)
    };

    Ok(smallvec![Value::bool(ok)])
}

/***************************************************************************************************
 * native fun internal_sub_string
 *
 *   gas cost: base_cost + unit_cost * sub_string_length_in_bytes
 *
 **************************************************************************************************/
fn native_sub_string(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(args.len() == 3);

    context.charge(STRING_SUB_STRING_BASE)?;

    let j = safely_pop_arg!(args, u64) as usize;
    let i = safely_pop_arg!(args, u64) as usize;

    if j < i {
        // TODO: The abort code should follow the error convention.
        return Err(aptos_native_interface::SafeNativeError::Abort { abort_code: 1 });
    }

    context.charge(STRING_SUB_STRING_PER_BYTE * NumBytes::new((j - i) as u64))?;

    let s_arg = safely_pop_arg!(args, VectorRef);
    let s_ref = s_arg.as_bytes_ref();
    let s_str = unsafe {
        // This is safe because we guarantee the bytes to be utf8.
        std::str::from_utf8_unchecked(s_ref)
    };
    let v = Value::vector_u8(s_str[i..j].as_bytes().iter().cloned());

    Ok(smallvec![v])
}

/***************************************************************************************************
 * native fun internal_index_of
 *
 *   gas cost: base_cost + unit_cost * bytes_searched
 *
 **************************************************************************************************/
fn native_index_of(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(args.len() == 2);

    context.charge(STRING_INDEX_OF_BASE)?;

    let r_arg = safely_pop_arg!(args, VectorRef);
    let r_ref = r_arg.as_bytes_ref();
    let r_str = unsafe { std::str::from_utf8_unchecked(r_ref) };

    context.charge(STRING_INDEX_OF_PER_BYTE_PATTERN * NumBytes::new(r_str.len() as u64))?;

    let s_arg = safely_pop_arg!(args, VectorRef);
    let s_ref = s_arg.as_bytes_ref();
    let s_str = unsafe { std::str::from_utf8_unchecked(s_ref) };
    let pos = match s_str.find(r_str) {
        Some(size) => size,
        None => s_str.len(),
    };

    // TODO(Gas): What is the algorithm used for the search?
    //            Ideally it should be something like KMP with O(n) time complexity...
    context.charge(STRING_INDEX_OF_PER_BYTE_SEARCHED * NumBytes::new(pos as u64))?;

    Ok(smallvec![Value::u64(pos as u64)])
}

/***************************************************************************************************
 * module
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [
        ("internal_check_utf8", native_check_utf8 as RawSafeNative),
        ("internal_is_char_boundary", native_is_char_boundary),
        ("internal_sub_string", native_sub_string),
        ("internal_index_of", native_index_of),
    ];

    builder.make_named_natives(natives)
}
