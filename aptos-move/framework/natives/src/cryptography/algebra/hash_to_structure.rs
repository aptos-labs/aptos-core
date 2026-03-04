// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    abort_unless_feature_flag_enabled,
    cryptography::algebra::{
        AlgebraContext, HashToStructureSuite, Structure,
        E_HASH_TO_STRUCTURE_BLS12381G1_HASH_FAILED, E_HASH_TO_STRUCTURE_BLS12381G1_MAPPER_FAILED,
        E_HASH_TO_STRUCTURE_BLS12381G2_HASH_FAILED, E_HASH_TO_STRUCTURE_BLS12381G2_MAPPER_FAILED,
        E_TOO_MUCH_MEMORY_USED, MEMORY_LIMIT_IN_BYTES, MOVE_ABORT_CODE_NOT_IMPLEMENTED,
    },
    store_element, structure_from_ty_arg,
};
use aptos_gas_schedule::gas_params::natives::{aptos_framework::*, move_stdlib::*};
use aptos_native_interface::{
    safely_pop_arg, SafeNativeContext, SafeNativeError, SafeNativeResult,
};
use aptos_types::on_chain_config::FeatureFlag;
use ark_ec::hashing::HashToCurve;
use either::Either;
use move_core_types::gas_algebra::{InternalGas, NumBytes};
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Value, VectorRef},
};
use smallvec::{smallvec, SmallVec};
use std::{collections::VecDeque, rc::Rc};

/// Equivalent to `std::error::internal(99)` in Move.
/// Used when type to type tag conversion fails unexpectedly.
const E_TYPE_TO_TYPE_TAG_CONVERSION_FAILED: u64 = 0x0B_0063;

fn feature_flag_of_hash_to_structure(
    structure_opt: Option<Structure>,
    suite_opt: Option<HashToStructureSuite>,
) -> Option<FeatureFlag> {
    match (structure_opt, suite_opt) {
        (Some(Structure::BLS12381G1), Some(HashToStructureSuite::Bls12381g1XmdSha256SswuRo))
        | (Some(Structure::BLS12381G2), Some(HashToStructureSuite::Bls12381g2XmdSha256SswuRo)) => {
            Some(FeatureFlag::BLS12_381_STRUCTURES)
        },
        _ => None,
    }
}

macro_rules! abort_unless_hash_to_structure_enabled {
    ($context:ident, $structure_opt:expr, $suite_opt:expr) => {
        let flag_opt = feature_flag_of_hash_to_structure($structure_opt, $suite_opt);
        abort_unless_feature_flag_enabled!($context, flag_opt);
    };
}

fn suite_from_ty_arg(
    context: &SafeNativeContext,
    ty: &Type,
) -> SafeNativeResult<Option<HashToStructureSuite>> {
    let type_tag = context.type_to_type_tag(ty).map_err(|_| {
        SafeNativeError::abort_with_message(
            E_TYPE_TO_TYPE_TAG_CONVERSION_FAILED,
            "Conversion from type to type tag failed (too complex)",
        )
    })?;
    Ok(HashToStructureSuite::try_from(type_tag).ok())
}

macro_rules! hash_to_bls12381gx_cost {
    (
        $dst_len: expr,
        $msg_len: expr,
        $dst_shortening_base: expr,
        $dst_shortening_per_byte: expr,
        $mapping_base: expr,
        $mapping_per_byte: expr
        $(,)?
    ) => {{
        let dst_len: usize = $dst_len;

        // DST shortening as defined in https://www.ietf.org/archive/id/draft-irtf-cfrg-hash-to-curve-16.html#name-using-dsts-longer-than-255-.
        let dst_shortening_cost = if dst_len <= 255 {
            Either::Left(InternalGas::zero())
        } else {
            Either::Right($dst_shortening_base + $dst_shortening_per_byte * NumBytes::from((17 + dst_len) as u64))
        };

        // Mapping cost. The gas formula is simplified by assuming the DST length is fixed at 256.
        let mapping_cost =
            $mapping_base + $mapping_per_byte * NumBytes::from($msg_len as u64);

        mapping_cost + dst_shortening_cost
    }};
}

pub fn hash_to_internal(
    context: &mut SafeNativeContext,
    ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(2, ty_args.len());
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    let suite_opt = suite_from_ty_arg(context, &ty_args[1])?;
    abort_unless_hash_to_structure_enabled!(context, structure_opt, suite_opt);
    let vector_ref = safely_pop_arg!(args, VectorRef);
    let bytes_ref = vector_ref.as_bytes_ref();
    let msg = bytes_ref.as_slice();
    let tag_ref = safely_pop_arg!(args, VectorRef);
    let bytes_ref = tag_ref.as_bytes_ref();
    let dst = bytes_ref.as_slice();
    match (structure_opt, suite_opt) {
        (Some(Structure::BLS12381G1), Some(HashToStructureSuite::Bls12381g1XmdSha256SswuRo)) => {
            context.charge(hash_to_bls12381gx_cost!(
                dst.len(),
                msg.len(),
                HASH_SHA2_256_BASE,
                HASH_SHA2_256_PER_BYTE,
                ALGEBRA_ARK_H2C_BLS12381G1_XMD_SHA256_SSWU_BASE,
                ALGEBRA_ARK_H2C_BLS12381G1_XMD_SHA256_SSWU_PER_MSG_BYTE,
            ))?;
            let mapper = ark_ec::hashing::map_to_curve_hasher::MapToCurveBasedHasher::<
                ark_ec::models::short_weierstrass::Projective<ark_bls12_381::g1::Config>,
                ark_ff::fields::field_hashers::DefaultFieldHasher<sha2_0_10_6::Sha256, 128>,
                ark_ec::hashing::curve_maps::wb::WBMap<ark_bls12_381::g1::Config>,
            >::new(dst)
            .map_err(|_e| {
                SafeNativeError::abort_with_message(
                    E_HASH_TO_STRUCTURE_BLS12381G1_MAPPER_FAILED,
                    "BLS12381 G1 hash-to-curve mapper creation failed",
                )
            })?;
            let new_element =
                <ark_bls12_381::G1Projective>::from(mapper.hash(msg).map_err(|_e| {
                    SafeNativeError::abort_with_message(
                        E_HASH_TO_STRUCTURE_BLS12381G1_HASH_FAILED,
                        "BLS12381 G1 hash-to-curve hash failed",
                    )
                })?);
            let new_handle = store_element!(context, new_element)?;
            Ok(smallvec![Value::u64(new_handle as u64)])
        },
        (Some(Structure::BLS12381G2), Some(HashToStructureSuite::Bls12381g2XmdSha256SswuRo)) => {
            context.charge(hash_to_bls12381gx_cost!(
                dst.len(),
                msg.len(),
                HASH_SHA2_256_BASE,
                HASH_SHA2_256_PER_BYTE,
                ALGEBRA_ARK_H2C_BLS12381G2_XMD_SHA256_SSWU_BASE,
                ALGEBRA_ARK_H2C_BLS12381G2_XMD_SHA256_SSWU_PER_MSG_BYTE,
            ))?;
            let mapper = ark_ec::hashing::map_to_curve_hasher::MapToCurveBasedHasher::<
                ark_ec::models::short_weierstrass::Projective<ark_bls12_381::g2::Config>,
                ark_ff::fields::field_hashers::DefaultFieldHasher<sha2_0_10_6::Sha256, 128>,
                ark_ec::hashing::curve_maps::wb::WBMap<ark_bls12_381::g2::Config>,
            >::new(dst)
            .map_err(|_e| {
                SafeNativeError::abort_with_message(
                    E_HASH_TO_STRUCTURE_BLS12381G2_MAPPER_FAILED,
                    "BLS12381 G2 hash-to-curve mapper creation failed",
                )
            })?;
            let new_element =
                <ark_bls12_381::G2Projective>::from(mapper.hash(msg).map_err(|_e| {
                    SafeNativeError::abort_with_message(
                        E_HASH_TO_STRUCTURE_BLS12381G2_HASH_FAILED,
                        "BLS12381 G2 hash-to-curve hash failed",
                    )
                })?);
            let new_handle = store_element!(context, new_element)?;
            Ok(smallvec![Value::u64(new_handle as u64)])
        },
        _ => Err(SafeNativeError::abort(MOVE_ABORT_CODE_NOT_IMPLEMENTED)),
    }
}
