// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use move_binary_format::errors::PartialVMResult;
use move_core_types::{account_address::AccountAddress, gas_algebra::InternalGas, ident_str};
use move_vm_runtime::native_functions::{NativeContext, NativeFunction, NativeFunctionTable};
use move_vm_types::{
    loaded_data::runtime_types::Type,
    natives::function::NativeResult,
    pop_arg,
    values::{Value, VectorRef},
};
use smallvec::smallvec;
use std::{collections::VecDeque, sync::Arc};

fn v1_native_u64_add(
    _ctx: &mut NativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let b = pop_arg!(args, u64);
    let a = pop_arg!(args, u64);
    match a.checked_add(b) {
        Some(sum) => Ok(NativeResult::ok(InternalGas::zero(), smallvec![
            Value::u64(sum)
        ])),
        None => Ok(NativeResult::err(InternalGas::zero(), 1)),
    }
}

fn v1_native_u64_identity(
    _ctx: &mut NativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let x = pop_arg!(args, u64);
    Ok(NativeResult::ok(InternalGas::zero(), smallvec![
        Value::u64(x)
    ]))
}

/// Mirrors the legacy `0x1::vector::move_range` native, registered under a
/// test-only module name. The real `vector` module in the test stdlib does not
/// declare `move_range`, so the differential test calls it through this alias.
fn v1_native_move_range(
    _ctx: &mut NativeContext,
    ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let insert_position = pop_arg!(args, u64) as usize;
    let to = pop_arg!(args, VectorRef);
    let length = pop_arg!(args, u64) as usize;
    let removal_position = pop_arg!(args, u64) as usize;
    let from = pop_arg!(args, VectorRef);

    let to_len = to.length_as_usize()?;
    let from_len = from.length_as_usize()?;
    if removal_position
        .checked_add(length)
        .is_none_or(|end| end > from_len)
        || insert_position > to_len
    {
        // EINDEX_OUT_OF_BOUNDS, matching the v2 native and the real one.
        return Ok(NativeResult::err(InternalGas::zero(), 1));
    }
    VectorRef::move_range(
        &from,
        removal_position,
        length,
        &to,
        insert_position,
        &ty_args[0],
    )?;
    Ok(NativeResult::ok(InternalGas::zero(), smallvec![]))
}

/// Build a list of test natives for the v1 VM, matching the ones we have for v2
/// (in the `mono-move-natives` crate).
///
/// These exist solely so the differential harness can register the same
/// set of natives on both VMs and compare their outputs side by side.
pub fn make_all_v1_test_natives() -> NativeFunctionTable {
    let module = ident_str!("test_natives").to_owned();
    vec![
        (
            AccountAddress::ONE,
            module.clone(),
            ident_str!("u64_add").to_owned(),
            Arc::new(v1_native_u64_add) as NativeFunction,
        ),
        (
            AccountAddress::ONE,
            module,
            ident_str!("u64_identity").to_owned(),
            Arc::new(v1_native_u64_identity) as NativeFunction,
        ),
        (
            AccountAddress::ONE,
            ident_str!("vector_natives").to_owned(),
            ident_str!("move_range").to_owned(),
            Arc::new(v1_native_move_range) as NativeFunction,
        ),
    ]
}
