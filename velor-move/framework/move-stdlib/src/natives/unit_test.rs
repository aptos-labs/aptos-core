// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use velor_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeResult,
};
use move_core_types::account_address::AccountAddress;
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

/***************************************************************************************************
 * native fun create_signers_for_testing
 *
 *   gas cost: base_cost + unit_cost * num_of_signers
 *
 **************************************************************************************************/
fn to_le_bytes(i: u64) -> [u8; AccountAddress::LENGTH] {
    let bytes = i.to_le_bytes();
    let mut result = [0u8; AccountAddress::LENGTH];
    result[..bytes.len()].clone_from_slice(bytes.as_ref());
    result
}

fn native_create_signers_for_testing(
    _context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(ty_args.is_empty());
    debug_assert!(args.len() == 1);

    let num_signers = safely_pop_arg!(args, u64);

    let signers = Value::vector_for_testing_only(
        (0..num_signers).map(|i| Value::master_signer(AccountAddress::new(to_le_bytes(i)))),
    );

    Ok(smallvec![signers])
}

/***************************************************************************************************
 * module
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [(
        "create_signers_for_testing",
        native_create_signers_for_testing as RawSafeNative,
    )];

    builder.make_named_natives(natives)
}
