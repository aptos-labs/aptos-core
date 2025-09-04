// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0
use velor_gas_schedule::gas_params::natives::{
    velor_framework::{
        IS_PERMISSIONED_SIGNER_BASE, PERMISSION_ADDRESS_BASE, SIGNER_FROM_PERMISSIONED_HANDLE_BASE,
    },
    move_stdlib::SIGNER_BORROW_ADDRESS_BASE,
};
use velor_native_interface::{
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

const EPERMISSION_SIGNER_DISABLED: u64 = 9;

/***************************************************************************************************
 * native fun is_permissioned_signer_impl
 *
 *   Returns true if the signer passed in is a permissioned signer
 *   gas cost: base_cost
 *
 **************************************************************************************************/
fn native_is_permissioned_signer_impl(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(arguments.len() == 1);

    if !context
        .get_feature_flags()
        .is_enabled(velor_types::on_chain_config::FeatureFlag::PERMISSIONED_SIGNER)
    {
        return SafeNativeResult::Err(SafeNativeError::Abort {
            abort_code: EPERMISSION_SIGNER_DISABLED,
        });
    }

    let signer = safely_pop_arg!(arguments, SignerRef);

    context.charge(IS_PERMISSIONED_SIGNER_BASE)?;
    let result = signer.is_permissioned()?;

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
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(args.len() == 1);

    if !context
        .get_feature_flags()
        .is_enabled(velor_types::on_chain_config::FeatureFlag::PERMISSIONED_SIGNER)
    {
        return SafeNativeResult::Err(SafeNativeError::Abort {
            abort_code: EPERMISSION_SIGNER_DISABLED,
        });
    }

    let signer = safely_pop_arg!(args, SignerRef);

    context.charge(PERMISSION_ADDRESS_BASE)?;
    if !signer.is_permissioned()? {
        return Err(SafeNativeError::Abort { abort_code: 3 });
    }

    Ok(smallvec![signer.permission_address()?])
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

    if !context
        .get_feature_flags()
        .is_enabled(velor_types::on_chain_config::FeatureFlag::PERMISSIONED_SIGNER)
    {
        return SafeNativeResult::Err(SafeNativeError::Abort {
            abort_code: EPERMISSION_SIGNER_DISABLED,
        });
    }

    let permission_addr = safely_pop_arg!(arguments, AccountAddress);
    let master_addr = safely_pop_arg!(arguments, AccountAddress);
    context.charge(SIGNER_FROM_PERMISSIONED_HANDLE_BASE)?;

    Ok(smallvec![Value::permissioned_signer(
        master_addr,
        permission_addr
    )])
}

/***************************************************************************************************
 * native fun borrow_address
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
#[inline]
fn native_borrow_address(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 1);

    let signer_reference = safely_pop_arg!(arguments, SignerRef);

    if !context
        .get_feature_flags()
        .is_enabled(velor_types::on_chain_config::FeatureFlag::PERMISSIONED_SIGNER)
        && signer_reference.is_permissioned()?
    {
        return SafeNativeResult::Err(SafeNativeError::Abort {
            abort_code: EPERMISSION_SIGNER_DISABLED,
        });
    }

    context.charge(SIGNER_BORROW_ADDRESS_BASE)?;

    Ok(smallvec![signer_reference.borrow_signer()?])
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
            "is_permissioned_signer_impl",
            native_is_permissioned_signer_impl as RawSafeNative,
        ),
        (
            "is_permissioned_signer",
            native_is_permissioned_signer_impl as RawSafeNative,
        ),
        ("permission_address", native_permission_address),
        (
            "signer_from_permissioned_handle_impl",
            native_signer_from_permissioned,
        ),
        ("borrow_address", native_borrow_address),
    ];

    builder.make_named_natives(natives)
}
