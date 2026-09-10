// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Hash-to-curve.

use super::{
    not_implemented, structure_of, suite_of, AlgebraStore, HashToStructureSuite, Structure,
    E_HASH_TO_STRUCTURE_BLS12381G1_HASH_FAILED, E_HASH_TO_STRUCTURE_BLS12381G1_MAPPER_FAILED,
    E_HASH_TO_STRUCTURE_BLS12381G2_HASH_FAILED, E_HASH_TO_STRUCTURE_BLS12381G2_MAPPER_FAILED,
};
use ark_ec::{
    hashing::{curve_maps::wb::WBMap, map_to_curve_hasher::MapToCurveBasedHasher, HashToCurve},
    short_weierstrass::Projective,
};
use ark_ff::fields::field_hashers::DefaultFieldHasher;
use mono_move_core::{
    native::{NativeContext, NativeStatus, Ref, Vector},
    VMResult,
};

/// The hasher both BLS12-381 suites use: expand-message-XMD over SHA-256 with a
/// 128-bit security parameter, then the simplified SWU map.
///
/// `sha2_0_10_6` is deliberate. This crate also links sha2 0.9, whose `Sha256`
/// would compile here and silently produce a different digest.
type Bls12381Hasher<Config> = MapToCurveBasedHasher<
    Projective<Config>,
    DefaultFieldHasher<sha2_0_10_6::Sha256, 128>,
    WBMap<Config>,
>;

/// `0x1::crypto_algebra::hash_to_internal<S, H>(dst: &vector<u8>, bytes: &vector<u8>): u64`
///
/// TODO(metering): charge gas.
pub fn native_hash_to<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let structure = structure_of(ctx.ty_arg(0)?);
    let suite = suite_of(ctx.ty_arg(1)?);
    // SAFETY: args 0 and 1 are `&vector<u8>`.
    let dst_ref = unsafe { ctx.arg::<Ref<Vector<u8>>>(0)? };
    let msg_ref = unsafe { ctx.arg::<Ref<Vector<u8>>>(1)? };
    let dst_vec = dst_ref.borrow();
    let msg_vec = msg_ref.borrow();
    // SAFETY: both slices are consumed by the hash below, which does not
    // allocate on the VM heap, so GC cannot relocate them while they are held.
    let dst = unsafe { dst_vec.as_bytes() };
    let msg = unsafe { msg_vec.as_bytes() };

    let mut store = ctx.get_extension::<AlgebraStore>()?;
    let result = match (structure, suite) {
        (Some(Structure::BLS12381G1), Some(HashToStructureSuite::Bls12381g1XmdSha256SswuRo)) => {
            let Ok(mapper) = Bls12381Hasher::<ark_bls12_381::g1::Config>::new(dst) else {
                return Ok(NativeStatus::Abort {
                    code: E_HASH_TO_STRUCTURE_BLS12381G1_MAPPER_FAILED,
                    message: Some("BLS12381 G1 hash-to-curve mapper creation failed".to_string()),
                });
            };
            let Ok(point) = mapper.hash(msg) else {
                return Ok(NativeStatus::Abort {
                    code: E_HASH_TO_STRUCTURE_BLS12381G1_HASH_FAILED,
                    message: Some("BLS12381 G1 hash-to-curve hash failed".to_string()),
                });
            };
            store.add(ark_bls12_381::G1Projective::from(point))
        },
        (Some(Structure::BLS12381G2), Some(HashToStructureSuite::Bls12381g2XmdSha256SswuRo)) => {
            let Ok(mapper) = Bls12381Hasher::<ark_bls12_381::g2::Config>::new(dst) else {
                return Ok(NativeStatus::Abort {
                    code: E_HASH_TO_STRUCTURE_BLS12381G2_MAPPER_FAILED,
                    message: Some("BLS12381 G2 hash-to-curve mapper creation failed".to_string()),
                });
            };
            let Ok(point) = mapper.hash(msg) else {
                return Ok(NativeStatus::Abort {
                    code: E_HASH_TO_STRUCTURE_BLS12381G2_HASH_FAILED,
                    message: Some("BLS12381 G2 hash-to-curve hash failed".to_string()),
                });
            };
            store.add(ark_bls12_381::G2Projective::from(point))
        },
        _ => return Ok(not_implemented()),
    };
    let handle = match result {
        Ok(handle) => handle,
        Err(abort) => return Ok(abort),
    };
    drop(store);

    // SAFETY: return slot 0 is `u64`.
    unsafe { ctx.set_return(0, handle)? };
    Ok(NativeStatus::Success)
}
