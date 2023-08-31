// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::aggregator_extension::AggregatorID;
use aptos_types::vm_status::StatusCode;
use move_binary_format::errors::PartialVMError;
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Struct, StructRef, Value},
};
use smallvec::{smallvec, SmallVec};
use crate::natives::{
    aggregator_natives::{NativeAggregatorContext, helpers_v2::{
        aggregator_snapshot_value_as_bytes, aggregator_snapshot_value_as_u128,
        aggregator_snapshot_value_as_u64, string_to_bytes, aggregator_info_u128, aggregator_info_u64
    }},
    AccountAddress,
};
use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
use move_core_types::value::{MoveStructLayout, MoveTypeLayout};
use std::{collections::VecDeque, ops::Deref};


/***************************************************************************************************
 * native fun create_aggregator<Element>(limit: Element): Aggregator<Element>;
 **************************************************************************************************/

fn native_create_aggregator(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 1);

    context.charge(AGGREGATOR_V2_CREATE_AGGREGATOR_BASE)?;
    // Get the current aggregator data.
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut aggregator_data = aggregator_context.aggregator_data.borrow_mut();
    let id = AggregatorID::ephemeral(aggregator_data.generate_id());

    match ty_args[0] {
        Type::U128 => {
            let limit = safely_pop_arg!(args, u128);
            aggregator_data.create_new_aggregator(id, limit);
            Ok(smallvec![Value::struct_(Struct::pack(vec![
                Value::u128(0),
                Value::u128(limit),
            ]))])
        },
        Type::U64 => {
            let limit = safely_pop_arg!(args, u64);
            aggregator_data.create_new_aggregator(id, limit as u128);
            Ok(smallvec![Value::struct_(Struct::pack(vec![
                Value::u64(0),
                Value::u64(limit),
            ]))])
        },
        _ => Err(PartialVMError::new(StatusCode::ABORTED)
            .with_message("Unsupported type supplied to aggregator".to_string())
            .with_sub_status(0x02_0005)
            .into()),
    }
}

/***************************************************************************************************
 * native fun try_add<Element>(aggregator: &mut Aggregator<Element>, value: Element): bool;
 **************************************************************************************************/
fn native_try_add(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 2);

    context.charge(AGGREGATOR_V2_TRY_ADD_BASE)?;
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut aggregator_data = aggregator_context.aggregator_data.borrow_mut();

    match ty_args[0] {
        Type::U128 => {
            // Get aggregator information and a value to add.
            let value = safely_pop_arg!(args, u128);
            let (id, limit) = aggregator_info_u128(&safely_pop_arg!(args, StructRef))?;
            let aggregator = aggregator_data.get_aggregator(id, limit)?;
            Ok(smallvec![Value::bool(aggregator.try_add(value).is_ok())])
        },
        Type::U64 => {
            // Get aggregator information and a value to add.
            let value = safely_pop_arg!(args, u64);
            let (id, limit) = aggregator_info_u64(&safely_pop_arg!(args, StructRef))?;
            let aggregator = aggregator_data.get_aggregator(id, limit as u128)?;
            Ok(smallvec![Value::bool(
                aggregator.try_add(value as u128).is_ok()
            )])
        },
        _ => Err(PartialVMError::new(StatusCode::ABORTED)
            .with_message("Unsupported type supplied to aggregator".to_string())
            .with_sub_status(0x02_0005)
            .into()),
    }
}

/***************************************************************************************************
 * native fun try_sub<Element>(aggregator: &mut Aggregator<Element>, value: Element): bool;
 **************************************************************************************************/
fn native_try_sub(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 2);

    context.charge(AGGREGATOR_V2_TRY_SUB_BASE)?;
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut aggregator_data = aggregator_context.aggregator_data.borrow_mut();

    match ty_args[0] {
        Type::U128 => {
            // Get aggregator information and a value to subtract.
            let value = safely_pop_arg!(args, u128);
            let (id, limit) = aggregator_info_u128(&safely_pop_arg!(args, StructRef))?;
            let aggregator = aggregator_data.get_aggregator(id, limit)?;
            Ok(smallvec![Value::bool(aggregator.try_sub(value).is_ok())])
        },
        Type::U64 => {
            // Get aggregator information and a value to subtract.
            let value = safely_pop_arg!(args, u64);
            let (id, limit) = aggregator_info_u64(&safely_pop_arg!(args, StructRef))?;
            let aggregator = aggregator_data.get_aggregator(id, limit as u128)?;
            Ok(smallvec![Value::bool(
                aggregator.try_sub(value as u128).is_ok()
            )])
        },
        _ => Err(PartialVMError::new(StatusCode::ABORTED)
            .with_message("Unsupported type supplied to aggregator".to_string())
            .with_sub_status(0x02_0005)
            .into()),
    }
}

/***************************************************************************************************
 * native fun read<Element>(aggregator: &Aggregator<Element>): Element;
 **************************************************************************************************/

fn native_read(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 1);

    context.charge(AGGREGATOR_V2_READ_BASE)?;
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut aggregator_data = aggregator_context.aggregator_data.borrow_mut();

    match ty_args[0] {
        Type::U128 => {
            // Extract information from aggregator struct reference.
            let (id, limit) = aggregator_info_u128(&safely_pop_arg!(args, StructRef))?;
            let aggregator = aggregator_data.get_aggregator(id, limit)?;
            let value = aggregator.read_and_materialize(aggregator_context.resolver, &id)?;
            Ok(smallvec![Value::u128(value)])
        },
        Type::U64 => {
            let (id, limit) = aggregator_info_u64(&safely_pop_arg!(args, StructRef))?;
            let aggregator = aggregator_data.get_aggregator(id, limit as u128)?;
            let value = aggregator.read_and_materialize(aggregator_context.resolver, &id)?;
            if value > u64::MAX as u128 {
                return Err(PartialVMError::new(StatusCode::ABORTED)
                    .with_message("Aggregator<u64>::read() output exceeds u64::MAX".to_string())
                    .with_sub_status(0x02_0001)
                    .into());
            }
            Ok(smallvec![Value::u64(value as u64)])
        },
        _ => Err(PartialVMError::new(StatusCode::ABORTED)
            .with_message("Unsupported type supplied to aggregator".to_string())
            .with_sub_status(0x02_0005)
            .into()),
    }
}



/// The generic type supplied to aggregator snapshots is not supported.
pub const EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE: u64 = 0x03_0005;

/// The aggregator snapshots feature is not enabled.
pub const EAGGREGATOR_SNAPSHOTS_NOT_ENABLED: u64 = 0x03_0006;

/// Checks if the type argument `type_arg` is a string type.
fn is_string_type(context: &SafeNativeContext, type_arg: &Type) -> SafeNativeResult<bool> {
    let ty = context.deref().type_to_fully_annotated_layout(type_arg)?;
    if let MoveTypeLayout::Struct(MoveStructLayout::WithTypes { type_, .. }) = ty {
        return Ok(type_.name.as_str() == "String"
            && type_.module.as_str() == "string"
            && type_.address == AccountAddress::ONE);
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
    if !context.aggregator_snapshots_enabled() {
        return Err(SafeNativeError::Abort {
            abort_code: EAGGREGATOR_SNAPSHOTS_NOT_ENABLED,
        });
    }

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
                let input = string_to_bytes(safely_pop_arg!(args, Struct))?;
                let move_string_value = Value::struct_(Struct::pack(vec![Value::vector_u8(input)]));
                let move_snapshot_value = Value::struct_(Struct::pack(vec![move_string_value]));
                return Ok(smallvec![move_snapshot_value]);
            }
            // If not a string, return an error
            Err(SafeNativeError::Abort {
                abort_code: EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE,
            })
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
            let value = aggregator_snapshot_value_as_u128(&safely_pop_arg!(args, StructRef))?;
            Ok(smallvec![Value::struct_(Struct::pack(vec![Value::u128(
                value
            )]))])
        },
        Type::U64 => {
            let value = aggregator_snapshot_value_as_u64(&safely_pop_arg!(args, StructRef))?;
            Ok(smallvec![Value::struct_(Struct::pack(vec![Value::u64(
                value
            )]))])
        },
        _ => {
            // Check if the type is a string
            if is_string_type(context, &ty_args[0])? {
                let value = aggregator_snapshot_value_as_bytes(&safely_pop_arg!(args, StructRef))?;
                let move_string_value = Value::struct_(Struct::pack(vec![Value::vector_u8(value)]));
                let move_snapshot_value = Value::struct_(Struct::pack(vec![move_string_value]));
                return Ok(smallvec![move_snapshot_value]);
            }
            // If not a string, return an error
            Err(SafeNativeError::Abort {
                abort_code: EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE,
            })
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
            let value = aggregator_snapshot_value_as_u128(&safely_pop_arg!(args, StructRef))?;
            Ok(smallvec![Value::u128(value)])
        },
        Type::U64 => {
            let value = aggregator_snapshot_value_as_u64(&safely_pop_arg!(args, StructRef))?;
            Ok(smallvec![Value::u64(value)])
        },
        _ => {
            // Check if the type is a string
            if is_string_type(context, &ty_args[0])? {
                let value = aggregator_snapshot_value_as_bytes(&safely_pop_arg!(args, StructRef))?;
                let move_string_value = Value::struct_(Struct::pack(vec![Value::vector_u8(value)]));
                return Ok(smallvec![move_string_value]);
            }
            // If not a string, return an error
            Err(SafeNativeError::Abort {
                abort_code: EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE,
            })
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

    let after = string_to_bytes(safely_pop_arg!(args, Struct))?;

    let snapshot_value = match ty_args[0] {
        Type::U128 => {
            let value = aggregator_snapshot_value_as_u128(&safely_pop_arg!(args, StructRef))?;
            Ok(value.to_string().into_bytes())
        },
        Type::U64 => {
            let value = aggregator_snapshot_value_as_u64(&safely_pop_arg!(args, StructRef))?;
            Ok(value.to_string().into_bytes())
        },
        _ => {
            // Check if the type is a string
            if is_string_type(context, &ty_args[0])? {
                Ok(aggregator_snapshot_value_as_bytes(&safely_pop_arg!(
                    args, StructRef
                ))?)
            } else {
                Err(SafeNativeError::Abort {
                    abort_code: EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE,
                })
            }
        },
    }?;
    let before = string_to_bytes(safely_pop_arg!(args, Struct))?;
    let mut result = before.clone();
    result.extend(&snapshot_value);
    result.extend(&after);
    let move_string_value = Value::struct_(Struct::pack(vec![Value::vector_u8(result)]));
    let move_snapshot_value = Value::struct_(Struct::pack(vec![move_string_value]));
    Ok(smallvec![move_snapshot_value])
}

/***************************************************************************************************
 * module
 **************************************************************************************************/

pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [
        (
            "create_aggregator",
            native_create_aggregator as RawSafeNative,
        ),
        ("try_add", native_try_add),
        ("read", native_read),
        ("try_sub", native_try_sub),
        ("create_snapshot", native_create_snapshot),
        ("copy_snapshot", native_copy_snapshot),
        ("read_snapshot", native_read_snapshot),
        ("string_concat", native_string_concat),

    ];
    builder.make_named_natives(natives)
}
