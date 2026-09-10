// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Bilinear pairings.

use super::{
    not_implemented, scalar_mul::read_handles, structure_of, AlgebraStore, Structure,
    MOVE_ABORT_CODE_INPUT_VECTOR_SIZES_NOT_MATCHING,
};
use ark_ec::{pairing::Pairing, CurveGroup};
use mono_move_core::{
    native::{NativeContext, NativeStatus, Vector},
    VMResult,
};
use std::any::Any;

/// Stores `e(g1, g2)` and returns its handle in slot 0.
fn pairing_op<C: NativeContext, P: Pairing>(
    ctx: &C,
    g1_handle: u64,
    g2_handle: u64,
) -> VMResult<NativeStatus>
where
    P::G1: Any + Copy,
    P::G2: Any + Copy,
    P::TargetField: Any,
{
    let mut store = ctx.get_extension::<AlgebraStore>()?;
    let g1 = store.get::<P::G1>(g1_handle)?.into_affine();
    let g2 = store.get::<P::G2>(g2_handle)?.into_affine();
    let handle = match store.add(P::pairing(g1, g2).0) {
        Ok(handle) => handle,
        Err(abort) => return Ok(abort),
    };
    drop(store);

    // SAFETY: return slot 0 is `u64`.
    unsafe { ctx.set_return(0, handle)? };
    Ok(NativeStatus::Success)
}

/// `0x1::crypto_algebra::pairing_internal<G1, G2, Gt>(g1_handle: u64, g2_handle: u64): u64`
///
/// TODO(correctness): run the arkworks call under an isolated rayon pool, as the
/// legacy VM's `with_native_rayon` does, or a native on a Block-STM worker can
/// deadlock.
///
/// TODO(metering): charge gas.
pub fn native_pairing<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let g1_structure = structure_of(ctx.ty_arg(0)?);
    let g2_structure = structure_of(ctx.ty_arg(1)?);
    let gt_structure = structure_of(ctx.ty_arg(2)?);
    // SAFETY: args 0 and 1 are `u64` handles.
    let g1_handle = unsafe { ctx.arg::<u64>(0)? };
    let g2_handle = unsafe { ctx.arg::<u64>(1)? };

    match (g1_structure, g2_structure, gt_structure) {
        (Some(Structure::BLS12381G1), Some(Structure::BLS12381G2), Some(Structure::BLS12381Gt)) => {
            pairing_op::<_, ark_bls12_381::Bls12_381>(ctx, g1_handle, g2_handle)
        },
        (Some(Structure::BN254G1), Some(Structure::BN254G2), Some(Structure::BN254Gt)) => {
            pairing_op::<_, ark_bn254::Bn254>(ctx, g1_handle, g2_handle)
        },
        _ => Ok(not_implemented()),
    }
}

/// Stores `prod_i e(g1s[i], g2s[i])` and returns its handle in slot 0.
fn multi_pairing_op<C: NativeContext, P: Pairing>(
    ctx: &C,
    g1_handles: &Vector<'_, u64>,
    g2_handles: &Vector<'_, u64>,
) -> VMResult<NativeStatus>
where
    P::G1: Any + Copy,
    P::G2: Any + Copy,
    P::TargetField: Any,
{
    if g1_handles.len() != g2_handles.len() {
        return Ok(NativeStatus::Abort {
            code: MOVE_ABORT_CODE_INPUT_VECTOR_SIZES_NOT_MATCHING,
            message: None,
        });
    }
    let g1_handles = read_handles(g1_handles)?;
    let g2_handles = read_handles(g2_handles)?;

    let mut store = ctx.get_extension::<AlgebraStore>()?;
    let mut g1s = Vec::with_capacity(g1_handles.len());
    for handle in g1_handles {
        g1s.push(store.get::<P::G1>(handle)?.into_affine());
    }
    let mut g2s = Vec::with_capacity(g2_handles.len());
    for handle in g2_handles {
        g2s.push(store.get::<P::G2>(handle)?.into_affine());
    }

    let handle = match store.add(P::multi_pairing(g1s, g2s).0) {
        Ok(handle) => handle,
        Err(abort) => return Ok(abort),
    };
    drop(store);

    // SAFETY: return slot 0 is `u64`.
    unsafe { ctx.set_return(0, handle)? };
    Ok(NativeStatus::Success)
}

/// `0x1::crypto_algebra::multi_pairing_internal<G1, G2, Gt>(g1_handles: vector<u64>, g2_handles: vector<u64>): u64`
///
/// TODO(correctness): run the arkworks call under an isolated rayon pool, as the
/// legacy VM's `with_native_rayon` does, or a native on a Block-STM worker can
/// deadlock.
///
/// TODO(metering): charge gas.
pub fn native_multi_pairing<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let g1_structure = structure_of(ctx.ty_arg(0)?);
    let g2_structure = structure_of(ctx.ty_arg(1)?);
    let gt_structure = structure_of(ctx.ty_arg(2)?);
    // SAFETY: args 0 and 1 are `vector<u64>`.
    let g1_handles = unsafe { ctx.arg::<Vector<u64>>(0)? };
    let g2_handles = unsafe { ctx.arg::<Vector<u64>>(1)? };

    // The structure check precedes the length check, as in the legacy VM: an
    // unsupported curve with mismatched lengths is "not implemented".
    match (g1_structure, g2_structure, gt_structure) {
        (Some(Structure::BLS12381G1), Some(Structure::BLS12381G2), Some(Structure::BLS12381Gt)) => {
            multi_pairing_op::<_, ark_bls12_381::Bls12_381>(ctx, &g1_handles, &g2_handles)
        },
        (Some(Structure::BN254G1), Some(Structure::BN254G2), Some(Structure::BN254Gt)) => {
            multi_pairing_op::<_, ark_bn254::Bn254>(ctx, &g1_handles, &g2_handles)
        },
        _ => Ok(not_implemented()),
    }
}
