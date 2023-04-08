// Copyright © Aptos Foundation

// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::natives::helpers::make_module_natives;
use move_binary_format::errors::PartialVMResult;
use move_core_types::gas_algebra::InternalGas;
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
 *   gas cost: base_cost
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct PrintGasParameters {
    pub base_cost: InternalGas,
}

#[inline]
fn native_print(
    gas_params: &PrintGasParameters,
    _context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(ty_args.len() == 0);
    debug_assert!(args.len() == 1);

    // No-op if the feature flag is not present.
    #[cfg(feature = "testing")]
    {
        let val = pop_arg!(args, Struct);
        let bytes = val.unpack()?.next().unwrap();

        println!(
            "[debug] {}",
            std::str::from_utf8(&bytes.value_as::<Vec<u8>>()?).unwrap()
        );
    }

    Ok(NativeResult::ok(gas_params.base_cost, smallvec![]))
}

pub fn make_native_print(gas_params: PrintGasParameters) -> NativeFunction {
    Arc::new(
        move |context, ty_args, args| -> PartialVMResult<NativeResult> {
            native_print(&gas_params, context, ty_args, args)
        },
    )
}

/***************************************************************************************************
 * native fun print_stack_trace
 *
 *   gas cost: base_cost
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct StackTraceGasParameters {
    pub base_cost: InternalGas,
}

#[allow(unused_variables)]
#[inline]
fn native_stack_trace(
    gas_params: &StackTraceGasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(ty_args.is_empty());
    debug_assert!(args.is_empty());

    let mut s = String::new();

    #[cfg(feature = "testing")]
    {
        context.print_stack_trace(&mut s)?;
    }

    let move_str = Value::struct_(Struct::pack(vec![Value::vector_u8(s.into_bytes())]));
    Ok(NativeResult::ok(gas_params.base_cost, smallvec![move_str]))
}

pub fn make_native_stack_trace(gas_params: StackTraceGasParameters) -> NativeFunction {
    Arc::new(
        move |context, ty_args, args| -> PartialVMResult<NativeResult> {
            native_stack_trace(&gas_params, context, ty_args, args)
        },
    )
}

/***************************************************************************************************
 * module
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct GasParameters {
    pub native_print: PrintGasParameters,
    pub native_stack_trace: StackTraceGasParameters,
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [
        ("native_print", make_native_print(gas_params.native_print)),
        (
            "native_stack_trace",
            make_native_stack_trace(gas_params.native_stack_trace),
        ),
    ];

    make_module_natives(natives)
}
