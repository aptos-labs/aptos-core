// Copyright © Aptos Foundation

use crate::{
    abort_unless_feature_flag_enabled,
    natives::{
        cryptography::algebra::{
            gas::GasParameters, AlgebraContext, HashToStructureSuite, Structure,
            MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        },
        helpers::{SafeNativeContext, SafeNativeError, SafeNativeResult},
    },
    safely_pop_arg, store_element, structure_from_ty_arg,
};
use aptos_types::on_chain_config::FeatureFlag;
use ark_ec::hashing::HashToCurve;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Value, VectorRef},
};
use smallvec::{smallvec, SmallVec};
use std::{collections::VecDeque, rc::Rc};

fn feature_flag_of_hash_to_structure(
    structure_opt: Option<Structure>,
    suite_opt: Option<HashToStructureSuite>,
) -> Option<FeatureFlag> {
    match (structure_opt, suite_opt) {
        (
            Some(Structure::BLS12381G1Affine),
            Some(HashToStructureSuite::Bls12381g1XmdSha256SswuRo),
        )
        | (
            Some(Structure::BLS12381G2Affine),
            Some(HashToStructureSuite::Bls12381g2XmdSha256SswuRo),
        ) => Some(FeatureFlag::BLS12_381_STRUCTURES),
        _ => None,
    }
}

macro_rules! abort_unless_hash_to_structure_enabled {
    ($context:ident, $structure_opt:expr, $suite_opt:expr) => {
        let flag_opt = feature_flag_of_hash_to_structure($structure_opt, $suite_opt);
        abort_unless_feature_flag_enabled!($context, flag_opt);
    };
}

macro_rules! suite_from_ty_arg {
    ($context:expr, $typ:expr) => {{
        let type_tag = $context.type_to_type_tag($typ).unwrap();
        HashToStructureSuite::try_from(type_tag).ok()
    }};
}

macro_rules! ark_bls12381gx_xmd_sha_256_sswu_ro_internal {
    (
        $gas_params:expr,
        $context:expr,
        $dst:expr,
        $msg:expr,
        $h2s_suite:expr,
        $target_type:ty,
        $config_type:ty
    ) => {{
        $context.charge($gas_params.hash_to($h2s_suite, $dst.len(), $msg.len()))?;
        let mapper = ark_ec::hashing::map_to_curve_hasher::MapToCurveBasedHasher::<
            ark_ec::models::short_weierstrass::Projective<$config_type>,
            ark_ff::fields::field_hashers::DefaultFieldHasher<sha2_0_10_6::Sha256, 128>,
            ark_ec::hashing::curve_maps::wb::WBMap<$config_type>,
        >::new($dst)
        .unwrap();
        let new_element = <$target_type>::from(mapper.hash($msg).unwrap());
        let new_handle = store_element!($context, new_element);
        Ok(smallvec![Value::u64(new_handle as u64)])
    }};
}

pub fn hash_to_internal(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(2, ty_args.len());
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    let suite_opt = suite_from_ty_arg!(context, &ty_args[1]);
    abort_unless_hash_to_structure_enabled!(context, structure_opt, suite_opt);
    let vector_ref = safely_pop_arg!(args, VectorRef);
    let bytes_ref = vector_ref.as_bytes_ref();
    let msg = bytes_ref.as_slice();
    let tag_ref = safely_pop_arg!(args, VectorRef);
    let bytes_ref = tag_ref.as_bytes_ref();
    let dst = bytes_ref.as_slice();
    match (structure_opt, suite_opt) {
        (
            Some(Structure::BLS12381G1Affine),
            Some(HashToStructureSuite::Bls12381g1XmdSha256SswuRo),
        ) => ark_bls12381gx_xmd_sha_256_sswu_ro_internal!(
            gas_params,
            context,
            dst,
            msg,
            HashToStructureSuite::Bls12381g1XmdSha256SswuRo,
            ark_bls12_381::G1Projective,
            ark_bls12_381::g1::Config
        ),
        (
            Some(Structure::BLS12381G2Affine),
            Some(HashToStructureSuite::Bls12381g2XmdSha256SswuRo),
        ) => ark_bls12381gx_xmd_sha_256_sswu_ro_internal!(
            gas_params,
            context,
            dst,
            msg,
            HashToStructureSuite::Bls12381g2XmdSha256SswuRo,
            ark_bls12_381::G2Projective,
            ark_bls12_381::g2::Config
        ),
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}
