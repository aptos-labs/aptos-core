// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_crypto::{
    bls12381,
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    multi_ed25519::{MultiEd25519PrivateKey, MultiEd25519PublicKey},
    test_utils::KeyPair,
    SigningKey, Uniform,
};
use move_binary_format::errors::PartialVMResult;
use move_core_types::{account_address::AccountAddress, gas_algebra::InternalGas, ident_str};
use move_vm_runtime::native_functions::{NativeContext, NativeFunction, NativeFunctionTable};
use move_vm_types::{
    loaded_data::runtime_types::Type, natives::function::NativeResult, pop_arg, values::Value,
};
use rand_core::OsRng;
use smallvec::smallvec;
use std::{collections::VecDeque, sync::Arc};

fn v1_native_u64_add(
    _ctx: &mut NativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let b = pop_arg!(args, u64);
    let a = pop_arg!(args, u64);
    match a.checked_add(b) {
        Some(sum) => Ok(NativeResult::ok(InternalGas::zero(), smallvec![
            Value::u64(sum)
        ])),
        None => Ok(NativeResult::err(InternalGas::zero(), 1)),
    }
}

fn v1_native_u64_identity(
    _ctx: &mut NativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let x = pop_arg!(args, u64);
    Ok(NativeResult::ok(InternalGas::zero(), smallvec![
        Value::u64(x)
    ]))
}

/// The address byte array for index `i`, with `i` little-endian encoded in the
/// first 8 bytes.
fn address_bytes_for_index(i: u64) -> [u8; AccountAddress::LENGTH] {
    let bytes = i.to_le_bytes();
    let mut result = [0u8; AccountAddress::LENGTH];
    result[..bytes.len()].clone_from_slice(bytes.as_ref());
    result
}

/// Mirrors `std::unit_test::create_signers_for_testing`
/// (`aptos-move/framework/move-stdlib/src/natives/unit_test.rs`): returns
/// `num_signers` master signers whose addresses are the little-endian encodings
/// of `0, 1, ..., num_signers - 1`.
///
/// Registered here because the testsuite does not enable
/// `aptos-move-stdlib/testing`, so `aptos_natives` omits this test-only native;
/// the V2 side registers it via `make_all_unit_test_natives`.
fn v1_native_create_signers_for_testing(
    _ctx: &mut NativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let num_signers = pop_arg!(args, u64);
    let signers = Value::vector_unchecked(
        (0..num_signers).map(|i| Value::signer(AccountAddress::new(address_bytes_for_index(i)))),
    )?;
    Ok(NativeResult::ok(InternalGas::zero(), smallvec![signers]))
}

// Abort codes mirrored from the V1 crypto natives, kept so both VMs abort with
// the same code on the same failure.
const E_ED25519_SIGN_COMPUTATION_FAILED: u64 = 0x0A_0001;
const E_MULTI_ED25519_SK_CREATE_FAILED: u64 = 0x0A_0001;
const E_MULTI_ED25519_PK_CREATE_FAILED: u64 = 0x0A_0002;
const E_MULTI_ED25519_SK_DESERIALIZATION_FAILED: u64 = 0x0A_0003;
const E_BLS12381_SIGN_COMPUTATION_FAILED: u64 = 0x0A_0001;
const E_BLS12381_POP_SK_DESERIALIZATION_FAILED: u64 = 0x0A_0002;

/// Mirrors `aptos_std::ed25519::generate_keys_internal`
/// (`framework/natives/src/cryptography/ed25519.rs`): returns a fresh
/// `(private_key, public_key)` byte pair.
fn v1_native_ed25519_generate_keys(
    _ctx: &mut NativeContext,
    _ty_args: &[Type],
    _args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let key_pair = KeyPair::<Ed25519PrivateKey, Ed25519PublicKey>::generate(&mut OsRng);
    Ok(NativeResult::ok(InternalGas::zero(), smallvec![
        Value::vector_u8(key_pair.private_key.to_bytes()),
        Value::vector_u8(key_pair.public_key.to_bytes()),
    ]))
}

/// Mirrors `aptos_std::ed25519::sign_internal`: signs `msg` under `sk`.
fn v1_native_ed25519_sign(
    _ctx: &mut NativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let msg_bytes = pop_arg!(args, Vec<u8>);
    let sk_bytes = pop_arg!(args, Vec<u8>);
    let sk = match Ed25519PrivateKey::try_from(sk_bytes.as_slice()) {
        Ok(sk) => sk,
        Err(_) => {
            return Ok(NativeResult::err(
                InternalGas::zero(),
                E_ED25519_SIGN_COMPUTATION_FAILED,
            ))
        },
    };
    let sig = sk.sign_arbitrary_message(msg_bytes.as_slice());
    Ok(NativeResult::ok(InternalGas::zero(), smallvec![
        Value::vector_u8(sig.to_bytes())
    ]))
}

/// Mirrors `aptos_std::multi_ed25519::generate_keys_internal`
/// (`framework/natives/src/cryptography/multi_ed25519.rs`): returns a fresh
/// `threshold`-of-`n` `(private_key, public_key)` byte pair.
fn v1_native_multi_ed25519_generate_keys(
    _ctx: &mut NativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let n = pop_arg!(args, u8);
    let threshold = pop_arg!(args, u8);
    let key_pairs = (0..n)
        .map(|_| KeyPair::<Ed25519PrivateKey, Ed25519PublicKey>::generate(&mut OsRng))
        .collect::<Vec<_>>();
    let private_keys = key_pairs
        .iter()
        .map(|pair| pair.private_key.clone())
        .collect();
    let public_keys = key_pairs
        .iter()
        .map(|pair| pair.public_key.clone())
        .collect();
    let group_sk = match MultiEd25519PrivateKey::new(private_keys, threshold) {
        Ok(sk) => sk,
        Err(_) => {
            return Ok(NativeResult::err(
                InternalGas::zero(),
                E_MULTI_ED25519_SK_CREATE_FAILED,
            ))
        },
    };
    let group_pk = match MultiEd25519PublicKey::new(public_keys, threshold) {
        Ok(pk) => pk,
        Err(_) => {
            return Ok(NativeResult::err(
                InternalGas::zero(),
                E_MULTI_ED25519_PK_CREATE_FAILED,
            ))
        },
    };
    Ok(NativeResult::ok(InternalGas::zero(), smallvec![
        Value::vector_u8(group_sk.to_bytes()),
        Value::vector_u8(group_pk.to_bytes()),
    ]))
}

/// Mirrors `aptos_std::multi_ed25519::sign_internal`: signs `message` under the
/// aggregate private key `sk`.
fn v1_native_multi_ed25519_sign(
    _ctx: &mut NativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let message = pop_arg!(args, Vec<u8>);
    let sk_bytes = pop_arg!(args, Vec<u8>);
    let group_sk = match MultiEd25519PrivateKey::try_from(sk_bytes.as_slice()) {
        Ok(sk) => sk,
        Err(_) => {
            return Ok(NativeResult::err(
                InternalGas::zero(),
                E_MULTI_ED25519_SK_DESERIALIZATION_FAILED,
            ))
        },
    };
    let sig = group_sk.sign_arbitrary_message(message.as_slice());
    Ok(NativeResult::ok(InternalGas::zero(), smallvec![
        Value::vector_u8(sig.to_bytes())
    ]))
}

/// Mirrors `aptos_std::bls12381::generate_keys_internal`
/// (`framework/natives/src/cryptography/bls12381.rs`): returns a fresh
/// `(private_key, public_key)` byte pair.
fn v1_native_bls12381_generate_keys(
    _ctx: &mut NativeContext,
    _ty_args: &[Type],
    _args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let key_pair = KeyPair::<bls12381::PrivateKey, bls12381::PublicKey>::generate(&mut OsRng);
    Ok(NativeResult::ok(InternalGas::zero(), smallvec![
        Value::vector_u8(key_pair.private_key.to_bytes()),
        Value::vector_u8(key_pair.public_key.to_bytes()),
    ]))
}

/// Mirrors `aptos_std::bls12381::sign_internal`: signs `msg` under `sk`.
fn v1_native_bls12381_sign(
    _ctx: &mut NativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let msg = pop_arg!(args, Vec<u8>);
    let sk_bytes = pop_arg!(args, Vec<u8>);
    let sk = match bls12381::PrivateKey::try_from(sk_bytes.as_slice()) {
        Ok(sk) => sk,
        Err(_) => {
            return Ok(NativeResult::err(
                InternalGas::zero(),
                E_BLS12381_SIGN_COMPUTATION_FAILED,
            ))
        },
    };
    let sig = sk.sign_arbitrary_message(msg.as_slice());
    Ok(NativeResult::ok(InternalGas::zero(), smallvec![
        Value::vector_u8(sig.to_bytes())
    ]))
}

/// Mirrors `aptos_std::bls12381::generate_proof_of_possession_internal`:
/// creates a PoP for the private key `sk`.
fn v1_native_bls12381_generate_proof_of_possession(
    _ctx: &mut NativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    let sk_bytes = pop_arg!(args, Vec<u8>);
    let sk = match bls12381::PrivateKey::try_from(sk_bytes.as_slice()) {
        Ok(sk) => sk,
        Err(_) => {
            return Ok(NativeResult::err(
                InternalGas::zero(),
                E_BLS12381_POP_SK_DESERIALIZATION_FAILED,
            ))
        },
    };
    let pop = bls12381::ProofOfPossession::create(&sk);
    Ok(NativeResult::ok(InternalGas::zero(), smallvec![
        Value::vector_u8(pop.to_bytes())
    ]))
}

/// Build a list of test natives for the v1 VM, matching the ones we have for v2
/// (in the `mono-move-natives` crate).
///
/// These exist solely so the differential harness can register the same
/// set of natives on both VMs and compare their outputs side by side.
pub fn make_all_v1_test_natives() -> NativeFunctionTable {
    let module = ident_str!("test_natives").to_owned();
    vec![
        (
            AccountAddress::ONE,
            module.clone(),
            ident_str!("u64_add").to_owned(),
            Arc::new(v1_native_u64_add) as NativeFunction,
        ),
        (
            AccountAddress::ONE,
            module,
            ident_str!("u64_identity").to_owned(),
            Arc::new(v1_native_u64_identity) as NativeFunction,
        ),
        (
            AccountAddress::ONE,
            ident_str!("unit_test").to_owned(),
            ident_str!("create_signers_for_testing").to_owned(),
            Arc::new(v1_native_create_signers_for_testing) as NativeFunction,
        ),
        (
            AccountAddress::ONE,
            ident_str!("ed25519").to_owned(),
            ident_str!("generate_keys_internal").to_owned(),
            Arc::new(v1_native_ed25519_generate_keys) as NativeFunction,
        ),
        (
            AccountAddress::ONE,
            ident_str!("ed25519").to_owned(),
            ident_str!("sign_internal").to_owned(),
            Arc::new(v1_native_ed25519_sign) as NativeFunction,
        ),
        (
            AccountAddress::ONE,
            ident_str!("multi_ed25519").to_owned(),
            ident_str!("generate_keys_internal").to_owned(),
            Arc::new(v1_native_multi_ed25519_generate_keys) as NativeFunction,
        ),
        (
            AccountAddress::ONE,
            ident_str!("multi_ed25519").to_owned(),
            ident_str!("sign_internal").to_owned(),
            Arc::new(v1_native_multi_ed25519_sign) as NativeFunction,
        ),
        (
            AccountAddress::ONE,
            ident_str!("bls12381").to_owned(),
            ident_str!("generate_keys_internal").to_owned(),
            Arc::new(v1_native_bls12381_generate_keys) as NativeFunction,
        ),
        (
            AccountAddress::ONE,
            ident_str!("bls12381").to_owned(),
            ident_str!("sign_internal").to_owned(),
            Arc::new(v1_native_bls12381_sign) as NativeFunction,
        ),
        (
            AccountAddress::ONE,
            ident_str!("bls12381").to_owned(),
            ident_str!("generate_proof_of_possession_internal").to_owned(),
            Arc::new(v1_native_bls12381_generate_proof_of_possession) as NativeFunction,
        ),
    ]
}
