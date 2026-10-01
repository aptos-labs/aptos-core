// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Scalar multiplication of a group element, and multi-scalar multiplication.

use super::{
    any_structure, not_implemented, return_handle, structure_of, AlgebraStore, Structure,
    E_SCALAR_MUL_MSM_COMPUTATION_FAILED, MOVE_ABORT_CODE_INPUT_VECTOR_SIZES_NOT_MATCHING,
};
use ark_ec::{CurveGroup, PrimeGroup};
use ark_ff::Field;
use mono_move_core::{
    native::{NativeContext, NativeStatus, Vector},
    VMResult,
};
use std::any::Any;

/// Stores `op(element, scalar)` and returns its handle in slot 0.
fn scalar_mul_op<C: NativeContext, G, S>(
    ctx: &C,
    element_handle: u64,
    scalar_handle: u64,
    op: impl FnOnce(G, ark_ff::BigInteger256) -> G,
) -> VMResult<NativeStatus>
where
    G: Any + Copy,
    S: Any + Copy + Into<ark_ff::BigInteger256>,
{
    let mut store = ctx.get_extension::<AlgebraStore>()?;
    let element = store.get::<G>(element_handle)?;
    let scalar = store.get::<S>(scalar_handle)?;
    let added = store.add(op(element, scalar.into()));
    return_handle(ctx, store, added)
}

/// `0x1::crypto_algebra::scalar_mul_internal<G, S>(element_handle: u64, scalar_handle: u64): u64`
///
/// TODO(metering): charge gas.
pub fn native_scalar_mul<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let group = structure_of(ctx.ty_arg(0)?);
    let scalar_field = structure_of(ctx.ty_arg(1)?);
    // SAFETY: args 0 and 1 are `u64` handles.
    let element_handle = unsafe { ctx.arg::<u64>(0)? };
    let scalar_handle = unsafe { ctx.arg::<u64>(1)? };

    match (group, scalar_field) {
        (Some(Structure::BLS12381G1), Some(Structure::BLS12381Fr)) => {
            scalar_mul_op::<_, ark_bls12_381::G1Projective, ark_bls12_381::Fr>(
                ctx,
                element_handle,
                scalar_handle,
                |element, scalar| element.mul_bigint(scalar),
            )
        },
        (Some(Structure::BLS12381G2), Some(Structure::BLS12381Fr)) => {
            scalar_mul_op::<_, ark_bls12_381::G2Projective, ark_bls12_381::Fr>(
                ctx,
                element_handle,
                scalar_handle,
                |element, scalar| element.mul_bigint(scalar),
            )
        },
        (Some(Structure::BLS12381Gt), Some(Structure::BLS12381Fr)) => {
            scalar_mul_op::<_, ark_bls12_381::Fq12, ark_bls12_381::Fr>(
                ctx,
                element_handle,
                scalar_handle,
                |element, scalar| element.pow(scalar),
            )
        },
        (Some(Structure::BN254G1), Some(Structure::BN254Fr)) => {
            scalar_mul_op::<_, ark_bn254::G1Projective, ark_bn254::Fr>(
                ctx,
                element_handle,
                scalar_handle,
                |element, scalar| element.mul_bigint(scalar),
            )
        },
        (Some(Structure::BN254G2), Some(Structure::BN254Fr)) => {
            scalar_mul_op::<_, ark_bn254::G2Projective, ark_bn254::Fr>(
                ctx,
                element_handle,
                scalar_handle,
                |element, scalar| element.mul_bigint(scalar),
            )
        },
        (Some(Structure::BN254Gt), Some(Structure::BN254Fr)) => {
            scalar_mul_op::<_, ark_bn254::Fq12, ark_bn254::Fr>(
                ctx,
                element_handle,
                scalar_handle,
                |element, scalar| element.pow(scalar),
            )
        },
        (any_structure!(), any_structure!()) => Ok(not_implemented()),
    }
}

/// Stores `sum_i scalars[i] * bases[i]` and returns its handle in slot 0.
fn msm_op<C: NativeContext, G>(
    ctx: &C,
    element_handles: &Vector<'_, u64>,
    scalar_handles: &Vector<'_, u64>,
) -> VMResult<NativeStatus>
where
    G: Any + Copy + CurveGroup,
    G::ScalarField: Any + Copy,
{
    if element_handles.len() != scalar_handles.len() {
        return Ok(NativeStatus::Abort {
            code: MOVE_ABORT_CODE_INPUT_VECTOR_SIZES_NOT_MATCHING,
            message: None,
        });
    }
    let mut store = ctx.get_extension::<AlgebraStore>()?;
    let mut bases = Vec::with_capacity(element_handles.len() as usize);
    for i in 0..element_handles.len() {
        let handle = element_handles.get_element(i)?;
        bases.push(store.get::<G>(handle)?.into_affine());
    }
    let mut scalars = Vec::with_capacity(scalar_handles.len() as usize);
    for i in 0..scalar_handles.len() {
        let handle = scalar_handles.get_element(i)?;
        scalars.push(store.get::<G::ScalarField>(handle)?);
    }

    let Ok(new_element) = G::msm(&bases, &scalars) else {
        return Ok(NativeStatus::Abort {
            code: E_SCALAR_MUL_MSM_COMPUTATION_FAILED,
            message: Some("MSM computation failed".to_string()),
        });
    };
    let added = store.add(new_element);
    return_handle(ctx, store, added)
}

/// `0x1::crypto_algebra::multi_scalar_mul_internal<G, S>(element_handles: vector<u64>, scalar_handles: vector<u64>): u64`
///
/// Unlike `scalar_mul_internal`, this does not support `Gt`.
///
/// TODO(perf, cleanup): the arkworks call fans out over rayon's global pool,
/// where the legacy VM confines it to a private one. See the module note.
///
/// TODO(metering): charge gas.
pub fn native_multi_scalar_mul<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let group = structure_of(ctx.ty_arg(0)?);
    let scalar_field = structure_of(ctx.ty_arg(1)?);
    // SAFETY: args 0 and 1 are `vector<u64>`.
    let element_handles = unsafe { ctx.arg::<Vector<u64>>(0)? };
    let scalar_handles = unsafe { ctx.arg::<Vector<u64>>(1)? };

    // The structure check precedes the length check, as in the legacy VM: an
    // unsupported group with mismatched lengths is "not implemented".
    match (group, scalar_field) {
        (Some(Structure::BLS12381G1), Some(Structure::BLS12381Fr)) => {
            msm_op::<_, ark_bls12_381::G1Projective>(ctx, &element_handles, &scalar_handles)
        },
        (Some(Structure::BLS12381G2), Some(Structure::BLS12381Fr)) => {
            msm_op::<_, ark_bls12_381::G2Projective>(ctx, &element_handles, &scalar_handles)
        },
        (Some(Structure::BN254G1), Some(Structure::BN254Fr)) => {
            msm_op::<_, ark_bn254::G1Projective>(ctx, &element_handles, &scalar_handles)
        },
        (Some(Structure::BN254G2), Some(Structure::BN254Fr)) => {
            msm_op::<_, ark_bn254::G2Projective>(ctx, &element_handles, &scalar_handles)
        },
        (any_structure!(), any_structure!()) => Ok(not_implemented()),
    }
}
