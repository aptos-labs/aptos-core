// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::aggregator_natives::{
    context::AggregatorValue,
    helpers_v1::{aggregator_info, unpack_aggregator_struct},
    NativeAggregatorContext,
};
use aptos_aggregator::{
    aggregator_v1_extension::AggregatorID,
    bounded_math::{BoundedMath, SignedU128},
};
use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Struct, StructRef, Value},
};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

// Error codes from `aggregator.move`
const EADD_OVERFLOW: u64 = 0x02_0001;

const ESUB_UNDERFLOW: u64 = 0x02_0002;

/// Records a delta against an aggregator V1. When the delayed field optimization
/// is enabled, the delta is recorded through the delayed field extension. If not
/// enabled, the value is read on first touch and the delta is applied in place.
/// Returns true of applying the delta was successful.
fn apply_aggregator_v1_delta(
    aggregator_context: &NativeAggregatorContext,
    id: AggregatorID,
    max_value: u128,
    delta: SignedU128,
) -> SafeNativeResult<bool> {
    let succeeded = match aggregator_context.resolve_aggregator_value(&id)? {
        AggregatorValue::Delayed(delayed_field_id) => aggregator_context
            .delayed_field_data
            .borrow_mut()
            .try_add_or_check_delta(
                delayed_field_id,
                max_value,
                delta,
                aggregator_context.delayed_field_resolver,
                true,
            )?,
        AggregatorValue::Concrete(current) => {
            let math = BoundedMath::new(max_value);
            let result = match delta {
                SignedU128::Positive(value) => math.unsigned_add(current, value),
                SignedU128::Negative(value) => math.unsigned_subtract(current, value),
            };
            if let Ok(updated) = result {
                aggregator_context
                    .aggregator_v1_data
                    .borrow_mut()
                    .set_value(id, updated);
                true
            } else {
                false
            }
        },
    };
    Ok(succeeded)
}

/***************************************************************************************************
 * native fun add(aggregator: &mut Aggregator, value: u128);
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
fn native_add(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 2);

    context.charge(AGGREGATOR_ADD_BASE)?;

    // Get aggregator information and a value to add.
    let input = safely_pop_arg!(args, u128);
    let (id, max_value) = aggregator_info(&safely_pop_arg!(args, StructRef))?;

    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let succeeded = apply_aggregator_v1_delta(
        aggregator_context,
        id,
        max_value,
        SignedU128::Positive(input),
    )?;
    if !succeeded {
        return Err(SafeNativeError::abort(EADD_OVERFLOW));
    }

    Ok(smallvec![])
}

/***************************************************************************************************
 * native fun read(aggregator: &Aggregator): u128;
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
fn native_read(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 1);

    context.charge(AGGREGATOR_READ_BASE)?;

    let (id, _) = aggregator_info(&safely_pop_arg!(args, StructRef))?;
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();

    let value = match aggregator_context.resolve_aggregator_value(&id)? {
        AggregatorValue::Delayed(delayed_field_id) => {
            let value = aggregator_context
                .delayed_field_data
                .borrow()
                .read_aggregator(delayed_field_id, aggregator_context.delayed_field_resolver)?;

            // Reading an aggregator transitions its state from delayed to a write.
            // This is important for later state value metadata calculations where
            // writes carry it while deltas (delayed aggregator operations) do not.
            let mut aggregator_data = aggregator_context.aggregator_v1_data.borrow_mut();
            aggregator_data.set_id(id.clone(), delayed_field_id);
            aggregator_data.mark_read(id);
            value
        },
        AggregatorValue::Concrete(value) => {
            let mut aggregator_data = aggregator_context.aggregator_v1_data.borrow_mut();
            aggregator_data.set_value(id.clone(), value);
            aggregator_data.mark_read(id);
            value
        },
    };

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
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 2);

    context.charge(AGGREGATOR_SUB_BASE)?;

    // Get aggregator information and a value to subtract.
    let input = safely_pop_arg!(args, u128);
    let (id, max_value) = aggregator_info(&safely_pop_arg!(args, StructRef))?;

    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let succeeded = apply_aggregator_v1_delta(
        aggregator_context,
        id,
        max_value,
        SignedU128::Negative(input),
    )?;
    if !succeeded {
        return Err(SafeNativeError::abort(ESUB_UNDERFLOW));
    }

    Ok(smallvec![])
}

/***************************************************************************************************
 * native fun destroy(aggregator: Aggregator);
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
fn native_destroy(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 1);

    context.charge(AGGREGATOR_DESTROY_BASE)?;

    // First, unpack the struct.
    let aggregator_struct = safely_pop_arg!(args, Struct);
    let (handle, key, _) = unpack_aggregator_struct(aggregator_struct)?;

    // Get aggregator data.
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut aggregator_data = aggregator_context.aggregator_v1_data.borrow_mut();

    // Actually remove the aggregator.
    let id = AggregatorID::new(handle, key);
    aggregator_data.remove_aggregator(id);

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
