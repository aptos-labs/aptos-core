// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

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

pub fn native_get_bucket(
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(ty_args.is_empty());
    debug_assert!(args.is_empty());

    // TODO: here we have some logic to calculate indices based on trnsaction hash, etc.
    let index = 0; // change this

    // TODO: We need to add theis calculation to NativeCostIndex, for now just use this.
    let cost = native_gas(
        context.cost_table(),
        NativeCostIndex::BCS_TO_BYTES,
        1,
    );
    Ok(NativeResult::ok(cost, smallvec![Value::u64(index)]))
}
