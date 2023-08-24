// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::natives::aggregator_natives::helpers_v2::{
    aggregator_snapshot_u128_info, aggregator_snapshot_u64_info,
};
use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeResult,
};
use aptos_types::vm_status::StatusCode;
use move_binary_format::errors::PartialVMError;
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Struct, StructRef, Value},
};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

/***************************************************************************************************
 * native fun create_snapshot(value: Element): AggregatorSnapshot<Element>;
 **************************************************************************************************/

fn native_create_snapshot(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 1);
    context.charge(AGGREGATOR_V2_CREATE_SNAPSHOT_BASE)?;

    match ty_args[0] {
        Type::U128 => {
            let input = safely_pop_arg!(args, u128);
            Ok(smallvec![Value::struct_(Struct::pack(vec![Value::u128(
                input
            )]))])
        },
        Type::U64 => {
            let input = safely_pop_arg!(args, u64);
            Ok(smallvec![Value::struct_(Struct::pack(vec![Value::u64(
                input
            )]))])
        },
        _ => Err(PartialVMError::new(StatusCode::ABORTED)
            .with_message("Unsupported type supplied to aggregator".to_string())
            .with_sub_status(0x02_0005)
            .into()),
    }
}

/***************************************************************************************************
 * native fun copy_snapshot(snapshot: AggregatorSnapshot<Element>): AggregatorSnapshot<Element>;
 **************************************************************************************************/

fn native_copy_snapshot(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 1);
    context.charge(AGGREGATOR_V2_COPY_SNAPSHOT_BASE)?;

    match ty_args[0] {
        Type::U128 => {
            let value = aggregator_snapshot_u128_info(&safely_pop_arg!(args, StructRef))?;
            Ok(smallvec![Value::struct_(Struct::pack(vec![Value::u128(
                value
            )]))])
        },
        Type::U64 => {
            let value = aggregator_snapshot_u64_info(&safely_pop_arg!(args, StructRef))?;
            Ok(smallvec![Value::struct_(Struct::pack(vec![Value::u64(
                value
            )]))])
        },
        _ => Err(PartialVMError::new(StatusCode::ABORTED)
            .with_message("Unsupported type supplied to aggregator".to_string())
            .with_sub_status(0x02_0005)
            .into()),
    }
}

/***************************************************************************************************
 * native fun read_snapshot(snapshot: AggregatorSnapshot<Element>): Element;
 **************************************************************************************************/

fn native_read_snapshot(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 1);
    context.charge(AGGREGATOR_V2_READ_SNAPSHOT_BASE)?;

    match ty_args[0] {
        Type::U128 => {
            let value = aggregator_snapshot_u128_info(&safely_pop_arg!(args, StructRef))?;
            Ok(smallvec![Value::u128(value)])
        },
        Type::U64 => {
            let value = aggregator_snapshot_u64_info(&safely_pop_arg!(args, StructRef))?;
            Ok(smallvec![Value::u64(value)])
        },
        _ => Err(PartialVMError::new(StatusCode::ABORTED)
            .with_message("Unsupported type supplied to aggregator".to_string())
            .with_sub_status(0x02_0005)
            .into()),
    }
}

/***************************************************************************************************
 * module
 **************************************************************************************************/

pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [
        ("create_snapshot", native_create_snapshot as RawSafeNative),
        ("copy_snapshot", native_copy_snapshot),
        ("read_snapshot", native_read_snapshot),
    ];
    builder.make_named_natives(natives)
}
