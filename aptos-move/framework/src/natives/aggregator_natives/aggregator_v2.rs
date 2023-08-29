// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::natives::{
    aggregator_natives::helpers_v2::{
        aggregator_snapshot_string_info, aggregator_snapshot_u128_info,
        aggregator_snapshot_u64_info, string_value_info,
    },
    AccountAddress,
};
use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeResult,
};
use aptos_types::vm_status::StatusCode;
use move_binary_format::errors::PartialVMError;
use move_core_types::value::{MoveStructLayout, MoveTypeLayout};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Struct, StructRef, Value},
};
use smallvec::{smallvec, SmallVec};
use std::{collections::VecDeque, ops::Deref};

/// Checks if the type argument `type_arg` is a string type
fn is_string_type(context: &SafeNativeContext, type_arg: &Type) -> SafeNativeResult<bool> {
    let ty = context.deref().type_to_fully_annotated_layout(type_arg)?;
    if let MoveTypeLayout::Struct(MoveStructLayout::WithTypes { type_, .. }) = ty {
        if type_.name.as_str() == "String"
            && type_.module.as_str() == "string"
            && type_.address == AccountAddress::ONE
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/***************************************************************************************************
 * native fun create_snapshot(value: Element): AggregatorSnapshot<Element>;
 **************************************************************************************************/

fn native_create_snapshot(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(ty_args.len(), 1);
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
        _ => {
            // Check if the type is a string
            if is_string_type(context, &ty_args[0])? {
                let input = string_value_info(safely_pop_arg!(args, Struct))?;
                let move_string_value = Value::struct_(Struct::pack(vec![Value::vector_u8(input)]));
                let move_snapshot_value = Value::struct_(Struct::pack(vec![move_string_value]));
                return Ok(smallvec![move_snapshot_value]);
            }
            // If not a string, return an error
            Err(PartialVMError::new(StatusCode::ABORTED)
                .with_message("Unsupported type supplied to aggregator snapshot".to_string())
                .with_sub_status(0x02_0005)
                .into())
        },
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
    debug_assert_eq!(ty_args.len(), 1);
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
        _ => {
            // Check if the type is a string
            if is_string_type(context, &ty_args[0])? {
                let value = aggregator_snapshot_string_info(&safely_pop_arg!(args, StructRef))?;
                let move_string_value = Value::struct_(Struct::pack(vec![Value::vector_u8(value)]));
                let move_snapshot_value = Value::struct_(Struct::pack(vec![move_string_value]));
                return Ok(smallvec![move_snapshot_value]);
            }
            // If not a string, return an error
            Err(PartialVMError::new(StatusCode::ABORTED)
                .with_message("Unsupported type supplied to aggregator snapshot".to_string())
                .with_sub_status(0x02_0005)
                .into())
        },
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
    debug_assert_eq!(ty_args.len(), 1);
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
        _ => {
            // Check if the type is a string
            if is_string_type(context, &ty_args[0])? {
                let value = aggregator_snapshot_string_info(&safely_pop_arg!(args, StructRef))?;
                let move_string_value = Value::struct_(Struct::pack(vec![Value::vector_u8(value)]));
                return Ok(smallvec![move_string_value]);
            }
            // If not a string, return an error
            Err(PartialVMError::new(StatusCode::ABORTED)
                .with_message("Unsupported type supplied to aggregator snapshot".to_string())
                .with_sub_status(0x02_0005)
                .into())
        },
    }
}

/***************************************************************************************************
 * native fun native fun string_concat<Element>(before: String, snapshot: &AggregatorSnapshot<Element>, after: String): AggregatorSnapshot<String>;
 **************************************************************************************************/

fn native_string_concat(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(ty_args.len(), 1);
    debug_assert_eq!(args.len(), 3);
    context.charge(AGGREGATOR_V2_STRING_CONCAT_BASE)?;
    // Check if the type is a string
    if is_string_type(context, &ty_args[0])? {
        let after = string_value_info(safely_pop_arg!(args, Struct))?;
        let snapshot_value = aggregator_snapshot_string_info(&safely_pop_arg!(args, StructRef))?;
        let before = string_value_info(safely_pop_arg!(args, Struct))?;
        let mut result = before.clone();
        result.extend(&snapshot_value);
        result.extend(&after);
        let move_string_value = Value::struct_(Struct::pack(vec![Value::vector_u8(result)]));
        let move_snapshot_value = Value::struct_(Struct::pack(vec![move_string_value]));
        return Ok(smallvec![move_snapshot_value]);
    }
    // If not a string, return an error
    Err(PartialVMError::new(StatusCode::ABORTED)
        .with_message("Unsupported type supplied to aggregator snapshot".to_string())
        .with_sub_status(0x02_0005)
        .into())
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
        ("string_concat", native_string_concat),
    ];
    builder.make_named_natives(natives)
}
