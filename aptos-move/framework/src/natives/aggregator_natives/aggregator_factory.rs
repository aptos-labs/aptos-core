// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_aggregator::aggregator_extension::{extension_error, AggregatorHandle, AggregatorID};
use aptos_crypto::hash::DefaultHasher;
use aptos_types::account_address::AccountAddress;
use move_deps::{
    move_binary_format::errors::PartialVMResult,
    move_core_types::gas_algebra::InternalGas,
    move_vm_runtime::native_functions::{NativeContext, NativeFunction},
    move_vm_types::{
        loaded_data::runtime_types::Type,
        natives::function::NativeResult,
        pop_arg,
        values::{Struct, StructRef, Value},
    },
};
use smallvec::smallvec;
use std::{collections::VecDeque, sync::Arc};

use crate::natives::aggregator_natives::{helpers::get_handle, NativeAggregatorContext};

/***************************************************************************************************
 * native fun new_aggregator(aggregator_factory: &mut AggregatorFactory, limit: u128): Aggregator;
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct NewAggregatorGasParameters {
    pub base: InternalGas,
}

fn native_new_aggregator(
    gas_params: &NewAggregatorGasParameters,
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert!(args.len() == 2);

    // Extract fields: `limit` of the new aggregator and a `phantom_handle` of
    // the parent factory.
    let limit = pop_arg!(args, u128);
    let handle = get_handle(&pop_arg!(args, StructRef))?;

    // Get the current aggregator data.
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut aggregator_data = aggregator_context.aggregator_data.borrow_mut();

    // Every aggregator instance uses a unique key in its id. Here we can reuse
    // the strategy from `table` implementation: taking hash of transaction and
    // number of aggregator instances created so far.
    let num_aggregators_len = aggregator_data.num_aggregators() as u32;

    let mut hasher = DefaultHasher::new(&[0_u8; 0]);
    hasher.update(&aggregator_context.txn_hash());
    hasher.update(&num_aggregators_len.to_be_bytes());
    let hash = hasher.finish().to_vec();
    let key = AggregatorHandle(
        AccountAddress::from_bytes(&hash)
            .map_err(|_| extension_error("unable to create aggregator key"))?,
    );

    let id = AggregatorID::new(handle, key);
    aggregator_data.create_new_aggregator(id, limit);

    Ok(NativeResult::ok(
        gas_params.base,
        smallvec![Value::struct_(Struct::pack(vec![
            Value::address(handle.0),
            Value::address(key.0),
            Value::u128(limit),
        ]))],
    ))
}

pub fn make_native_new_aggregator(gas_params: NewAggregatorGasParameters) -> NativeFunction {
    Arc::new(move |context, ty_args, args| {
        native_new_aggregator(&gas_params, context, ty_args, args)
    })
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct GasParameters {
    pub new_aggregator: NewAggregatorGasParameters,
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [(
        "new_aggregator",
        make_native_new_aggregator(gas_params.new_aggregator),
    )];

    crate::natives::helpers::make_module_natives(natives)
}
