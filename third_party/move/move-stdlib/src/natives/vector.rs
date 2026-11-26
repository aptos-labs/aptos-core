// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::natives::helpers::make_module_natives;
use move_binary_format::errors::PartialVMResult;
use move_core_types::gas_algebra::{InternalGas, InternalGasPerArg, NumArgs};
use move_vm_runtime::native_functions::{NativeContext, NativeFunction};
use move_vm_types::{
    loaded_data::runtime_types::Type,
    natives::function::NativeResult,
    pop_arg,
    values::{Value, VectorRef},
};
use smallvec::smallvec;
use std::{collections::VecDeque, sync::Arc};

/// Abort code for a range outside the source vector or an insert position past the end of the
/// target vector.
pub const EINDEX_OUT_OF_BOUNDS: u64 = 1;

/***************************************************************************************************
 * native fun move_range
 *
 *   gas cost: base + per_index_moved *
 *             (from_len - removal_position + to_len - insert_position + length)
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct MoveRangeGasParameters {
    pub base: InternalGas,
    pub per_index_moved: InternalGasPerArg,
}

/// Rust implementation of Move's
/// `native public fun move_range<T>(&mut vector<T>, u64, u64, &mut vector<T>, u64)`.
fn native_move_range(
    gas_params: &MoveRangeGasParameters,
    _context: &mut NativeContext,
    ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(ty_args.len() == 1);
    debug_assert!(args.len() == 5);

    let insert_position = pop_arg!(args, u64);
    let to = pop_arg!(args, VectorRef);
    let length = pop_arg!(args, u64);
    let removal_position = pop_arg!(args, u64);
    let from = pop_arg!(args, VectorRef);

    let cost = gas_params.base;
    let (Ok(insert_position), Ok(length), Ok(removal_position)) = (
        usize::try_from(insert_position),
        usize::try_from(length),
        usize::try_from(removal_position),
    ) else {
        return Ok(NativeResult::err(cost, EINDEX_OUT_OF_BOUNDS));
    };
    let to_len = to.length_as_usize()?;
    let from_len = from.length_as_usize()?;
    if removal_position
        .checked_add(length)
        .is_none_or(|end| end > from_len)
        || insert_position > to_len
    {
        return Ok(NativeResult::err(cost, EINDEX_OUT_OF_BOUNDS));
    }

    // Charge for both suffixes, including the source range, plus the inserted range length.
    let moved = (from_len - removal_position)
        .saturating_add(to_len - insert_position)
        .saturating_add(length);
    let cost = cost + gas_params.per_index_moved * NumArgs::new(moved as u64);

    VectorRef::move_range(
        &from,
        removal_position,
        length,
        &to,
        insert_position,
        &ty_args[0],
    )?;

    Ok(NativeResult::ok(cost, smallvec![]))
}

pub fn make_native_move_range(gas_params: MoveRangeGasParameters) -> NativeFunction {
    Arc::new(
        move |context, ty_args, args| -> PartialVMResult<NativeResult> {
            native_move_range(&gas_params, context, ty_args, args)
        },
    )
}

/***************************************************************************************************
 * module
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct GasParameters {
    pub move_range: MoveRangeGasParameters,
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [("move_range", make_native_move_range(gas_params.move_range))];

    make_module_natives(natives)
}
