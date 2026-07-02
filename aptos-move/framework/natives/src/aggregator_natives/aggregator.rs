// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::aggregator_natives::{
    helpers_v1::{aggregator_info, unpack_aggregator_struct},
    NativeAggregatorContext,
};
use aptos_aggregator::{
    aggregator_v1_extension::{
        addition_v1_error, subtraction_v1_error, AggregatorDelayedChange, AggregatorID,
    },
    bounded_math::SignedU128,
};
use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
use move_binary_format::errors::PartialVMError;
use move_core_types::vm_status::StatusCode;
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    delayed_values::delayed_field_id::DelayedFieldID,
    loaded_data::runtime_types::Type,
    values::{Struct, StructRef, Value},
};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

/// Exchanges the storage value of an existing aggregator (a bare u128 state item) for a stable
/// delayed field id. This records the read so the value is materialized back at commit, and caches
/// the id in the versioned data so all readers in the block observe the same id.
fn exchange_aggregator_v1_id(
    aggregator_context: &NativeAggregatorContext,
    id: &AggregatorID,
) -> SafeNativeResult<DelayedFieldID> {
    aggregator_context
        .aggregator_v1_resolver
        .get_aggregator_v1_id_for_delayed_field(id.as_state_key())?
        .ok_or_else(|| {
            SafeNativeError::from(
                PartialVMError::new(StatusCode::SPECULATIVE_EXECUTION_ABORT_ERROR).with_message(
                    "Aggregator v1 value not found in storage for delayed field exchange."
                        .to_string(),
                ),
            )
        })
}

/// Records a delta against an aggregator V1. When the delayed field optimization is enabled, the
/// delta is recorded through the delayed field machinery against the aggregator's stable id (a
/// read-materialized aggregator stays an id-bearing write). Otherwise the delta is applied in
/// place to the aggregator's value.
fn apply_aggregator_v1_delta(
    aggregator_context: &NativeAggregatorContext,
    id: AggregatorID,
    max_value: u128,
    delta: SignedU128,
) -> SafeNativeResult<()> {
    if aggregator_context.delayed_field_optimization_enabled {
        let existing = aggregator_context
            .aggregator_v1_data
            .borrow()
            .delayed_change(&id)
            .map(|disposition| disposition.id());
        let delayed_field_id = match existing {
            Some(delayed_field_id) => delayed_field_id,
            None => {
                let delayed_field_id = exchange_aggregator_v1_id(aggregator_context, &id)?;
                aggregator_context
                    .aggregator_v1_data
                    .borrow_mut()
                    .record_delayed_delta(id, delayed_field_id);
                delayed_field_id
            },
        };
        let succeeded = aggregator_context
            .delayed_field_data
            .borrow_mut()
            .try_add_or_check_delta(
                delayed_field_id,
                max_value,
                delta,
                aggregator_context.delayed_field_resolver,
                true,
            )?;
        if !succeeded {
            // An add that would overflow or a subtract that would underflow aborts with the
            // aggregator v1 error code.
            return Err(match delta {
                SignedU128::Positive(_) => addition_v1_error(()),
                SignedU128::Negative(_) => subtraction_v1_error(()),
            }
            .into());
        }
    } else {
        let mut aggregator_data = aggregator_context.aggregator_v1_data.borrow_mut();
        let aggregator = aggregator_data.get_aggregator(id, max_value)?;
        match delta {
            SignedU128::Positive(value) => aggregator.add(value)?,
            SignedU128::Negative(value) => aggregator.sub(value)?,
        }
    }
    Ok(())
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
    apply_aggregator_v1_delta(
        aggregator_context,
        id,
        max_value,
        SignedU128::Positive(input),
    )?;

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

    // Extract information from aggregator struct reference.
    let (id, max_value) = aggregator_info(&safely_pop_arg!(args, StructRef))?;

    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();

    let value = if aggregator_context.delayed_field_optimization_enabled {
        let existing = aggregator_context
            .aggregator_v1_data
            .borrow()
            .delayed_change(&id)
            .map(|disposition| disposition.id());
        let delayed_field_id = match existing {
            Some(delayed_field_id) => delayed_field_id,
            None => exchange_aggregator_v1_id(aggregator_context, &id)?,
        };
        let value = aggregator_context
            .delayed_field_data
            .borrow()
            .read_aggregator(delayed_field_id, aggregator_context.delayed_field_resolver)?;
        // Reading materializes the aggregator into a concrete (id-bearing) write.
        aggregator_context
            .aggregator_v1_data
            .borrow_mut()
            .record_change(id, AggregatorDelayedChange::Materialized(delayed_field_id));
        value
    } else {
        let mut aggregator_data = aggregator_context.aggregator_v1_data.borrow_mut();
        let aggregator = aggregator_data.get_aggregator(id.clone(), max_value)?;
        aggregator.read_and_materialize(aggregator_context.aggregator_v1_resolver, &id)?
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
    apply_aggregator_v1_delta(
        aggregator_context,
        id,
        max_value,
        SignedU128::Negative(input),
    )?;

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
