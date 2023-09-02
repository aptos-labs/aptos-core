// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::natives::aggregator_natives::{
    helpers_v2::{
        aggregator_info_u128, aggregator_info_u64, aggregator_snapshot_u128_info,
        aggregator_snapshot_u64_info,
    },
    NativeAggregatorContext,
};
use aptos_aggregator::aggregator_extension::AggregatorID;
use aptos_gas_schedule::gas_params::natives::aptos_framework::{
    AGGREGATOR_V2_READ_SNAPSHOT_BASE, AGGREGATOR_V2_SNAPSHOT_BASE,
};
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
            let (aggregator_id, _) = aggregator_info_u128(&safely_pop_arg!(args, StructRef))?;
            let id = aggregator_data.snapshot(&aggregator_id);
            Ok(smallvec![Value::struct_(Struct::pack(vec![Value::u128(
                id as u128
            )]))])
        },
        Type::U64 => {
            let (aggregator_id, _) = aggregator_info_u64(&safely_pop_arg!(args, StructRef))?;
            let id = aggregator_data.snapshot(&aggregator_id);
            Ok(smallvec![Value::struct_(Struct::pack(vec![Value::u64(
                id
            )]))])
        },
        _ => Err(PartialVMError::new(StatusCode::ABORTED)
            .with_message("Unsupported type supplied to aggregator".to_string())
            .with_sub_status(0x02_0005)
            .into()),
    }
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
    let aggregator_data = aggregator_context.aggregator_data.borrow_mut();

    match ty_args[0] {
        Type::U128 => {
            let snapshot_id = aggregator_snapshot_u128_info(&safely_pop_arg!(args, StructRef))?;
            assert!(
                snapshot_id <= u64::MAX as u128,
                "Snapshot ID can't exceed u64::MAX"
            );
            let value = aggregator_data.read_snapshot(AggregatorID::ephemeral(snapshot_id as u64));
            Ok(smallvec![Value::u128(value)])
        },
        Type::U64 => {
            let snapshot_id = aggregator_snapshot_u64_info(&safely_pop_arg!(args, StructRef))?;
            let value = aggregator_data.read_snapshot(AggregatorID::ephemeral(snapshot_id));
            assert!(
                value <= u64::MAX as u128,
                "Snapshot value can't exceed u64::MAX"
            );
            Ok(smallvec![Value::u64(value as u64)])
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
        ("snapshot", native_snapshot as RawSafeNative),
        ("read_snapshot", native_read_snapshot),
    ];
    builder.make_named_natives(natives)
}
