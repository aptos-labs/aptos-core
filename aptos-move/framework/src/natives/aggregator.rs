// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_extensions::NativeTransactionContext;
use move_deps::{
    move_binary_format::errors::PartialVMResult,
    move_core_types::gas_schedule::GasCost,
    move_vm_runtime::native_functions::NativeContext,
    move_vm_types::{
        loaded_data::runtime_types::Type,
        natives::function::NativeResult,
        values::Value,
    },
};
use smallvec::smallvec;
use std::collections::VecDeque;

// TODO: this is a placeholder value for now.
const NUM_BUCKETS: u128 = 100;

pub fn native_get_bucket(
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(ty_args.is_empty());
    debug_assert!(args.is_empty());

    // Calculate the index based on transaction hash and the maximum number
    // of buckets.
    let txn_context = context.extensions().get::<NativeTransactionContext>();
    let index = (txn_context.txn_hash() % NUM_BUCKETS) as u64;

    // TODO: Calculate gas cost based on the formula.
    let cost = GasCost::new(super::cost::APTOS_GET_BUCKET, 1).total();
    Ok(NativeResult::ok(cost, smallvec![Value::u64(index)]))
}
