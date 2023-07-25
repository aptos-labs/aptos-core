// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    natives::{
        aggregator_natives::{helpers_v2::aggregator_info, NativeAggregatorContext},
        helpers::{make_safe_native, SafeNativeContext, SafeNativeResult},
    },
    safely_pop_arg,
};
use aptos_aggregator::aggregator_extension::AggregatorID;
use aptos_types::on_chain_config::{Features, TimedFeatures};
use move_core_types::gas_algebra::InternalGas;
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Struct, StructRef, Value},
};
use smallvec::{smallvec, SmallVec};
use std::{collections::VecDeque, sync::Arc};


/***************************************************************************************************
 * native fun create_aggregator(limit: u128): Aggregator;
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct CreateAggregatorGasParameters {
    pub base: InternalGas,
}

fn native_create_aggregator(
    gas_params: &CreateAggregatorGasParameters,
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 2);

    context.charge(gas_params.base)?;

    // Extract fields: `limit` of the new aggregator and a `phantom_handle` of
    // the parent factory.
    let limit = safely_pop_arg!(args, u128);

    // Get the current aggregator data.
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut aggregator_data = aggregator_context.aggregator_data.borrow_mut();

    let id = AggregatorID::ephemeral(aggregator_data.generate_id());
    aggregator_data.create_new_aggregator(id, limit);

    Ok(smallvec![Value::struct_(Struct::pack(vec![
        Value::u128(0),
        Value::u128(limit),
    ]))])
}

/***************************************************************************************************
 * native fun try_add(aggregator: &mut Aggregator, value: u128): bool;
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct TryAddGasParameters {
    pub base: InternalGas,
}

fn native_try_add(
    gas_params: &TryAddGasParameters,
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 2);

    context.charge(gas_params.base)?;

    // Get aggregator information and a value to add.
    let value = safely_pop_arg!(args, u128);
    let (id, limit) = aggregator_info(&safely_pop_arg!(args, StructRef))?;

    // Get aggregator.
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut aggregator_data = aggregator_context.aggregator_data.borrow_mut();
    let aggregator = aggregator_data.get_aggregator(
        id,
        limit,
        aggregator_context.resolver,
        aggregator_context.aggregator_enabled,
    )?;

    Ok(smallvec![Value::bool(aggregator.try_add(value).is_ok())])
}

/***************************************************************************************************
 * native fun try_sub(aggregator: &mut Aggregator, value: u128): bool;
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct TrySubGasParameters {
    pub base: InternalGas,
}

fn native_try_sub(
    gas_params: &TrySubGasParameters,
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 2);

    context.charge(gas_params.base)?;

    // Get aggregator information and a value to subtract.
    let value = safely_pop_arg!(args, u128);
    let (id, limit) = aggregator_info(&safely_pop_arg!(args, StructRef))?;

    // Get aggregator.
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut aggregator_data = aggregator_context.aggregator_data.borrow_mut();
    let aggregator = aggregator_data.get_aggregator(
        id,
        limit,
        aggregator_context.resolver,
        aggregator_context.aggregator_enabled,
    )?;
    Ok(smallvec![Value::bool(aggregator.try_sub(value).is_ok())])
}

/***************************************************************************************************
 * native fun read(aggregator: &Aggregator): u128;
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct ReadGasParameters {
    pub base: InternalGas,
}

fn native_read(
    gas_params: &ReadGasParameters,
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 1);

    context.charge(gas_params.base)?;

    // Extract information from aggregator struct reference.
    let (id, limit) = aggregator_info(&safely_pop_arg!(args, StructRef))?;

    // Get aggregator.
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut aggregator_data = aggregator_context.aggregator_data.borrow_mut();
    let aggregator = aggregator_data.get_aggregator(
        id,
        limit,
        aggregator_context.resolver,
        aggregator_context.aggregator_enabled,
    )?;

    let value = aggregator.read_and_materialize(aggregator_context.resolver, &id)?;

    Ok(smallvec![Value::u128(value)])
}

/***************************************************************************************************
 * native fun destroy(aggregator: Aggregator);
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct DestroyGasParameters {
    pub base: InternalGas,
}

fn native_destroy(
    gas_params: &DestroyGasParameters,
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 1);

    context.charge(gas_params.base)?;

    // Extract information from aggregator struct reference.
    let (id, _limit) = aggregator_info(&safely_pop_arg!(args, StructRef))?;

    // Get aggregator data.
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut aggregator_data = aggregator_context.aggregator_data.borrow_mut();

    // Actually remove the aggregator.
    aggregator_data.remove_aggregator(id);

    Ok(smallvec![])
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct GasParameters {
    pub create_aggregator: CreateAggregatorGasParameters,
    pub try_add: TryAddGasParameters,
    pub read: ReadGasParameters,
    pub try_sub: TrySubGasParameters,
    pub destroy: DestroyGasParameters,
}

pub fn make_all(
    gas_params: GasParameters,
    timed_features: TimedFeatures,
    features: Arc<Features>,
) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [
        (
            "create_aggregator",
            make_safe_native(
                gas_params.create_aggregator,
                timed_features.clone(),
                features.clone(),
                native_create_aggregator,
            ),
        ),
        (
            "try_add",
            make_safe_native(
                gas_params.try_add,
                timed_features.clone(),
                features.clone(),
                native_try_add,
            ),
        ),
        (
            "read",
            make_safe_native(
                gas_params.read,
                timed_features.clone(),
                features.clone(),
                native_read,
            ),
        ),
        (
            "try_sub",
            make_safe_native(
                gas_params.try_sub,
                timed_features.clone(),
                features.clone(),
                native_try_sub,
            ),
        ),
        (
            "destroy",
            make_safe_native(gas_params.destroy, timed_features, features, native_destroy),
        ),
    ];
    crate::natives::helpers::make_module_natives(natives)
}
