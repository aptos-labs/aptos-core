// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::natives::string_utils::native_format_debug;
use velor_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
use move_vm_runtime::native_functions::NativeFunction;
#[allow(unused_imports)]
use move_vm_types::{
    loaded_data::runtime_types::Type,
    natives::function::NativeResult,
    values::{Reference, Struct, Value},
};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

/***************************************************************************************************
 * native fun print
 *
 **************************************************************************************************/
#[inline]
fn native_print(
    _: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(ty_args.is_empty());
    debug_assert!(args.len() == 1);

    if cfg!(feature = "testing") {
        let val = safely_pop_arg!(args, Struct);
        let bytes = val.unpack()?.next().unwrap();

        println!(
            "[debug] {}",
            std::str::from_utf8(&bytes.value_as::<Vec<u8>>()?).unwrap()
        );
    }

    Ok(smallvec![])
}

/***************************************************************************************************
 * native fun print_stack_trace
 *
 **************************************************************************************************/
#[allow(unused_variables)]
#[inline]
fn native_stack_trace(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(ty_args.is_empty());
    debug_assert!(args.is_empty());

    let mut s = String::new();

    if cfg!(feature = "testing") {
        context.print_stack_trace(&mut s)?;
    }

    let move_str = Value::struct_(Struct::pack(vec![Value::vector_u8(s.into_bytes())]));
    Ok(smallvec![move_str])
}

#[inline]
fn native_old_debug_print(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    if cfg!(feature = "testing") {
        let x = safely_pop_arg!(args, Reference);
        let val = x.read_ref().map_err(SafeNativeError::InvariantViolation)?;

        println!(
            "[debug] {}",
            native_format_debug(context, &ty_args[0], val)?
        );
    }
    Ok(smallvec![])
}

#[inline]
fn native_old_print_stacktrace(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(ty_args.is_empty());
    debug_assert!(args.is_empty());

    if cfg!(feature = "testing") {
        let mut s = String::new();
        context.print_stack_trace(&mut s)?;
        println!("{}", s);
    }
    Ok(smallvec![])
}

/***************************************************************************************************
 * module
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [
        ("native_print", native_print as RawSafeNative),
        ("native_stack_trace", native_stack_trace),
        // For re-playability on-chain we still implement the old versions of these functions
        ("print", native_old_debug_print),
        ("print_stack_trace", native_old_print_stacktrace),
    ];

    builder.make_named_natives(natives)
}
