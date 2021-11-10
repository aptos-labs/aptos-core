// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{gas_schedule::GasAlgebra, vm_status::StatusCode};
use move_vm_runtime::native_functions::NativeContext;
use move_vm_types::{
    gas_schedule::NativeCostIndex,
    loaded_data::runtime_types::Type,
    natives::function::{native_gas, NativeResult},
    pop_arg,
    values::{Value, Vector, VectorRef},
};

use std::collections::VecDeque;

pub fn native_empty(
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(ty_args.len() == 1);
    debug_assert!(args.is_empty());

    let cost = native_gas(context.cost_table(), NativeCostIndex::EMPTY, 1);
    NativeResult::map_partial_vm_result_one(cost, Vector::empty(&ty_args[0]))
}

pub fn native_length(
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(ty_args.len() == 1);
    debug_assert!(args.len() == 1);

    let r = pop_arg!(args, VectorRef);
    let cost = native_gas(context.cost_table(), NativeCostIndex::LENGTH, 1);
    NativeResult::map_partial_vm_result_one(cost, r.len(&ty_args[0]))
}

pub fn native_push_back(
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(ty_args.len() == 1);
    debug_assert!(args.len() == 2);

    let e = args.pop_back().unwrap();
    let r = pop_arg!(args, VectorRef);
    let cost = native_gas(
        context.cost_table(),
        NativeCostIndex::PUSH_BACK,
        e.size().get() as usize,
    );
    NativeResult::map_partial_vm_result_empty(cost, r.push_back(e, &ty_args[0]))
}

pub fn native_borrow(
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(ty_args.len() == 1);
    debug_assert!(args.len() == 2);

    let idx = pop_arg!(args, u64) as usize;
    let r = pop_arg!(args, VectorRef);
    let cost = native_gas(context.cost_table(), NativeCostIndex::BORROW, 1);
    NativeResult::map_partial_vm_result_one(
        cost,
        r.borrow_elem(idx, &ty_args[0])
            .map_err(native_error_to_abort),
    )
}

pub fn native_pop(
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(ty_args.len() == 1);
    debug_assert!(args.len() == 1);

    let r = pop_arg!(args, VectorRef);
    let cost = native_gas(context.cost_table(), NativeCostIndex::POP_BACK, 1);
    NativeResult::map_partial_vm_result_one(cost, r.pop(&ty_args[0]).map_err(native_error_to_abort))
}

pub fn native_destroy_empty(
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(ty_args.len() == 1);
    debug_assert!(args.len() == 1);

    let v = pop_arg!(args, Vector);
    let cost = native_gas(context.cost_table(), NativeCostIndex::DESTROY_EMPTY, 1);
    NativeResult::map_partial_vm_result_empty(
        cost,
        v.destroy_empty(&ty_args[0]).map_err(native_error_to_abort),
    )
}

pub fn native_swap(
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(ty_args.len() == 1);
    debug_assert!(args.len() == 3);

    let idx2 = pop_arg!(args, u64) as usize;
    let idx1 = pop_arg!(args, u64) as usize;
    let r = pop_arg!(args, VectorRef);
    let cost = native_gas(context.cost_table(), NativeCostIndex::SWAP, 1);
    NativeResult::map_partial_vm_result_empty(
        cost,
        r.swap(idx1, idx2, &ty_args[0])
            .map_err(native_error_to_abort),
    )
}

fn native_error_to_abort(err: PartialVMError) -> PartialVMError {
    let (major_status, sub_status_opt, message_opt, indices, offsets) = err.all_data();
    let new_err = match major_status {
        StatusCode::VECTOR_OPERATION_ERROR => PartialVMError::new(StatusCode::ABORTED),
        _ => PartialVMError::new(major_status),
    };
    let new_err = match sub_status_opt {
        None => new_err,
        Some(code) => new_err.with_sub_status(code),
    };
    let new_err = match message_opt {
        None => new_err,
        Some(message) => new_err.with_message(message),
    };
    new_err.at_indices(indices).at_code_offsets(offsets)
}
