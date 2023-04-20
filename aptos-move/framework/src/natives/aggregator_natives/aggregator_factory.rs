// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    natives::{
        aggregator_natives::{helpers::get_handle, NativeAggregatorContext},
        helpers::{make_safe_native, SafeNativeContext, SafeNativeResult},
    },
    safely_pop_arg,
};
use aptos_aggregator::aggregator_extension::{extension_error, AggregatorHandle, AggregatorID};
use aptos_crypto::hash::DefaultHasher;
use aptos_types::{
    account_address::AccountAddress,
    on_chain_config::{Features, TimedFeatures},
};
use move_core_types::gas_algebra::InternalGas;
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Struct, StructRef, Value},
};
use smallvec::{smallvec, SmallVec};
use std::{collections::VecDeque, sync::Arc};

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
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 2);

    context.charge(gas_params.base)?;

    // Extract fields: `limit` of the new aggregator and a `phantom_handle` of
    // the parent factory.
    let limit = safely_pop_arg!(args, u128);
    let handle = get_handle(&safely_pop_arg!(args, StructRef))?;

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
        AccountAddress::from_bytes(hash)
            .map_err(|_| extension_error("unable to create aggregator key"))?,
    );

    let id = AggregatorID::new(handle, key);
    aggregator_data.create_new_aggregator(id, limit);

    Ok(smallvec![Value::struct_(Struct::pack(vec![
        Value::address(handle.0),
        Value::address(key.0),
        Value::u128(limit),
    ]))])
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct GasParameters {
    pub new_aggregator: NewAggregatorGasParameters,
}

pub fn make_all(
    gas_params: GasParameters,
    timed_features: TimedFeatures,
    features: Arc<Features>,
) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [(
        "new_aggregator",
        make_safe_native(
            gas_params.new_aggregator,
            timed_features,
            features,
            native_new_aggregator,
        ),
    )];

    crate::natives::helpers::make_module_natives(natives)
}
