// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::{ed25519, traits::*};
use move_deps::{
    move_binary_format::errors::PartialVMResult,
    move_core_types::gas_schedule::GasCost,
    move_vm_runtime::native_functions::NativeContext,
    move_vm_types::{
        gas_schedule::NativeCostIndex,
        loaded_data::runtime_types::Type,
        natives::function::{native_gas, NativeResult},
        pop_arg,
        values::Value,
    },
};
use smallvec::smallvec;
use std::{collections::VecDeque, convert::TryFrom};

pub fn native_ed25519_publickey_validation(
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 1);

    let key_bytes = pop_arg!(arguments, Vec<u8>);

    let cost = native_gas(
        context.cost_table(),
        NativeCostIndex::ED25519_VALIDATE_KEY,
        key_bytes.len(),
    );

    // This deserialization performs point-on-curve and small subgroup checks
    let valid = ed25519::Ed25519PublicKey::try_from(&key_bytes[..]).is_ok();
    Ok(NativeResult::ok(cost, smallvec![Value::bool(valid)]))
}

pub fn native_ed25519_signature_verification(
    context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 3);

    let msg = pop_arg!(arguments, Vec<u8>);
    let pubkey = pop_arg!(arguments, Vec<u8>);
    let signature = pop_arg!(arguments, Vec<u8>);

    let cost = native_gas(
        context.cost_table(),
        NativeCostIndex::ED25519_VERIFY,
        msg.len(),
    );

    let sig = match ed25519::Ed25519Signature::try_from(signature.as_slice()) {
        Ok(sig) => sig,
        Err(_) => {
            return Ok(NativeResult::ok(cost, smallvec![Value::bool(false)]));
        }
    };
    let pk = match ed25519::Ed25519PublicKey::try_from(pubkey.as_slice()) {
        Ok(pk) => pk,
        Err(_) => {
            return Ok(NativeResult::ok(cost, smallvec![Value::bool(false)]));
        }
    };

    let verify_result = sig.verify_arbitrary_msg(msg.as_slice(), &pk).is_ok();
    Ok(NativeResult::ok(
        cost,
        smallvec![Value::bool(verify_result)],
    ))
}

pub fn native_secp256k1_recover(
    _context: &mut NativeContext,
    _ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.len() == 3);

    let signature = pop_arg!(arguments, Vec<u8>);
    let recovery_id = pop_arg!(arguments, u8);
    let msg = pop_arg!(arguments, Vec<u8>);

    let cost = GasCost::new(super::cost::APTOS_SECP256K1_RECOVER, 1).total();

    let msg = match libsecp256k1::Message::parse_slice(&msg) {
        Ok(msg) => msg,
        Err(_) => {
            return Ok(NativeResult::ok(
                cost,
                smallvec![Value::vector_u8([0u8; 0]), Value::bool(false)],
            ));
        }
    };
    let rid = match libsecp256k1::RecoveryId::parse(recovery_id) {
        Ok(rid) => rid,
        Err(_) => {
            return Ok(NativeResult::ok(
                cost,
                smallvec![Value::vector_u8([0u8; 0]), Value::bool(false)],
            ));
        }
    };
    let sig = match libsecp256k1::Signature::parse_standard_slice(&signature) {
        Ok(sig) => sig,
        Err(_) => {
            return Ok(NativeResult::ok(
                cost,
                smallvec![Value::vector_u8([0u8; 0]), Value::bool(false)],
            ));
        }
    };

    let pk = match libsecp256k1::recover(&msg, &sig, &rid) {
        Ok(pk) => pk,
        Err(_) => {
            return Ok(NativeResult::ok(
                cost,
                smallvec![Value::vector_u8([0u8; 0]), Value::bool(false)],
            ));
        }
    };

    Ok(NativeResult::ok(
        cost,
        smallvec![
            Value::vector_u8(pk.serialize()[1..].to_vec()),
            Value::bool(true)
        ],
    ))
}
