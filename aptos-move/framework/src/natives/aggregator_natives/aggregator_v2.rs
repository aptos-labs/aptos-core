// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::natives::{
    aggregator_natives::{
        helpers_v2::{
            aggregator_snapshot_value_as_bytes, aggregator_snapshot_value_as_u128,
            aggregator_snapshot_value_as_u64, aggregator_value_as_u128, aggregator_value_as_u64,
            string_to_bytes,
        },
        NativeAggregatorContext,
    },
    AccountAddress,
};
use aptos_aggregator::types::{AggregatorID, AggregatorVersionedID};
use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
use move_binary_format::errors::PartialVMError;
use move_core_types::{
    value::{MoveStructLayout, MoveTypeLayout},
    vm_status::StatusCode,
};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Struct, StructRef, Value},
};
use smallvec::{smallvec, SmallVec};
use std::{collections::VecDeque, ops::Deref};

/// The generic type supplied to aggregator snapshots is not supported.
pub const EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE: u64 = 0x03_0005;

/// The aggregator snapshots feature is not enabled.
pub const EAGGREGATOR_SNAPSHOTS_NOT_ENABLED: u64 = 0x03_0006;

/// The generic type supplied to the aggregators is not supported.
pub const EUNSUPPORTED_AGGREGATOR_TYPE: u64 = 0x03_0007;

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

/// Given the list of native function arguments and a type, returns a tuple of its
/// fields: (`aggregator id`, `limit`).
pub fn pop_aggregator_fields_by_type(
    ty_arg: &Type,
    args: &mut VecDeque<Value>,
) -> SafeNativeResult<(AggregatorID, u128)> {
    match ty_arg {
        Type::U128 => {
            // Get aggregator information and a value to add.
            let (id, limit) = aggregator_value_as_u128(&safely_pop_arg!(args, StructRef))?;
            Ok((id, limit))
        },
        Type::U64 => {
            // Get aggregator information and a value to add.
            let (id, limit) = aggregator_value_as_u64(&safely_pop_arg!(args, StructRef))?;
            Ok((id, limit as u128))
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: EUNSUPPORTED_AGGREGATOR_TYPE,
        }),
    }
}

/// Given the list of native function arguments and a type, pop the next argument if it is of given type
pub fn pop_value_by_type(ty_arg: &Type, args: &mut VecDeque<Value>) -> SafeNativeResult<u128> {
    match ty_arg {
        Type::U128 => Ok(safely_pop_arg!(args, u128)),
        Type::U64 => Ok(safely_pop_arg!(args, u64) as u128),
        _ => Err(SafeNativeError::Abort {
            abort_code: EUNSUPPORTED_AGGREGATOR_TYPE,
        }),
    }
}

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
    let limit = pop_value_by_type(&ty_args[0], &mut args)?;
    let mut aggregator_data = aggregator_context.aggregator_data.borrow_mut();
    let id = aggregator_data.generate_id();
    let aggregator_id = AggregatorVersionedID::V2(aggregator_data.generate_id());
    aggregator_data.create_new_aggregator(aggregator_id, limit);

    match ty_args[0] {
        Type::U128 => Ok(smallvec![Value::struct_(Struct::pack(vec![
            Value::u128(id.id() as u128),
            Value::u128(limit),
        ]))]),
        Type::U64 => Ok(smallvec![Value::struct_(Struct::pack(vec![
            Value::u64(id.id()),
            Value::u64(limit as u64),
        ]))]),
        _ => Err(SafeNativeError::Abort {
            abort_code: EUNSUPPORTED_AGGREGATOR_TYPE,
        }),
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
    let value = pop_value_by_type(&ty_args[0], &mut args)?;
    let (id, limit) = pop_aggregator_fields_by_type(&ty_args[0], &mut args)?;
    let aggregator = aggregator_data.get_aggregator(AggregatorVersionedID::V2(id), limit)?;
    Ok(smallvec![Value::bool(
        aggregator.try_add(aggregator_context.resolver, value)?
    )])
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
    let value = pop_value_by_type(&ty_args[0], &mut args)?;
    let (id, limit) = pop_aggregator_fields_by_type(&ty_args[0], &mut args)?;
    let aggregator = aggregator_data.get_aggregator(AggregatorVersionedID::V2(id), limit)?;
    Ok(smallvec![Value::bool(
        aggregator.try_sub(aggregator_context.resolver, value)?
    )])
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
    let (id, limit) = pop_aggregator_fields_by_type(&ty_args[0], &mut args)?;
    let aggregator = aggregator_data.get_aggregator(AggregatorVersionedID::V2(id), limit)?;
    let value = aggregator.read_most_recent_aggregator_value(aggregator_context.resolver)?;
    if value > limit {
        return Err(SafeNativeError::InvariantViolation(PartialVMError::new(
            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
        )));
    };
    match ty_args[0] {
        Type::U128 => Ok(smallvec![Value::u128(value)]),
        Type::U64 => Ok(smallvec![Value::u64(value as u64)]),
        _ => Err(SafeNativeError::Abort {
            abort_code: EUNSUPPORTED_AGGREGATOR_TYPE,
        }),
    }
}

/***************************************************************************************************
 * native fun snapshot(aggregator: &Aggregator): AggregatorSnapshot<u128>;
 **************************************************************************************************/

fn native_snapshot(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 1);

    context.charge(AGGREGATOR_V2_SNAPSHOT_BASE)?;
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut aggregator_data = aggregator_context.aggregator_data.borrow_mut();

    match ty_args[0] {
        Type::U128 => {
            let (aggregator_id, _) = aggregator_value_as_u128(&safely_pop_arg!(args, StructRef))?;
            let id = aggregator_data.snapshot(aggregator_id)?;
            Ok(smallvec![Value::struct_(Struct::pack(vec![Value::u128(
                id.id() as u128
            )]))])
        },
        Type::U64 => {
            let (aggregator_id, _) = aggregator_value_as_u64(&safely_pop_arg!(args, StructRef))?;
            let id = aggregator_data.snapshot(aggregator_id)?;
            Ok(smallvec![Value::struct_(Struct::pack(vec![Value::u64(
                id.id()
            )]))])
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: EUNSUPPORTED_AGGREGATOR_SNAPSHOT_TYPE,
        }),
    }
}

/***************************************************************************************************
 * native fun create_snapshot(value: Element): AggregatorSnapshot<Element>;
 **************************************************************************************************/

fn native_create_snapshot(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    if !context.aggregator_v2_api_enabled() {
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
        ("snapshot", native_snapshot),
        ("create_snapshot", native_create_snapshot),
        ("copy_snapshot", native_copy_snapshot),
        ("read_snapshot", native_read_snapshot),
        ("string_concat", native_string_concat),
    ];
    builder.make_named_natives(natives)
}
