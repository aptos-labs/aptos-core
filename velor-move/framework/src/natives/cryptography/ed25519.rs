// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "testing")]
use velor_crypto::ed25519::{Ed25519PrivateKey, Ed25519PublicKey};
#[cfg(feature = "testing")]
use velor_crypto::test_utils::KeyPair;
use velor_crypto::{ed25519, ed25519::ED25519_PUBLIC_KEY_LENGTH, traits::*};
use velor_gas_schedule::gas_params::natives::velor_framework::*;
use velor_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
use velor_types::on_chain_config::FeatureFlag;
use curve25519_dalek::edwards::CompressedEdwardsY;
use move_core_types::gas_algebra::{
    InternalGas, InternalGasPerArg, InternalGasPerByte, NumArgs, NumBytes,
};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
#[cfg(feature = "testing")]
use rand_core::OsRng;
use smallvec::{smallvec, SmallVec};
use std::{collections::VecDeque, convert::TryFrom};

pub mod abort_codes {
    pub const E_WRONG_PUBKEY_SIZE: u64 = 1;
    pub const E_WRONG_SIGNATURE_SIZE: u64 = 2;
}

/***************************************************************************************************
 * native fun pubkey_validate_internal
 *
 *   gas cost: base_cost + per_pubkey_deserialize_cost +? per_pubkey_small_order_check
 *
 * where +? indicates that the expression stops evaluating there if the previous gas-charging step
 * failed
 **************************************************************************************************/
fn native_public_key_validate(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 1);

    let key_bytes = safely_pop_arg!(arguments, Vec<u8>);

    context.charge(ED25519_BASE + ED25519_PER_PUBKEY_DESERIALIZE * NumArgs::one())?;

    let key_bytes_slice = match <[u8; ED25519_PUBLIC_KEY_LENGTH]>::try_from(key_bytes) {
        Ok(slice) => slice,
        Err(_) => {
            if context
                .get_feature_flags()
                .is_enabled(FeatureFlag::ED25519_PUBKEY_VALIDATE_RETURN_FALSE_WRONG_LENGTH)
            {
                return Ok(smallvec![Value::bool(false)]);
            } else {
                return Err(SafeNativeError::Abort {
                    abort_code: abort_codes::E_WRONG_PUBKEY_SIZE,
                });
            }
        },
    };

    // This deserialization only performs point-on-curve checks, so we check for small subgroup below
    // NOTE(Gas): O(1) cost: some arithmetic for converting to (X, Y, Z, T) coordinates
    let point = match CompressedEdwardsY(key_bytes_slice).decompress() {
        Some(point) => point,
        None => {
            return Ok(smallvec![Value::bool(false)]);
        },
    };

    // Check if the point lies on a small subgroup. This is required when using curves with a
    // small cofactor (e.g., in Ed25519, cofactor = 8).
    // NOTE(Gas): O(1) cost: multiplies the point by the cofactor
    context.charge(ED25519_PER_PUBKEY_SMALL_ORDER_CHECK * NumArgs::one())?;
    let valid = !point.is_small_order();

    Ok(smallvec![Value::bool(valid)])
}

/***************************************************************************************************
 * native fun signature_verify_strict_internal
 *
 *   gas cost: base_cost + per_pubkey_deserialize_cost
 *                       +? ( per_sig_deserialize_cost
 *                            +? ( per_sig_strict_verify_cost + per_msg_hashing_base_cost
 *                                 + per_msg_byte_hashing_cost * |msg| ) )
 *
 * where +? indicates that the expression stops evaluating there if the previous gas-charging step
 * failed
 **************************************************************************************************/
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

    context.charge(ED25519_PER_PUBKEY_DESERIALIZE * NumArgs::one())?;

    let pk = match ed25519::Ed25519PublicKey::try_from(pubkey.as_slice()) {
        Ok(pk) => pk,
        Err(_) => {
            return Ok(smallvec![Value::bool(false)]);
        },
    };

    context.charge(ED25519_PER_SIG_DESERIALIZE * NumArgs::one())?;

    let sig = match ed25519::Ed25519Signature::try_from(signature.as_slice()) {
        Ok(sig) => sig,
        Err(_) => {
            return Ok(smallvec![Value::bool(false)]);
        },
    };

    // NOTE(Gas): hashing the message to the group and a size-2 multi-scalar multiplication
    let hash_then_verify_cost = ED25519_PER_SIG_STRICT_VERIFY * NumArgs::one()
        + ED25519_PER_MSG_HASHING_BASE * NumArgs::one()
        + ED25519_PER_MSG_BYTE_HASHING * NumBytes::new(msg.len() as u64);
    context.charge(hash_then_verify_cost)?;

    let verify_result = sig.verify_arbitrary_msg(msg.as_slice(), &pk).is_ok();
    Ok(smallvec![Value::bool(verify_result)])
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
#[derive(Debug, Clone)]
pub struct GasParameters {
    pub base: InternalGas,
    pub per_pubkey_deserialize: InternalGasPerArg,
    pub per_pubkey_small_order_check: InternalGasPerArg,
    pub per_sig_deserialize: InternalGasPerArg,
    pub per_sig_strict_verify: InternalGasPerArg,
    pub per_msg_hashing_base: InternalGasPerArg,
    pub per_msg_byte_hashing: InternalGasPerByte, // signature verification involves signing |msg| bytes
}

pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let mut natives = vec![];

    natives.extend([
        (
            "public_key_validate_internal",
            native_public_key_validate as RawSafeNative,
        ),
        (
            "signature_verify_strict_internal",
            native_signature_verify_strict,
        ),
    ]);

    // Test-only natives.
    #[cfg(feature = "testing")]
    natives.extend([
        (
            "generate_keys_internal",
            native_test_only_generate_keys_internal as RawSafeNative,
        ),
        ("sign_internal", native_test_only_sign_internal),
    ]);

    builder.make_named_natives(natives)
}

#[cfg(feature = "testing")]
fn native_test_only_generate_keys_internal(
    _context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let key_pair = KeyPair::<Ed25519PrivateKey, Ed25519PublicKey>::generate(&mut OsRng);
    Ok(smallvec![
        Value::vector_u8(key_pair.private_key.to_bytes()),
        Value::vector_u8(key_pair.public_key.to_bytes())
    ])
}

#[cfg(feature = "testing")]
fn native_test_only_sign_internal(
    _context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let msg_bytes = safely_pop_arg!(args, Vec<u8>);
    let sk_bytes = safely_pop_arg!(args, Vec<u8>);
    let sk = Ed25519PrivateKey::try_from(sk_bytes.as_slice()).unwrap();
    let sig = sk.sign_arbitrary_message(msg_bytes.as_slice());
    Ok(smallvec![Value::vector_u8(sig.to_bytes())])
}
