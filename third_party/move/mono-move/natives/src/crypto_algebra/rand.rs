// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Test-only element sampling.

use super::{
    structure_of, AlgebraStore, Structure, BLS12381_GT_GENERATOR, BN254_GT_GENERATOR,
    E_RAND_BLS12381GT_GT_GENERATOR_LOADING_FAILED, E_RAND_BN254GT_GT_GENERATOR_LOADING_FAILED,
    E_RAND_INSECURE_NOT_IMPLEMENTED,
};
use ark_ff::Field;
use ark_std::{test_rng, UniformRand};
use mono_move_core::{
    native::{NativeContext, NativeStatus},
    VMResult,
};
use std::any::Any;

/// Stores `value` and returns its handle in slot 0.
///
/// The legacy VM gives this native its own message-less copy of the store
/// macro, so a memory-limit abort here carries no message.
fn rand_op<C: NativeContext, T: Any>(ctx: &C, value: T) -> VMResult<NativeStatus> {
    let mut store = ctx.get_extension::<AlgebraStore>()?;
    let handle = match store.add_unmessaged(value) {
        Ok(handle) => handle,
        Err(abort) => return Ok(abort),
    };
    drop(store);

    // SAFETY: return slot 0 is `u64`.
    unsafe { ctx.set_return(0, handle)? };
    Ok(NativeStatus::Success)
}

/// `0x1::crypto_algebra::rand_insecure_internal<S>(): u64`
///
/// TODO(metering): charge gas.
pub fn native_rand_insecure<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    match structure_of(ctx.ty_arg(0)?) {
        Some(Structure::BLS12381Fr) => rand_op(ctx, ark_bls12_381::Fr::rand(&mut test_rng())),
        Some(Structure::BLS12381Fq12) => rand_op(ctx, ark_bls12_381::Fq12::rand(&mut test_rng())),
        Some(Structure::BLS12381G1) => {
            rand_op(ctx, ark_bls12_381::G1Projective::rand(&mut test_rng()))
        },
        Some(Structure::BLS12381G2) => {
            rand_op(ctx, ark_bls12_381::G2Projective::rand(&mut test_rng()))
        },
        Some(Structure::BLS12381Gt) => {
            let Some(generator) = BLS12381_GT_GENERATOR.as_ref() else {
                return Ok(NativeStatus::Abort {
                    code: E_RAND_BLS12381GT_GT_GENERATOR_LOADING_FAILED,
                    message: Some("BLS12381 GT generator loading failed".to_string()),
                });
            };
            let k: ark_ff::BigInteger256 = ark_bls12_381::Fr::rand(&mut test_rng()).into();
            rand_op(ctx, generator.pow(k))
        },
        Some(Structure::BN254Fr) => rand_op(ctx, ark_bn254::Fr::rand(&mut test_rng())),
        Some(Structure::BN254Fq) => rand_op(ctx, ark_bn254::Fq::rand(&mut test_rng())),
        Some(Structure::BN254Fq12) => rand_op(ctx, ark_bn254::Fq12::rand(&mut test_rng())),
        Some(Structure::BN254G1) => rand_op(ctx, ark_bn254::G1Projective::rand(&mut test_rng())),
        Some(Structure::BN254G2) => rand_op(ctx, ark_bn254::G2Projective::rand(&mut test_rng())),
        Some(Structure::BN254Gt) => {
            let Some(generator) = BN254_GT_GENERATOR.as_ref() else {
                return Ok(NativeStatus::Abort {
                    code: E_RAND_BN254GT_GT_GENERATOR_LOADING_FAILED,
                    message: Some("BN254 GT generator loading failed".to_string()),
                });
            };
            let k: ark_ff::BigInteger256 = ark_bn254::Fr::rand(&mut test_rng()).into();
            rand_op(ctx, generator.pow(k))
        },
        // Unlike every other native's fallback, this one has its own code and
        // carries a message.
        None => Ok(NativeStatus::Abort {
            code: E_RAND_INSECURE_NOT_IMPLEMENTED,
            message: Some("Not implemented".to_string()),
        }),
    }
}
