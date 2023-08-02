// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::natives::aggregator_natives::{helpers_v2::{aggregator_info, aggregator_snapshot_u128_info, aggregator_snapshot_u64_info}, NativeAggregatorContext};
use aptos_aggregator::aggregator_extension::AggregatorID;
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeResult,
};
use aptos_gas_schedule::gas_params::natives::aptos_framework::{
    AGGREGATOR_V2_SNAPSHOT_BASE, AGGREGATOR_V2_SNAPSHOT_WITH_U64_LIMIT_BASE, AGGREGATOR_V2_READ_SNAPSHOT_BASE
};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Struct, StructRef, Value},
};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;


/***************************************************************************************************
 * native fun snapshot(aggregator: &Aggregator): AggregatorSnapshot<u128>;
 **************************************************************************************************/

fn native_snapshot(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 1);

    context.charge(AGGREGATOR_V2_SNAPSHOT_BASE)?;
    let (id, limit) = aggregator_info(&safely_pop_arg!(args, StructRef))?;

    // Get aggregator.
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut aggregator_data = aggregator_context.aggregator_data.borrow_mut();
    let aggregator = aggregator_data.get_aggregator(id, limit)?;
    let snapshot = aggregator_data.snapshot(id);

    Ok(smallvec![Value::struct_(Struct::pack(vec![
        Value::u128(0),
        Value::u128(limit),
    ]))])
}


/***************************************************************************************************
 * native fun snapshot_with_u64_limit(aggregator: &Aggregator): AggregatorSnapshot<u64>;
 **************************************************************************************************/

fn native_snapshot_with_u64_limit(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 1);

    context.charge(AGGREGATOR_V2_SNAPSHOT_WITH_U64_LIMIT_BASE)?;
    let (id, limit) = aggregator_info(&safely_pop_arg!(args, StructRef))?;

    // Get aggregator.
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut aggregator_data = aggregator_context.aggregator_data.borrow_mut();
    let aggregator = aggregator_data.get_aggregator(id, limit)?;


    Ok(smallvec![Value::struct_(Struct::pack(vec![
        Value::u128(limit),
    ]))])
}


/***************************************************************************************************
 * native fun read_snapshot<Element>(snapshot: &AggregatorSnapshot<Element>): Element;
 **************************************************************************************************/

fn native_read_snapshot(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 1);

    context.charge(AGGREGATOR_V2_READ_SNAPSHOT_BASE)?;
    // Get the current aggregator data.
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();

    match ty_args[0] {
        Type::U64 => {
            let snapshot_id = aggregator_snapshot_u64_info(&safely_pop_arg!(args, StructRef))?
            let mut aggregator_data = aggregator_context.aggregator_data.borrow_mut();
            let value = aggregator_data.read_snapshot_u64(snapshot_id);
            Ok(smallvec![Value::u64(value)])
        },
        Type::U128 => {
            let snapshot_id = aggregator_snapshot_u128_info(&safely_pop_arg!(args, StructRef))?,
            let mut aggregator_data = aggregator_context.aggregator_data.borrow_mut();
            let value = aggregator_data.read_snapshot_u128(snapshot_id);
            Ok(smallvec![Value::u128(value)])
        },
        _ => unreachable!("Snapshot can only be u64 or u128")
    }
}

/***************************************************************************************************
 * module
 **************************************************************************************************/

 pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [
        (
            "snapshot",
            native_snapshot as RawSafeNative,
        ),
        ("snapshot_with_u64_limit", native_snapshot_with_u64_limit),
        ("read_snapshot", native_read_snapshot),
    ];
    builder.make_named_natives(natives)
}
