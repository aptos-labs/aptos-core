// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use aptos_gas_schedule::gas_params::natives::{
    aptos_framework::IS_PERMISSIONED_SIGNER_BASE, move_stdlib::*,
};
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{values_impl::SignerRef, Value},
};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

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

    context.charge(SIGNER_BORROW_ADDRESS_BASE)?;

    if context
        .get_feature_flags()
        .is_enabled(aptos_types::on_chain_config::FeatureFlag::PERMISSIONED_SIGNER)
        && signer_reference.is_permissioned()?
    {
        return Err(SafeNativeError::Abort { abort_code: 1 });
    }

    Ok(smallvec![signer_reference.borrow_signer()?])
}

/***************************************************************************************************
 * native fun borrow_address_unpermissioned
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
#[inline]
fn native_borrow_address_unpermissioned(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
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
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [
        ("borrow_address", native_borrow_address as RawSafeNative),
        (
            "borrow_address_unpermissioned",
            native_borrow_address_unpermissioned,
        ),
    ];
    builder.make_named_natives(natives)
}
