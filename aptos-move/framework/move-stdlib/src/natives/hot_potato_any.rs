// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use aptos_gas_schedule::gas_params::natives::move_stdlib::*;
use aptos_native_interface::{
    RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeResult,
};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::Value,
};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

/***************************************************************************************************
 * native fun drop
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
#[inline]
fn native_drop(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(ty_args.len() == 1);
    debug_assert!(arguments.len() == 1);

    context.charge(HOT_POTATO_ANY_DROP_BASE)?;

    Ok(smallvec![])
}

/***************************************************************************************************
 * module
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [("drop", native_drop as RawSafeNative)];
    builder.make_named_natives(natives)
}
