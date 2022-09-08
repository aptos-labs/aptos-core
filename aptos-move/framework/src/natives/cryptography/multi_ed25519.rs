// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::natives::cryptography::ed25519::GasParameters;
use crate::natives::util::make_native_from_func;
use aptos_crypto::ed25519::{ED25519_PUBLIC_KEY_LENGTH, ED25519_SIGNATURE_LENGTH};
use aptos_crypto::{multi_ed25519, traits::*};
use curve25519_dalek::edwards::CompressedEdwardsY;
use move_deps::move_core_types::gas_algebra::{InternalGasPerArg, NumArgs};
use move_deps::{
    move_binary_format::errors::PartialVMResult,
    move_core_types::gas_algebra::NumBytes,
    move_vm_runtime::native_functions::{NativeContext, NativeFunction},
    move_vm_types::{
        loaded_data::runtime_types::Type, natives::function::NativeResult, pop_arg, values::Value,
    },
};
use smallvec::smallvec;
use std::{collections::VecDeque, convert::TryFrom};

fn native_public_key_validate(
    gas_params: &GasParameters,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 1);

    let pks_bytes = pop_arg!(arguments, Vec<u8>);

    let num_sub_pks = pks_bytes.len() / ED25519_PUBLIC_KEY_LENGTH;

    let mut cost = gas_params.base;

    if num_sub_pks > multi_ed25519::MAX_NUM_OF_KEYS {
        return Ok(NativeResult::ok(cost, smallvec![Value::bool(false)]));
    };

    let num_valid = pks_bytes
        .chunks_exact(ED25519_PUBLIC_KEY_LENGTH)
        .filter(|&pk_bytes| {
            <[u8; ED25519_PUBLIC_KEY_LENGTH]>::try_from(pk_bytes)
                .ok()
                .and_then(|slice| CompressedEdwardsY(slice).decompress())
                .map_or(false, |point| !point.is_small_order())
        })
        .count();

    let all_valid = num_valid == num_sub_pks;
    let mut num_checked = num_valid;
    if !all_valid {
        num_checked += 1;
    }

    let num_checked = NumArgs::new(num_checked as u64);
    cost += gas_params.per_pubkey_deserialize * num_checked
        + gas_params.per_pubkey_small_order_check * num_checked;

    Ok(NativeResult::ok(cost, smallvec![Value::bool(all_valid)]))
}

fn native_signature_verify_strict(
    gas_params: &GasParameters,
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 3);

    let msg = pop_arg!(arguments, Vec<u8>);
    let pubkey = pop_arg!(arguments, Vec<u8>);
    let signature = pop_arg!(arguments, Vec<u8>);

    let mut cost = gas_params.base;

    let num_sub_pks = NumArgs::new((pubkey.len() / ED25519_PUBLIC_KEY_LENGTH) as u64);
    let num_sub_sigs = NumArgs::new((signature.len() / ED25519_SIGNATURE_LENGTH) as u64);

    cost += gas_params.per_pubkey_deserialize * num_sub_pks;
    let pk = match multi_ed25519::MultiEd25519PublicKey::try_from(pubkey.as_slice()) {
        Ok(pk) => pk,
        Err(_) => {
            return Ok(NativeResult::ok(cost, smallvec![Value::bool(false)]));
        }
    };

    cost += gas_params.per_sig_deserialize * num_sub_sigs;
    let sig = match multi_ed25519::MultiEd25519Signature::try_from(signature.as_slice()) {
        Ok(sig) => sig,
        Err(_) => {
            return Ok(NativeResult::ok(cost, smallvec![Value::bool(false)]));
        }
    };

    // TODO(Gas): Have Victor improve type safety here
    cost += gas_params.per_sig_strict_verify * num_sub_sigs
        + gas_params.per_msg_hashing_base * num_sub_sigs
        + InternalGasPerArg::from(u64::from(
            gas_params.per_msg_byte_hashing * NumBytes::new(msg.len() as u64),
        )) * num_sub_sigs;

    let verify_result = sig.verify_arbitrary_msg(msg.as_slice(), &pk).is_ok();
    Ok(NativeResult::ok(
        cost,
        smallvec![Value::bool(verify_result)],
    ))
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [
        // MultiEd25519
        (
            "public_key_validate_internal",
            make_native_from_func(gas_params.clone(), native_public_key_validate),
        ),
        (
            "signature_verify_strict_internal",
            make_native_from_func(gas_params, native_signature_verify_strict),
        ),
    ];

    crate::natives::helpers::make_module_natives(natives)
}
