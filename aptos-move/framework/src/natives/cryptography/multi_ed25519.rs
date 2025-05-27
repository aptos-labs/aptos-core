// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

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
use aptos_gas_algebra::{Arg, GasExpression};
use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_assert_eq, safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext,
    SafeNativeResult,
};
use curve25519_dalek::edwards::CompressedEdwardsY;
use move_core_types::gas_algebra::{NumArgs, NumBytes};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
#[cfg(feature = "testing")]
use rand_core::OsRng;
use smallvec::{smallvec, SmallVec};
use std::{collections::VecDeque, convert::TryFrom};

/// See `public_key_validate_v2_internal` comments in `multi_ed25519.move`.
fn native_public_key_validate_v2(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(_ty_args.len(), 0);
    safely_assert_eq!(arguments.len(), 1);

    let pks_bytes = safely_pop_arg!(arguments, Vec<u8>);

    context.charge(ED25519_BASE)?;

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

    let num_valid = num_valid_subpks(context, pks_bytes)?;
    let all_valid = num_valid == num_sub_pks as usize;

    Ok(smallvec![Value::bool(all_valid)])
}

fn native_public_key_validate_with_gas_fix(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(_ty_args.len(), 0);
    safely_assert_eq!(arguments.len(), 1);

    let pks_bytes = safely_pop_arg!(arguments, Vec<u8>);

    let num_sub_pks = pks_bytes.len() / ED25519_PUBLIC_KEY_LENGTH;

    context.charge(ED25519_BASE)?;

    if num_sub_pks > multi_ed25519::MAX_NUM_OF_KEYS {
        return Ok(smallvec![Value::bool(false)]);
    };

    let num_valid = num_valid_subpks(context, pks_bytes)?;
    let all_valid = num_valid == num_sub_pks;

    Ok(smallvec![Value::bool(all_valid)])
}

fn num_valid_subpks(
    context: &mut SafeNativeContext,
    pks_bytes: Vec<u8>,
) -> SafeNativeResult<usize> {
    // Go through all sub-PKs and check that (1) they are valid points and (2) they are NOT small order points.
    let mut num_valid = 0;

    for chunk in pks_bytes.chunks_exact(ED25519_PUBLIC_KEY_LENGTH) {
        // First, we charge for the work.
        context.charge(
            (ED25519_PER_PUBKEY_DESERIALIZE + ED25519_PER_PUBKEY_SMALL_ORDER_CHECK)
                * NumArgs::new(1),
        )?;

        // Then, we do the work.
        match <[u8; ED25519_PUBLIC_KEY_LENGTH]>::try_from(chunk) {
            Ok(slice) => {
                if CompressedEdwardsY(slice)
                    .decompress()
                    .is_some_and(|point| !point.is_small_order())
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
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 3);

    let msg = safely_pop_arg!(arguments, Vec<u8>);
    let pubkey = safely_pop_arg!(arguments, Vec<u8>);
    let signature = safely_pop_arg!(arguments, Vec<u8>);

    context.charge(ED25519_BASE)?;

    let num_sub_pks = NumArgs::new((pubkey.len() / ED25519_PUBLIC_KEY_LENGTH) as u64);
    let num_sub_sigs = NumArgs::new((signature.len() / ED25519_SIGNATURE_LENGTH) as u64);

    context.charge(ED25519_PER_PUBKEY_DESERIALIZE * num_sub_pks)?;
    let pk = match multi_ed25519::MultiEd25519PublicKey::try_from(pubkey.as_slice()) {
        Ok(pk) => pk,
        Err(_) => {
            return Ok(smallvec![Value::bool(false)]);
        },
    };

    context.charge(ED25519_PER_SIG_DESERIALIZE * num_sub_sigs)?;
    let sig = match multi_ed25519::MultiEd25519Signature::try_from(signature.as_slice()) {
        Ok(sig) => sig,
        Err(_) => {
            return Ok(smallvec![Value::bool(false)]);
        },
    };

    context.charge(
        ED25519_PER_SIG_STRICT_VERIFY * num_sub_sigs
            + ED25519_PER_MSG_HASHING_BASE * num_sub_sigs
            + (ED25519_PER_MSG_BYTE_HASHING * NumBytes::new(msg.len() as u64)).per::<Arg>()
                * num_sub_sigs,
    )?;

    let verify_result = sig.verify_arbitrary_msg(msg.as_slice(), &pk).is_ok();
    Ok(smallvec![Value::bool(verify_result)])
}

#[cfg(feature = "testing")]
fn native_generate_keys(
    _context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let n = safely_pop_arg!(arguments, u8);
    let threshold = safely_pop_arg!(arguments, u8);
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
    Ok(smallvec![
        Value::vector_u8(group_sk.to_bytes()),
        Value::vector_u8(group_pk.to_bytes()),
    ])
}

#[cfg(feature = "testing")]
fn native_sign(
    _context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let message = safely_pop_arg!(arguments, Vec<u8>);
    let sk_bytes = safely_pop_arg!(arguments, Vec<u8>);
    let group_sk = multi_ed25519::MultiEd25519PrivateKey::try_from(sk_bytes.as_slice()).unwrap();
    let sig = group_sk.sign_arbitrary_message(message.as_slice());
    Ok(smallvec![Value::vector_u8(sig.to_bytes()),])
}
/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let mut natives = vec![];

    natives.extend([
        // MultiEd25519
        (
            "public_key_validate_internal",
            native_public_key_validate_with_gas_fix as RawSafeNative,
        ),
        (
            "public_key_validate_v2_internal",
            native_public_key_validate_v2,
        ),
        (
            "signature_verify_strict_internal",
            native_signature_verify_strict,
        ),
    ]);
    #[cfg(feature = "testing")]
    natives.extend([
        (
            "generate_keys_internal",
            native_generate_keys as RawSafeNative,
        ),
        ("sign_internal", native_sign),
    ]);

    builder.make_named_natives(natives)
}
