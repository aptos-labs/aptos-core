// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The permissioned signer feature has been removed and the current framework no longer
//! declares these natives. The registrations must remain so that historical versions of
//! `0x1::permissioned_signer`, which declare them, can still be loaded (e.g. during replay).
//! The feature flag was never enabled on a public network, so the observable behavior of
//! every historical call is the flag-disabled path preserved below.

use aptos_gas_schedule::gas_params::natives::move_stdlib::SIGNER_BORROW_ADDRESS_BASE;
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{SignerRef, Value},
};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

const EPERMISSION_SIGNER_DISABLED: u64 = 9;

/// Shared implementation for the removed permissioned signer natives: abort exactly like
/// the historical implementations did when the feature flag was disabled (no gas charged,
/// abort code 9).
fn native_permissioned_signer_removed(
    _context: &mut SafeNativeContext,
    _ty_args: &[Type],
    _arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    Err(SafeNativeError::abort(EPERMISSION_SIGNER_DISABLED))
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
    _ty_args: &[Type],
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 1);

    let signer_reference = safely_pop_arg!(arguments, SignerRef);

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
            native_permissioned_signer_removed as RawSafeNative,
        ),
        (
            "is_permissioned_signer",
            native_permissioned_signer_removed as RawSafeNative,
        ),
        ("permission_address", native_permissioned_signer_removed),
        (
            "signer_from_permissioned_handle_impl",
            native_permissioned_signer_removed,
        ),
        ("borrow_address", native_borrow_address),
    ];

    builder.make_named_natives(natives)
}
