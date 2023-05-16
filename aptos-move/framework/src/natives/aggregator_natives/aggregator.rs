// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    natives::{
        aggregator_natives::{
            helpers::{aggregator_info, unpack_aggregator_struct},
            NativeAggregatorContext,
        },
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
 * native fun add(aggregator: &mut Aggregator, value: u128);
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct AddGasParameters {
    pub base: InternalGas,
}

fn native_add(
    gas_params: &AddGasParameters,
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

    aggregator.add(value)?;

    Ok(smallvec![])
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
 * native fun sub(aggregator: &mut Aggregator, value: u128);
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct SubGasParameters {
    pub base: InternalGas,
}

fn native_sub(
    gas_params: &SubGasParameters,
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

    aggregator.sub(value)?;

    Ok(smallvec![])
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

    // First, unpack the struct.
    let aggregator_struct = safely_pop_arg!(args, Struct);
    let (handle, key, _) = unpack_aggregator_struct(aggregator_struct)?;

    // Get aggregator data.
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut aggregator_data = aggregator_context.aggregator_data.borrow_mut();

    // Actually remove the aggregator.
    let id = AggregatorID::new(handle, key);
    aggregator_data.remove_aggregator(id);

    Ok(smallvec![])
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct GasParameters {
    pub add: AddGasParameters,
    pub read: ReadGasParameters,
    pub sub: SubGasParameters,
    pub destroy: DestroyGasParameters,
}

pub fn make_all(
    gas_params: GasParameters,
    timed_features: TimedFeatures,
    features: Arc<Features>,
) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [
        (
            "add",
            make_safe_native(
                gas_params.add,
                timed_features.clone(),
                features.clone(),
                native_add,
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
            "sub",
            make_safe_native(
                gas_params.sub,
                timed_features.clone(),
                features.clone(),
                native_sub,
            ),
        ),
        (
            "destroy",
            make_safe_native(gas_params.destroy, timed_features, features, native_destroy),
        ),
    ];

    crate::natives::helpers::make_module_natives(natives)
}
