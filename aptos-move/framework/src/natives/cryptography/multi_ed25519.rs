// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "testing")]
use crate::natives::helpers::make_test_only_native_from_func;
use crate::{
    natives::{
        cryptography::ed25519::GasParameters,
        helpers::{make_safe_native, SafeNativeContext, SafeNativeResult},
    },
    safely_assert_eq, safely_pop_arg,
};
#[cfg(feature = "testing")]
use aptos_crypto::ed25519::{Ed25519PrivateKey, Ed25519PublicKey};
#[cfg(feature = "testing")]
use aptos_crypto::test_utils::KeyPair;
use aptos_crypto::{
    ed25519,
    ed25519::{ED25519_PUBLIC_KEY_LENGTH, ED25519_SIGNATURE_LENGTH},
    multi_ed25519,
    traits::*,
};
use aptos_types::on_chain_config::{Features, TimedFeatures};
use curve25519_dalek::edwards::CompressedEdwardsY;
#[cfg(feature = "testing")]
use move_binary_format::errors::PartialVMResult;
#[cfg(feature = "testing")]
use move_core_types::gas_algebra::InternalGas;
use move_core_types::gas_algebra::{InternalGasPerArg, NumArgs, NumBytes};
#[cfg(feature = "testing")]
use move_vm_runtime::native_functions::NativeContext;
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
#[cfg(feature = "testing")]
use move_vm_types::{natives::function::NativeResult, pop_arg};
#[cfg(feature = "testing")]
use rand_core::OsRng;
use smallvec::{smallvec, SmallVec};
use std::{collections::VecDeque, convert::TryFrom, sync::Arc};

/// See `public_key_validate_v2_internal` comments in `multi_ed25519.move`.
fn native_public_key_validate_v2(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(_ty_args.len(), 0);
    safely_assert_eq!(arguments.len(), 1);

    let pks_bytes = safely_pop_arg!(arguments, Vec<u8>);

    context.charge(gas_params.base)?;

    // Checks that these bytes correctly-encode a t-out-of-n MultiEd25519 PK
    let (_, num_sub_pks) = match multi_ed25519::check_and_get_threshold(
        &pks_bytes,
        ed25519::ED25519_PUBLIC_KEY_LENGTH,
    ) {
        Ok((t, n)) => (t, n),
        Err(_) => {
            return Ok(smallvec![Value::bool(false)]);
        },
    };

    let num_valid = num_valid_subpks(gas_params, context, pks_bytes)?;
    let all_valid = num_valid == num_sub_pks as usize;

    Ok(smallvec![Value::bool(all_valid)])
}

fn native_public_key_validate_with_gas_fix(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(_ty_args.len(), 0);
    safely_assert_eq!(arguments.len(), 1);

    let pks_bytes = safely_pop_arg!(arguments, Vec<u8>);

    let num_sub_pks = pks_bytes.len() / ED25519_PUBLIC_KEY_LENGTH;

    context.charge(gas_params.base)?;

    if num_sub_pks > multi_ed25519::MAX_NUM_OF_KEYS {
        return Ok(smallvec![Value::bool(false)]);
    };

    let num_valid = num_valid_subpks(gas_params, context, pks_bytes)?;
    let all_valid = num_valid == num_sub_pks;

    Ok(smallvec![Value::bool(all_valid)])
}

fn num_valid_subpks(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    pks_bytes: Vec<u8>,
) -> SafeNativeResult<usize> {
    // Go through all sub-PKs and check that (1) they are valid points and (2) they are NOT small order points.
    let mut num_valid = 0;

    for chunk in pks_bytes.chunks_exact(ED25519_PUBLIC_KEY_LENGTH) {
        // First, we charge for the work.
        context.charge(
            (gas_params.per_pubkey_deserialize + gas_params.per_pubkey_small_order_check)
                * NumArgs::new(1),
        )?;

        // Then, we do the work.
        match <[u8; ED25519_PUBLIC_KEY_LENGTH]>::try_from(chunk) {
            Ok(slice) => {
                if CompressedEdwardsY(slice)
                    .decompress()
                    .map_or(false, |point| !point.is_small_order())
                {
                    num_valid += 1;
                } else {
                    break;
                }
            },
            Err(_) => break,
        }
    }

    Ok(num_valid)
}

fn native_signature_verify_strict(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 3);

    let msg = safely_pop_arg!(arguments, Vec<u8>);
    let pubkey = safely_pop_arg!(arguments, Vec<u8>);
    let signature = safely_pop_arg!(arguments, Vec<u8>);

    context.charge(gas_params.base)?;

    let num_sub_pks = NumArgs::new((pubkey.len() / ED25519_PUBLIC_KEY_LENGTH) as u64);
    let num_sub_sigs = NumArgs::new((signature.len() / ED25519_SIGNATURE_LENGTH) as u64);

    context.charge(gas_params.per_pubkey_deserialize * num_sub_pks)?;
    let pk = match multi_ed25519::MultiEd25519PublicKey::try_from(pubkey.as_slice()) {
        Ok(pk) => pk,
        Err(_) => {
            return Ok(smallvec![Value::bool(false)]);
        },
    };

    context.charge(gas_params.per_sig_deserialize * num_sub_sigs)?;
    let sig = match multi_ed25519::MultiEd25519Signature::try_from(signature.as_slice()) {
        Ok(sig) => sig,
        Err(_) => {
            return Ok(smallvec![Value::bool(false)]);
        },
    };

    // TODO(Gas): Have Victor improve type safety here
    context.charge(
        gas_params.per_sig_strict_verify * num_sub_sigs
            + gas_params.per_msg_hashing_base * num_sub_sigs
            + InternalGasPerArg::from(u64::from(
                gas_params.per_msg_byte_hashing * NumBytes::new(msg.len() as u64),
            )) * num_sub_sigs,
    )?;

    let verify_result = sig.verify_arbitrary_msg(msg.as_slice(), &pk).is_ok();
    Ok(smallvec![Value::bool(verify_result)])
}

#[cfg(feature = "testing")]
fn native_generate_keys(
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let n = pop_arg!(arguments, u8);
    let threshold = pop_arg!(arguments, u8);
    let key_pairs: Vec<KeyPair<Ed25519PrivateKey, Ed25519PublicKey>> = (0..n)
        .map(|_i| KeyPair::<Ed25519PrivateKey, Ed25519PublicKey>::generate(&mut OsRng))
        .collect();
    let private_keys = key_pairs
        .iter()
        .map(|pair| pair.private_key.clone())
        .collect();
    let public_keys = key_pairs
        .iter()
        .map(|pair| pair.public_key.clone())
        .collect();
    let group_sk = multi_ed25519::MultiEd25519PrivateKey::new(private_keys, threshold).unwrap();
    let group_pk = multi_ed25519::MultiEd25519PublicKey::new(public_keys, threshold).unwrap();
    Ok(NativeResult::ok(InternalGas::zero(), smallvec![
        Value::vector_u8(group_sk.to_bytes()),
        Value::vector_u8(group_pk.to_bytes()),
    ]))
}

#[cfg(feature = "testing")]
fn native_sign(
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let message = pop_arg!(arguments, Vec<u8>);
    let sk_bytes = pop_arg!(arguments, Vec<u8>);
    let group_sk = multi_ed25519::MultiEd25519PrivateKey::try_from(sk_bytes.as_slice()).unwrap();
    let sig = group_sk.sign_arbitrary_message(message.as_slice());
    Ok(NativeResult::ok(InternalGas::zero(), smallvec![
        Value::vector_u8(sig.to_bytes()),
    ]))
}
/***************************************************************************************************
 * module
 *
 **************************************************************************************************/

pub fn make_all(
    gas_params: GasParameters,
    timed_features: TimedFeatures,
    features: Arc<Features>,
) -> impl Iterator<Item = (String, NativeFunction)> {
    let mut natives = vec![];
    natives.append(&mut vec![
        // MultiEd25519
        (
            "public_key_validate_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                native_public_key_validate_with_gas_fix,
            ),
        ),
        (
            "public_key_validate_v2_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                native_public_key_validate_v2,
            ),
        ),
        (
            "signature_verify_strict_internal",
            make_safe_native(
                gas_params,
                timed_features,
                features,
                native_signature_verify_strict,
            ),
        ),
    ]);
    #[cfg(feature = "testing")]
    natives.append(&mut vec![
        (
            "generate_keys_internal",
            make_test_only_native_from_func(native_generate_keys),
        ),
        (
            "sign_internal",
            make_test_only_native_from_func(native_sign),
        ),
    ]);
    crate::natives::helpers::make_module_natives(natives)
}
