// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::natives::create_signer;
use velor_gas_schedule::gas_params::natives::velor_framework::*;
use velor_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
use move_core_types::{account_address::AccountAddress, gas_algebra::InternalGas};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

/***************************************************************************************************
 * native fun create_address
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct CreateAddressGasParameters {
    pub base: InternalGas,
}

fn native_create_address(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(ty_args.is_empty());
    debug_assert!(arguments.len() == 1);

    context.charge(ACCOUNT_CREATE_ADDRESS_BASE)?;

    let bytes = safely_pop_arg!(arguments, Vec<u8>);
    let address = AccountAddress::from_bytes(bytes);
    if let Ok(address) = address {
        Ok(smallvec![Value::address(address)])
    } else {
        Err(SafeNativeError::Abort {
            abort_code: super::status::NFE_UNABLE_TO_PARSE_ADDRESS,
        })
    }
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [
        ("create_address", native_create_address as RawSafeNative),
        // Despite that this is no longer present in account.move, we must keep this around for
        // replays.
        ("create_signer", create_signer::native_create_signer),
    ];

    builder.make_named_natives(natives)
}
