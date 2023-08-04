// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::natives::aggregator_natives::{helpers_v2::aggregator_info, NativeAggregatorContext};
use aptos_aggregator::aggregator_extension::AggregatorID;
use aptos_gas_schedule::gas_params::natives::aptos_framework::{
    AGGREGATOR_V2_READ_BASE, AGGREGATOR_V2_TRY_ADD_BASE, AGGREGATOR_V2_TRY_SUB_BASE, *,
};
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeResult,
};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Struct, StructRef, Value},
};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

/***************************************************************************************************
 * native fun create_aggregator(max_value: u128): Aggregator;
 **************************************************************************************************/

fn native_create_aggregator(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 1);

    context.charge(AGGREGATOR_V2_CREATE_AGGREGATOR_BASE)?;
    let max_value = safely_pop_arg!(args, u128);

    // Get the current aggregator data.
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut aggregator_data = aggregator_context.aggregator_data.borrow_mut();

    let id = AggregatorID::ephemeral(aggregator_data.generate_id());
    aggregator_data.create_new_aggregator(id, max_value);

    Ok(smallvec![Value::struct_(Struct::pack(vec![
        Value::u128(0),
        Value::u128(max_value),
    ]))])
}

/***************************************************************************************************
 * native fun try_add(aggregator: &mut Aggregator, value: u128): bool;
 **************************************************************************************************/
fn native_try_add(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 2);

    context.charge(AGGREGATOR_V2_TRY_ADD_BASE)?;

    // Get aggregator information and a value to add.
    let value = safely_pop_arg!(args, u128);
    let (id, max_value) = aggregator_info(&safely_pop_arg!(args, StructRef))?;

    // Get aggregator.
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut aggregator_data = aggregator_context.aggregator_data.borrow_mut();
    let aggregator = aggregator_data.get_aggregator(id, max_value)?;

    Ok(smallvec![Value::bool(aggregator.try_add(value).is_ok())])
}

/***************************************************************************************************
 * native fun try_sub(aggregator: &mut Aggregator, value: u128): bool;
 **************************************************************************************************/
fn native_try_sub(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 2);

    context.charge(AGGREGATOR_V2_TRY_SUB_BASE)?;

    // Get aggregator information and a value to subtract.
    let value = safely_pop_arg!(args, u128);
    let (id, max_value) = aggregator_info(&safely_pop_arg!(args, StructRef))?;

    // Get aggregator.
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut aggregator_data = aggregator_context.aggregator_data.borrow_mut();
    let aggregator = aggregator_data.get_aggregator(id, max_value)?;
    Ok(smallvec![Value::bool(aggregator.try_sub(value).is_ok())])
}

/***************************************************************************************************
 * native fun read(aggregator: &Aggregator): u128;
 **************************************************************************************************/

fn native_read(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 1);

    context.charge(AGGREGATOR_V2_READ_BASE)?;

    // Extract information from aggregator struct reference.
    let (id, max_value) = aggregator_info(&safely_pop_arg!(args, StructRef))?;

    // Get aggregator.
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut aggregator_data = aggregator_context.aggregator_data.borrow_mut();
    let aggregator = aggregator_data.get_aggregator(id, max_value)?;

    let value = aggregator.read_and_materialize(aggregator_context.resolver, &id)?;

    Ok(smallvec![Value::u128(value)])
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
    ];

    builder.make_named_natives(natives)
}
