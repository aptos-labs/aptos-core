// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Structure constants: additive identity, multiplicative identity and order.

use super::{
    not_implemented, structure_of, AlgebraStore, Structure, BLS12381_GT_GENERATOR,
    BLS12381_Q12_LENDIAN, BLS12381_R_LENDIAN, BN254_GT_GENERATOR, BN254_Q12_LENDIAN,
    BN254_Q_LENDIAN, BN254_R_LENDIAN, E_CONSTANTS_BLS12381FQ12_Q12_ORDER_LOADING_FAILED,
    E_CONSTANTS_BLS12381GT_GT_GENERATOR_LOADING_FAILED,
    E_CONSTANTS_BLS12381_R_ORDER_LOADING_FAILED, E_CONSTANTS_BN254FQ12_Q12_ORDER_LOADING_FAILED,
    E_CONSTANTS_BN254GT_GT_GENERATOR_LOADING_FAILED,
};
use ark_ec::PrimeGroup;
use mono_move_core::{
    native::{NativeContext, NativeStatus},
    VMResult,
};
use num_traits::{One, Zero};
use std::any::Any;

/// Stores `value` and returns its handle in slot 0.
fn constant_op<C: NativeContext, T: Any>(ctx: &C, value: T) -> VMResult<NativeStatus> {
    let mut store = ctx.get_extension::<AlgebraStore>()?;
    let handle = match store.add(value) {
        Ok(handle) => handle,
        Err(abort) => return Ok(abort),
    };
    drop(store);

    // SAFETY: return slot 0 is `u64`.
    unsafe { ctx.set_return(0, handle)? };
    Ok(NativeStatus::Success)
}

/// `0x1::crypto_algebra::zero_internal<S>(): u64`
///
/// TODO(metering): charge gas.
pub fn native_zero<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    match structure_of(ctx.ty_arg(0)?) {
        Some(Structure::BLS12381Fr) => constant_op(ctx, ark_bls12_381::Fr::zero()),
        Some(Structure::BLS12381Fq12) => constant_op(ctx, ark_bls12_381::Fq12::zero()),
        Some(Structure::BLS12381G1) => constant_op(ctx, ark_bls12_381::G1Projective::zero()),
        Some(Structure::BLS12381G2) => constant_op(ctx, ark_bls12_381::G2Projective::zero()),
        Some(Structure::BLS12381Gt) => constant_op(ctx, ark_bls12_381::Fq12::one()),
        Some(Structure::BN254Fr) => constant_op(ctx, ark_bn254::Fr::zero()),
        Some(Structure::BN254Fq) => constant_op(ctx, ark_bn254::Fq::zero()),
        Some(Structure::BN254Fq12) => constant_op(ctx, ark_bn254::Fq12::zero()),
        Some(Structure::BN254G1) => constant_op(ctx, ark_bn254::G1Projective::zero()),
        Some(Structure::BN254G2) => constant_op(ctx, ark_bn254::G2Projective::zero()),
        Some(Structure::BN254Gt) => constant_op(ctx, ark_bn254::Fq12::one()),
        None => Ok(not_implemented()),
    }
}

/// `0x1::crypto_algebra::one_internal<S>(): u64`
///
/// TODO(metering): charge gas.
pub fn native_one<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    match structure_of(ctx.ty_arg(0)?) {
        Some(Structure::BLS12381Fr) => constant_op(ctx, ark_bls12_381::Fr::one()),
        Some(Structure::BLS12381Fq12) => constant_op(ctx, ark_bls12_381::Fq12::one()),
        Some(Structure::BLS12381G1) => constant_op(ctx, ark_bls12_381::G1Projective::generator()),
        Some(Structure::BLS12381G2) => constant_op(ctx, ark_bls12_381::G2Projective::generator()),
        Some(Structure::BLS12381Gt) => match BLS12381_GT_GENERATOR.as_ref() {
            Some(element) => constant_op(ctx, *element),
            None => Ok(NativeStatus::Abort {
                code: E_CONSTANTS_BLS12381GT_GT_GENERATOR_LOADING_FAILED,
                message: Some("BLS12381 GT generator loading failed".to_string()),
            }),
        },
        Some(Structure::BN254Fr) => constant_op(ctx, ark_bn254::Fr::one()),
        Some(Structure::BN254Fq) => constant_op(ctx, ark_bn254::Fq::one()),
        Some(Structure::BN254Fq12) => constant_op(ctx, ark_bn254::Fq12::one()),
        Some(Structure::BN254G1) => constant_op(ctx, ark_bn254::G1Projective::generator()),
        Some(Structure::BN254G2) => constant_op(ctx, ark_bn254::G2Projective::generator()),
        Some(Structure::BN254Gt) => match BN254_GT_GENERATOR.as_ref() {
            Some(element) => constant_op(ctx, *element),
            None => Ok(NativeStatus::Abort {
                code: E_CONSTANTS_BN254GT_GT_GENERATOR_LOADING_FAILED,
                message: Some("BN254 GT generator loading failed".to_string()),
            }),
        },
        None => Ok(not_implemented()),
    }
}

/// `0x1::crypto_algebra::order_internal<S>(): vector<u8>`
///
/// Returns the group order (fields: the characteristic) little-endian.
///
/// TODO(metering): charge gas.
pub fn native_order<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let bytes: &[u8] = match structure_of(ctx.ty_arg(0)?) {
        Some(
            Structure::BLS12381Fr
            | Structure::BLS12381G1
            | Structure::BLS12381G2
            | Structure::BLS12381Gt,
        ) => match BLS12381_R_LENDIAN.as_ref() {
            Some(bytes) => bytes,
            None => {
                return Ok(NativeStatus::Abort {
                    code: E_CONSTANTS_BLS12381_R_ORDER_LOADING_FAILED,
                    message: Some("BLS12381 R order loading failed".to_string()),
                })
            },
        },
        Some(Structure::BLS12381Fq12) => match BLS12381_Q12_LENDIAN.as_ref() {
            Some(bytes) => bytes,
            None => {
                return Ok(NativeStatus::Abort {
                    code: E_CONSTANTS_BLS12381FQ12_Q12_ORDER_LOADING_FAILED,
                    message: Some("BLS12381 Fq12 Q12 order loading failed".to_string()),
                })
            },
        },
        Some(Structure::BN254Fr | Structure::BN254Gt | Structure::BN254G1 | Structure::BN254G2) => {
            BN254_R_LENDIAN.as_slice()
        },
        Some(Structure::BN254Fq) => BN254_Q_LENDIAN.as_slice(),
        Some(Structure::BN254Fq12) => match BN254_Q12_LENDIAN.as_ref() {
            Some(bytes) => bytes,
            None => {
                return Ok(NativeStatus::Abort {
                    code: E_CONSTANTS_BN254FQ12_Q12_ORDER_LOADING_FAILED,
                    message: Some("BN254 Fq12 Q12 order loading failed".to_string()),
                })
            },
        },
        // Legacy's fallback here aborts with the message "Not implemented",
        // but it is dead: the feature-flag check rejects an unknown marker
        // first, with no message, and every known structure has an arm above.
        None => return Ok(not_implemented()),
    };

    let out = ctx.new_byte_vector(bytes)?;
    // SAFETY: return slot 0 is `vector<u8>`.
    unsafe { ctx.set_return(0, out)? };
    Ok(NativeStatus::Success)
}
