// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::natives::aggregator_natives::{
    helpers_v1::{aggregator_info, unpack_aggregator_struct},
    NativeAggregatorContext,
};
use aptos_aggregator::types::{AggregatorVersionedID, ReadPosition};
use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
use move_binary_format::errors::PartialVMError;
use move_core_types::vm_status::StatusCode;
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Struct, StructRef, Value},
};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

/// The value of aggregator overflows. Raised by native code.
pub(crate) const EAGGREGATOR_OVERFLOW: u64 = 0x02_0001;

/// The value of aggregator underflows (goes below zero). Raised by native code.
pub(crate) const EAGGREGATOR_UNDERFLOW: u64 = 0x02_0002;

pub(crate) fn abort_error(message: impl ToString, code: u64) -> PartialVMError {
    PartialVMError::new(StatusCode::ABORTED)
        .with_message(message.to_string())
        .with_sub_status(code)
}

/***************************************************************************************************
 * native fun add(aggregator: &mut Aggregator, value: u128);
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
fn native_add(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 2);

    context.charge(AGGREGATOR_ADD_BASE)?;

    // Get aggregator information and a value to add.
    let input = safely_pop_arg!(args, u128);
    let (id, max_value) = aggregator_info(&safely_pop_arg!(args, StructRef))?;

    // Get aggregator.
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut aggregator_data = aggregator_context.aggregator_data.borrow_mut();
    let aggregator = aggregator_data.get_aggregator(id, max_value)?;
    let result = aggregator.try_add(input, aggregator_context.resolver)?;

    if result {
        Ok(smallvec![])
    } else {
        Err(SafeNativeError::from(abort_error(
            "Aggregator_v1 overflow",
            EAGGREGATOR_OVERFLOW,
        )))
    }
}

/***************************************************************************************************
 * native fun read(aggregator: &Aggregator): u128;
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
fn native_read(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 1);

    context.charge(AGGREGATOR_READ_BASE)?;

    // Extract information from aggregator struct reference.
    let (id, max_value) = aggregator_info(&safely_pop_arg!(args, StructRef))?;

    // Get aggregator.
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut aggregator_data = aggregator_context.aggregator_data.borrow_mut();
    let aggregator = aggregator_data.get_aggregator(id.clone(), max_value)?;

    let value = aggregator.read_aggregated_aggregator_value(
        aggregator_context.resolver,
        ReadPosition::AfterCurrentTxn,
    )?;

    Ok(smallvec![Value::u128(value)])
}

/***************************************************************************************************
 * native fun sub(aggregator: &mut Aggregator, value: u128);
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/

fn native_sub(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 2);

    context.charge(AGGREGATOR_SUB_BASE)?;

    // Get aggregator information and a value to subtract.
    let input = safely_pop_arg!(args, u128);
    let (id, max_value) = aggregator_info(&safely_pop_arg!(args, StructRef))?;

    // Get aggregator.
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut aggregator_data = aggregator_context.aggregator_data.borrow_mut();
    let aggregator = aggregator_data.get_aggregator(id, max_value)?;
    let result = aggregator.try_sub(input, aggregator_context.resolver)?;

    if result {
        Ok(smallvec![])
    } else {
        Err(SafeNativeError::from(abort_error(
            "Aggregator_v1 underflow",
            EAGGREGATOR_UNDERFLOW,
        )))
    }
}

/***************************************************************************************************
 * native fun destroy(aggregator: Aggregator);
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
fn native_destroy(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 1);

    context.charge(AGGREGATOR_DESTROY_BASE)?;

    // First, unpack the struct.
    let aggregator_struct = safely_pop_arg!(args, Struct);
    let (handle, key, _) = unpack_aggregator_struct(aggregator_struct)?;

    // Get aggregator data.
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut aggregator_data = aggregator_context.aggregator_data.borrow_mut();

    // Actually remove the aggregator.
    let id = AggregatorVersionedID::v1(handle, key);
    aggregator_data.remove_aggregator_v1(id);

    Ok(smallvec![])
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [
        ("add", native_add as RawSafeNative),
        ("read", native_read),
        ("sub", native_sub),
        ("destroy", native_destroy),
    ];

    builder.make_named_natives(natives)
}
