// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use aptos_gas_schedule::gas_params::natives::aptos_framework::{
    IS_PERMISSIONED_SIGNER_BASE, PERMISSION_ADDRESS_BASE, SIGNER_FROM_PERMISSIONED_HANDLE_BASE,
};
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
use move_core_types::account_address::AccountAddress;
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{SignerRef, Value},
};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

/***************************************************************************************************
 * native fun is_permissioned_signer
 *
 *   Returns true if the signer passed in is a permissioned signer
 *   gas cost: base_cost
 *
 **************************************************************************************************/
fn native_is_permissioned_signer(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(arguments.len() == 1);

    let s_arg = safely_pop_arg!(arguments, SignerRef);

    context.charge(IS_PERMISSIONED_SIGNER_BASE)?;
    let result = s_arg.is_permissioned()?;

    Ok(smallvec![Value::bool(result)])
}

/***************************************************************************************************
 * native fun permission_address
 *
 *   Returns the permission storage address if the signer passed in is a permissioned signer
 *   gas cost: base_cost
 *
 **************************************************************************************************/
fn native_permission_address(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(arguments.len() == 1);

    let s_arg = safely_pop_arg!(arguments, SignerRef);

    context.charge(PERMISSION_ADDRESS_BASE)?;
    if !s_arg.is_permissioned()? {
        return Err(SafeNativeError::Abort { abort_code: 3 });
    }

    Ok(smallvec![s_arg.permissioned_signer()?])
}

/***************************************************************************************************
 * native fun signer_from_permissioned_handle_impl
 *
 *   Returns the permission signer from a master signer.
 *   gas cost: base_cost
 *
 **************************************************************************************************/
fn native_signer_from_permissioned(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(arguments.len() == 2);

    let permission_addr = safely_pop_arg!(arguments, AccountAddress);
    let master_addr = safely_pop_arg!(arguments, AccountAddress);
    context.charge(SIGNER_FROM_PERMISSIONED_HANDLE_BASE)?;

    Ok(smallvec![Value::permissioned_signer(
        master_addr,
        permission_addr
    )])
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [
        (
            "is_permissioned_signer",
            native_is_permissioned_signer as RawSafeNative,
        ),
        ("permission_address", native_permission_address),
        (
            "signer_from_permissioned_handle_impl",
            native_signer_from_permissioned,
        ),
    ];

    builder.make_named_natives(natives)
}
