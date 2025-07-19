// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::natives::aggregator_natives::{helpers_v1::get_handle, NativeAggregatorContext};
use aptos_aggregator::aggregator_v1_extension::AggregatorID;
use aptos_crypto::hash::DefaultHasher;
use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
use aptos_types::account_address::AccountAddress;
use move_binary_format::errors::PartialVMError;
use move_core_types::vm_status::StatusCode;
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Struct, StructRef, Value},
};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

/***************************************************************************************************
 * native fun new_aggregator(aggregator_factory: &mut AggregatorFactory, limit: u128): Aggregator;
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
fn native_new_aggregator(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert_eq!(args.len(), 2);

    context.charge(AGGREGATOR_FACTORY_NEW_AGGREGATOR_BASE)?;

    // Extract fields: `limit` of the new aggregator and a `phantom_handle` of
    // the parent factory.
    let limit = safely_pop_arg!(args, u128);
    let handle = get_handle(&safely_pop_arg!(args, StructRef))?;

    // Get the current aggregator data.
    let aggregator_context = context.extensions().get::<NativeAggregatorContext>();
    let mut aggregator_data = aggregator_context.aggregator_v1_data.borrow_mut();

    // Every aggregator V1 instance uses a unique key for its id. Here we can reuse
    // the strategy from `table` implementation: taking hash of transaction and
    // number of aggregator instances created so far.
    let mut hasher = DefaultHasher::new(&[0_u8; 0]);
    hasher.update(&aggregator_context.txn_hash());
    hasher.update(&aggregator_data.aggregator_count().to_be_bytes());
    let hash = hasher.finish().to_vec();

    if let Ok(key) = AccountAddress::from_bytes(hash) {
        let id = AggregatorID::new(handle, key);
        aggregator_data.create_new_aggregator(id, limit);

        Ok(smallvec![Value::struct_(Struct::pack(vec![
            Value::address(handle.0),
            Value::address(key),
            Value::u128(limit),
        ]))])
    } else {
        Err(SafeNativeError::InvariantViolation(
            PartialVMError::new(StatusCode::VM_EXTENSION_ERROR)
                .with_message("Unable to create a key for aggregator V1".to_string()),
        ))
    }
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [("new_aggregator", native_new_aggregator as RawSafeNative)];

    builder.make_named_natives(natives)
}
