// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Bilinear pairings.

use super::{
    any_structure, not_implemented, return_handle, structure_of, AlgebraStore, Structure,
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
    let added = store.add(P::pairing(g1, g2).0);
    return_handle(ctx, store, added)
}

/// `0x1::crypto_algebra::pairing_internal<G1, G2, Gt>(g1_handle: u64, g2_handle: u64): u64`
///
/// TODO(perf, cleanup): the arkworks call fans out over rayon's global pool,
/// where the legacy VM confines it to a private one. See the module note.
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
        (any_structure!(), any_structure!(), any_structure!()) => Ok(not_implemented()),
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
    let mut store = ctx.get_extension::<AlgebraStore>()?;
    let mut g1s = Vec::with_capacity(g1_handles.len() as usize);
    for i in 0..g1_handles.len() {
        let handle = g1_handles.get_element(i)?;
        g1s.push(store.get::<P::G1>(handle)?.into_affine());
    }
    let mut g2s = Vec::with_capacity(g2_handles.len() as usize);
    for i in 0..g2_handles.len() {
        let handle = g2_handles.get_element(i)?;
        g2s.push(store.get::<P::G2>(handle)?.into_affine());
    }

    let added = store.add(P::multi_pairing(g1s, g2s).0);
    return_handle(ctx, store, added)
}

/// `0x1::crypto_algebra::multi_pairing_internal<G1, G2, Gt>(g1_handles: vector<u64>, g2_handles: vector<u64>): u64`
///
/// TODO(perf, cleanup): the arkworks call fans out over rayon's global pool,
/// where the legacy VM confines it to a private one. See the module note.
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
        (any_structure!(), any_structure!(), any_structure!()) => Ok(not_implemented()),
    }
}
