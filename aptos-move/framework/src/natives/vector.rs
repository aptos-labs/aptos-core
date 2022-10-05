// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::natives::helpers::make_module_natives;
use aptos_types::on_chain_config::{FeatureFlag, NativeFeatureContext};
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{gas_algebra::InternalGas, vm_status::StatusCode};
use move_stdlib::natives::vector::{
    BorrowGasParameters, DestroyEmptyGasParameters, EmptyGasParameters, GasParameters,
    LengthGasParameters, PopBackGasParameters, PushBackGasParameters, SwapGasParameters,
};
use move_vm_runtime::native_functions::{NativeContext, NativeFunction};
use move_vm_types::{
    loaded_data::runtime_types::Type,
    natives::function::NativeResult,
    pop_arg,
    values::{Value, Vector, VectorRef},
    views::ValueView,
};
use std::{collections::VecDeque, sync::Arc};

// error::permission_denied(0x1)
const ELEGACY_VECTOR_NOT_SUPPORTED: u64 = 0x05_0001;

fn check_vector_legacy_supported(
    context: &NativeContext,
    cost: InternalGas,
) -> Option<NativeResult> {
    if context
        .extensions()
        .get::<NativeFeatureContext>()
        .features
        .is_enabled(FeatureFlag::NO_LEGACY_VECTOR)
    {
        Some(NativeResult::err(cost, ELEGACY_VECTOR_NOT_SUPPORTED))
    } else {
        None
    }
}

/***************************************************************************************************
 * native fun empty
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/

pub fn native_empty(
    gas_params: &EmptyGasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(ty_args.len() == 1);
    debug_assert!(args.is_empty());
    if let Some(err) = check_vector_legacy_supported(context, gas_params.base) {
        return Ok(err);
    }
    NativeResult::map_partial_vm_result_one(gas_params.base, Vector::empty(&ty_args[0]))
}

pub fn make_native_empty(gas_params: EmptyGasParameters) -> NativeFunction {
    Arc::new(
        move |context, ty_args, args| -> PartialVMResult<NativeResult> {
            native_empty(&gas_params, context, ty_args, args)
        },
    )
}

/***************************************************************************************************
 * native fun length
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/

pub fn native_length(
    gas_params: &LengthGasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(ty_args.len() == 1);
    debug_assert!(args.len() == 1);
    if let Some(err) = check_vector_legacy_supported(context, gas_params.base) {
        return Ok(err);
    }

    let r = pop_arg!(args, VectorRef);
    NativeResult::map_partial_vm_result_one(gas_params.base, r.len(&ty_args[0]))
}

pub fn make_native_length(gas_params: LengthGasParameters) -> NativeFunction {
    Arc::new(
        move |context, ty_args, args| -> PartialVMResult<NativeResult> {
            native_length(&gas_params, context, ty_args, args)
        },
    )
}

/***************************************************************************************************
 * native fun push_back
 *
 *   gas cost: base_cost + legacy_unit_cost * max(1, size_of(val))
 *
 **************************************************************************************************/

pub fn native_push_back(
    gas_params: &PushBackGasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(ty_args.len() == 1);
    debug_assert!(args.len() == 2);
    if let Some(err) = check_vector_legacy_supported(context, gas_params.base) {
        return Ok(err);
    }

    let e = args.pop_back().unwrap();
    let r = pop_arg!(args, VectorRef);

    let mut cost = gas_params.base;
    if gas_params.legacy_per_abstract_memory_unit != 0.into() {
        cost += gas_params.legacy_per_abstract_memory_unit
            * std::cmp::max(e.legacy_abstract_memory_size(), 1.into());
    }

    NativeResult::map_partial_vm_result_empty(cost, r.push_back(e, &ty_args[0]))
}

pub fn make_native_push_back(gas_params: PushBackGasParameters) -> NativeFunction {
    Arc::new(
        move |context, ty_args, args| -> PartialVMResult<NativeResult> {
            native_push_back(&gas_params, context, ty_args, args)
        },
    )
}

/***************************************************************************************************
 * native fun borrow
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/

pub fn native_borrow(
    gas_params: &BorrowGasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(ty_args.len() == 1);
    debug_assert!(args.len() == 2);
    if let Some(err) = check_vector_legacy_supported(context, gas_params.base) {
        return Ok(err);
    }

    let idx = pop_arg!(args, u64) as usize;
    let r = pop_arg!(args, VectorRef);
    NativeResult::map_partial_vm_result_one(
        gas_params.base,
        r.borrow_elem(idx, &ty_args[0])
            .map_err(native_error_to_abort),
    )
}

pub fn make_native_borrow(gas_params: BorrowGasParameters) -> NativeFunction {
    Arc::new(
        move |context, ty_args, args| -> PartialVMResult<NativeResult> {
            native_borrow(&gas_params, context, ty_args, args)
        },
    )
}

/***************************************************************************************************
 * native fun pop
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/

pub fn native_pop_back(
    gas_params: &PopBackGasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(ty_args.len() == 1);
    debug_assert!(args.len() == 1);
    if let Some(err) = check_vector_legacy_supported(context, gas_params.base) {
        return Ok(err);
    }

    let r = pop_arg!(args, VectorRef);
    NativeResult::map_partial_vm_result_one(
        gas_params.base,
        r.pop(&ty_args[0]).map_err(native_error_to_abort),
    )
}

pub fn make_native_pop_back(gas_params: PopBackGasParameters) -> NativeFunction {
    Arc::new(
        move |context, ty_args, args| -> PartialVMResult<NativeResult> {
            native_pop_back(&gas_params, context, ty_args, args)
        },
    )
}

/***************************************************************************************************
 * native fun destroy_empty
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/

pub fn native_destroy_empty(
    gas_params: &DestroyEmptyGasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(ty_args.len() == 1);
    debug_assert!(args.len() == 1);
    if let Some(err) = check_vector_legacy_supported(context, gas_params.base) {
        return Ok(err);
    }

    let v = pop_arg!(args, Vector);
    NativeResult::map_partial_vm_result_empty(
        gas_params.base,
        v.destroy_empty(&ty_args[0]).map_err(native_error_to_abort),
    )
}

pub fn make_native_destroy_empty(gas_params: DestroyEmptyGasParameters) -> NativeFunction {
    Arc::new(
        move |context, ty_args, args| -> PartialVMResult<NativeResult> {
            native_destroy_empty(&gas_params, context, ty_args, args)
        },
    )
}

/***************************************************************************************************
 * native fun swap
 **************************************************************************************************/

pub fn native_swap(
    gas_params: &SwapGasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(ty_args.len() == 1);
    debug_assert!(args.len() == 3);
    if let Some(err) = check_vector_legacy_supported(context, gas_params.base) {
        return Ok(err);
    }

    let idx2 = pop_arg!(args, u64) as usize;
    let idx1 = pop_arg!(args, u64) as usize;
    let r = pop_arg!(args, VectorRef);
    NativeResult::map_partial_vm_result_empty(
        gas_params.base,
        r.swap(idx1, idx2, &ty_args[0])
            .map_err(native_error_to_abort),
    )
}

pub fn make_native_swap(gas_params: SwapGasParameters) -> NativeFunction {
    Arc::new(
        move |context, ty_args, args| -> PartialVMResult<NativeResult> {
            native_swap(&gas_params, context, ty_args, args)
        },
    )
}

fn native_error_to_abort(err: PartialVMError) -> PartialVMError {
    let (major_status, sub_status_opt, message_opt, exec_state_opt, indices, offsets) =
        err.all_data();
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
    let new_err = match exec_state_opt {
        None => new_err,
        Some(stacktrace) => new_err.with_exec_state(stacktrace),
    };
    new_err.at_indices(indices).at_code_offsets(offsets)
}

/***************************************************************************************************
 * module
 **************************************************************************************************/

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [
        ("empty", make_native_empty(gas_params.empty)),
        ("length", make_native_length(gas_params.length)),
        ("push_back", make_native_push_back(gas_params.push_back)),
        ("borrow", make_native_borrow(gas_params.borrow.clone())),
        ("borrow_mut", make_native_borrow(gas_params.borrow)),
        ("pop_back", make_native_pop_back(gas_params.pop_back)),
        (
            "destroy_empty",
            make_native_destroy_empty(gas_params.destroy_empty),
        ),
        ("swap", make_native_swap(gas_params.swap)),
    ];

    make_module_natives(natives)
}
