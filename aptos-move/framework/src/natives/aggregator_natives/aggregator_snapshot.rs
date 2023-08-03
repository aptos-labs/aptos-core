// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::natives::aggregator_natives::{
    helpers_v2::{aggregator_info, aggregator_snapshot_u128_info, aggregator_snapshot_u64_info},
    NativeAggregatorContext,
};
use aptos_aggregator::aggregator_extension::AggregatorSnapshotID;
use aptos_gas_schedule::gas_params::natives::aptos_framework::{
    AGGREGATOR_V2_READ_SNAPSHOT_BASE, AGGREGATOR_V2_SNAPSHOT_BASE,
    AGGREGATOR_V2_SNAPSHOT_WITH_U64_LIMIT_BASE,
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
 * native fun snapshot(aggregator: &Aggregator): AggregatorSnapshot<u128>;
 **************************************************************************************************/

fn native_snapshot(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 1);

    context.charge(AGGREGATOR_V2_SNAPSHOT_BASE)?;
    let (aggregator_id, _limit) = aggregator_info(&safely_pop_arg!(args, StructRef))?;

    // Get aggregator.
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut aggregator_data = aggregator_context.aggregator_data.borrow_mut();
    let AggregatorSnapshotID { id } = aggregator_data.snapshot(&aggregator_id);

    Ok(smallvec![Value::struct_(Struct::pack(vec![Value::u128(
        id as u128
    ),]))])
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
    let (aggregator_id, _limit) = aggregator_info(&safely_pop_arg!(args, StructRef))?;

    // Get aggregator.
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut aggregator_data = aggregator_context.aggregator_data.borrow_mut();
    let AggregatorSnapshotID { id } = aggregator_data.snapshot(&aggregator_id);

    Ok(smallvec![Value::struct_(Struct::pack(vec![Value::u64(
        id
    ),]))])
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
            let snapshot_id = aggregator_snapshot_u64_info(&safely_pop_arg!(args, StructRef))?;
            let aggregator_data = aggregator_context.aggregator_data.borrow_mut();
            let value = aggregator_data.read_snapshot(AggregatorSnapshotID { id: snapshot_id });
            assert!(
                value <= u64::MAX as u128,
                "Snapshot value can't exceed u64::MAX"
            );
            Ok(smallvec![Value::u64(value as u64)])
        },
        Type::U128 => {
            let snapshot_id = aggregator_snapshot_u128_info(&safely_pop_arg!(args, StructRef))?;
            let aggregator_data = aggregator_context.aggregator_data.borrow_mut();
            assert!(
                snapshot_id <= u64::MAX as u128,
                "Snapshot ID can't exceed u64::MAX"
            );
            let value = aggregator_data.read_snapshot(AggregatorSnapshotID {
                id: snapshot_id as u64,
            });
            Ok(smallvec![Value::u128(value)])
        },
        _ => unreachable!("Snapshot can only be u64 or u128"),
    }
}

/***************************************************************************************************
 * module
 **************************************************************************************************/

pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [
        ("snapshot", native_snapshot as RawSafeNative),
        ("snapshot_with_u64_limit", native_snapshot_with_u64_limit),
        ("read_snapshot", native_read_snapshot),
    ];
    builder.make_named_natives(natives)
}
