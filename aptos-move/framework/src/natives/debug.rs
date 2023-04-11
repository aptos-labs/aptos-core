// Copyright © Aptos Foundation

// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::natives::helpers::make_module_natives;
use move_binary_format::errors::PartialVMResult;
use move_vm_runtime::native_functions::{NativeContext, NativeFunction};
#[allow(unused_imports)]
use move_vm_types::{
    loaded_data::runtime_types::Type,
    natives::function::NativeResult,
    pop_arg,
    values::{Reference, Struct, Value},
};
use smallvec::smallvec;
use std::{collections::VecDeque, sync::Arc};

/***************************************************************************************************
 * native fun print
 *
 **************************************************************************************************/
#[inline]
fn native_print(ty_args: Vec<Type>, mut args: VecDeque<Value>) -> PartialVMResult<NativeResult> {
    debug_assert!(ty_args.is_empty());
    debug_assert!(args.len() == 1);

    if cfg!(feature = "testing") {
        let val = pop_arg!(args, Struct);
        let bytes = val.unpack()?.next().unwrap();

        println!(
            "[debug] {}",
            std::str::from_utf8(&bytes.value_as::<Vec<u8>>()?).unwrap()
        );
    }

    Ok(NativeResult::ok(0.into(), smallvec![]))
}

pub fn make_native_print() -> NativeFunction {
    Arc::new(
        move |_context, ty_args, args| -> PartialVMResult<NativeResult> {
            native_print(ty_args, args)
        },
    )
}

/***************************************************************************************************
 * native fun print_stack_trace
 *
 **************************************************************************************************/
#[allow(unused_variables)]
#[inline]
fn native_stack_trace(
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(ty_args.is_empty());
    debug_assert!(args.is_empty());

    let mut s = String::new();

    if cfg!(feature = "testing") {
        context.print_stack_trace(&mut s)?;
    }

    let move_str = Value::struct_(Struct::pack(vec![Value::vector_u8(s.into_bytes())]));
    Ok(NativeResult::ok(0.into(), smallvec![move_str]))
}

pub fn make_native_stack_trace() -> NativeFunction {
    Arc::new(
        move |context, ty_args, args| -> PartialVMResult<NativeResult> {
            native_stack_trace(context, ty_args, args)
        },
    )
}

pub fn make_dummy() -> NativeFunction {
    Arc::new(
        move |_context, _ty_args, _args| -> PartialVMResult<NativeResult> {
            Ok(NativeResult::ok(0.into(), smallvec![]))
        },
    )
}

/***************************************************************************************************
 * module
 **************************************************************************************************/
pub fn make_all() -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [
        ("native_print", make_native_print()),
        ("native_stack_trace", make_native_stack_trace()),
        // For replayability on-chain we need dummy implementations of these functions
        ("print", make_dummy()),
        ("print_stack_trace", make_dummy()),
    ];

    make_module_natives(natives)
}
