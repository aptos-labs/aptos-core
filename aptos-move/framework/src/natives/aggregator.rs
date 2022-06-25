// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_extensions::NativeTransactionContext;
use move_deps::{
    move_binary_format::errors::PartialVMResult,
    move_vm_runtime::native_functions::NativeContext,
    move_vm_types::{
        gas_schedule::NativeCostIndex,
        loaded_data::runtime_types::Type,
        natives::function::{native_gas, NativeResult},
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

    let cost = native_gas(
        context.cost_table(),
        NativeCostIndex::BCS_TO_BYTES,
        1,
    );
    Ok(NativeResult::ok(cost, smallvec![Value::u64(index)]))
}
