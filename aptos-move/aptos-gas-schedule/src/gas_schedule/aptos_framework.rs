// Copyright (c) 2024 Supra.
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module defines the gas parameters for Aptos Framework & Stdlib.

use crate::{
    gas_feature_versions::{RELEASE_V1_14, RELEASE_V1_8, RELEASE_V1_9_SKIPPED},
    gas_schedule::NativeGasParameters,
    ver::gas_feature_versions::{RELEASE_V1_12, RELEASE_V1_13, RELEASE_V1_16_SUPRA_V1_6_0, RELEASE_V1_16_SUPRA_V1_7_14},
};
use aptos_gas_algebra::{
    InternalGas, InternalGasPerAbstractValueUnit, InternalGasPerArg, InternalGasPerByte,
};

crate::gas_schedule::macros::define_gas_parameters!(
    AptosFrameworkGasParameters,
    "aptos_framework",
    NativeGasParameters => .aptos_framework,
    [
        [account_create_address_base: InternalGas, "account.create_address.base", 1102],
        [account_create_signer_base: InternalGas, "account.create_signer.base", 1102],

        // BN254 algebra gas parameters begin.
        // Generated at time 1701559125.5498126 by `scripts/algebra-gas/update_bn254_algebra_gas_params.py` with gas_per_ns=209.10511688369482.
        [algebra_ark_bn254_fq12_add: InternalGas, { 12.. => "algebra.ark_bn254_fq12_add" }, 809],
        [algebra_ark_bn254_fq12_clone: InternalGas, { 12.. => "algebra.ark_bn254_fq12_clone" }, 807],
        [algebra_ark_bn254_fq12_deser: InternalGas, { 12.. => "algebra.ark_bn254_fq12_deser" }, 23721],
        [algebra_ark_bn254_fq12_div: InternalGas, { 12.. => "algebra.ark_bn254_fq12_div" }, 517140],
        [algebra_ark_bn254_fq12_eq: InternalGas, { 12.. => "algebra.ark_bn254_fq12_eq" }, 2231],
        [algebra_ark_bn254_fq12_from_u64: InternalGas, { 12.. => "algebra.ark_bn254_fq12_from_u64" }, 2658],
        [algebra_ark_bn254_fq12_inv: InternalGas, { 12.. => "algebra.ark_bn254_fq12_inv" }, 398555],
        [algebra_ark_bn254_fq12_mul: InternalGas, { 12.. => "algebra.ark_bn254_fq12_mul" }, 118351],
        [algebra_ark_bn254_fq12_neg: InternalGas, { 12.. => "algebra.ark_bn254_fq12_neg" }, 2446],
        [algebra_ark_bn254_fq12_one: InternalGas, { 12.. => "algebra.ark_bn254_fq12_one" }, 38],
        [algebra_ark_bn254_fq12_pow_u256: InternalGas, { 12.. => "algebra.ark_bn254_fq12_pow_u256" }, 35449826],
        [algebra_ark_bn254_fq12_serialize: InternalGas, { 12.. => "algebra.ark_bn254_fq12_serialize" }, 21566],
        [algebra_ark_bn254_fq12_square: InternalGas, { 12.. => "algebra.ark_bn254_fq12_square" }, 86193],
        [algebra_ark_bn254_fq12_sub: InternalGas, { 12.. => "algebra.ark_bn254_fq12_sub" }, 5605],
        [algebra_ark_bn254_fq12_zero: InternalGas, { 12.. => "algebra.ark_bn254_fq12_zero" }, 38],
        [algebra_ark_bn254_fq_add: InternalGas, { 12.. => "algebra.ark_bn254_fq_add" }, 803],
        [algebra_ark_bn254_fq_clone: InternalGas, { 12.. => "algebra.ark_bn254_fq_clone" }, 792],
        [algebra_ark_bn254_fq_deser: InternalGas, { 12.. => "algebra.ark_bn254_fq_deser" }, 3232],
        [algebra_ark_bn254_fq_div: InternalGas, { 12.. => "algebra.ark_bn254_fq_div" }, 209631],
        [algebra_ark_bn254_fq_eq: InternalGas, { 12.. => "algebra.ark_bn254_fq_eq" }, 803],
        [algebra_ark_bn254_fq_from_u64: InternalGas, { 12.. => "algebra.ark_bn254_fq_from_u64" }, 2598],
        [algebra_ark_bn254_fq_inv: InternalGas, { 12.. => "algebra.ark_bn254_fq_inv" }, 208902],
        [algebra_ark_bn254_fq_mul: InternalGas, { 12.. => "algebra.ark_bn254_fq_mul" }, 1847],
        [algebra_ark_bn254_fq_neg: InternalGas, { 12.. => "algebra.ark_bn254_fq_neg" }, 792],
        [algebra_ark_bn254_fq_one: InternalGas, { 12.. => "algebra.ark_bn254_fq_one" }, 38],
        [algebra_ark_bn254_fq_pow_u256: InternalGas, { 12.. => "algebra.ark_bn254_fq_pow_u256" }, 382570],
        [algebra_ark_bn254_fq_serialize: InternalGas, { 12.. => "algebra.ark_bn254_fq_serialize" }, 4767],
        [algebra_ark_bn254_fq_square: InternalGas, { 12.. => "algebra.ark_bn254_fq_square" }, 792],
        [algebra_ark_bn254_fq_sub: InternalGas, { 12.. => "algebra.ark_bn254_fq_sub" }, 1130],
        [algebra_ark_bn254_fq_zero: InternalGas, { 12.. => "algebra.ark_bn254_fq_zero" }, 38],
        [algebra_ark_bn254_fr_add: InternalGas, { 12.. => "algebra.ark_bn254_fr_add" }, 804],
        [algebra_ark_bn254_fr_deser: InternalGas, { 12.. => "algebra.ark_bn254_fr_deser" }, 3073],
        [algebra_ark_bn254_fr_div: InternalGas, { 12.. => "algebra.ark_bn254_fr_div" }, 223857],
        [algebra_ark_bn254_fr_eq: InternalGas, { 12.. => "algebra.ark_bn254_fr_eq" }, 807],
        [algebra_ark_bn254_fr_from_u64: InternalGas, { 12.. => "algebra.ark_bn254_fr_from_u64" }, 2478],
        [algebra_ark_bn254_fr_inv: InternalGas, { 12.. => "algebra.ark_bn254_fr_inv" }, 222216],
        [algebra_ark_bn254_fr_mul: InternalGas, { 12.. => "algebra.ark_bn254_fr_mul" }, 1813],
        [algebra_ark_bn254_fr_neg: InternalGas, { 12.. => "algebra.ark_bn254_fr_neg" }, 792],
        [algebra_ark_bn254_fr_one: InternalGas, { 12.. => "algebra.ark_bn254_fr_one" }, 0],
        [algebra_ark_bn254_fr_serialize: InternalGas, { 12.. => "algebra.ark_bn254_fr_serialize" }, 4732],
        [algebra_ark_bn254_fr_square: InternalGas, { 12.. => "algebra.ark_bn254_fr_square" }, 792],
        [algebra_ark_bn254_fr_sub: InternalGas, { 12.. => "algebra.ark_bn254_fr_sub" }, 1906],
        [algebra_ark_bn254_fr_zero: InternalGas, { 12.. => "algebra.ark_bn254_fr_zero" }, 38],
        [algebra_ark_bn254_g1_affine_deser_comp: InternalGas, { 12.. => "algebra.ark_bn254_g1_affine_deser_comp" }, 4318809],
        [algebra_ark_bn254_g1_affine_deser_uncomp: InternalGas, { 12.. => "algebra.ark_bn254_g1_affine_deser_uncomp" }, 3956976],
        [algebra_ark_bn254_g1_affine_serialize_comp: InternalGas, { 12.. => "algebra.ark_bn254_g1_affine_serialize_comp" }, 8257],
        [algebra_ark_bn254_g1_affine_serialize_uncomp: InternalGas, { 12.. => "algebra.ark_bn254_g1_affine_serialize_uncomp" }, 10811],
        [algebra_ark_bn254_g1_proj_add: InternalGas, { 12.. => "algebra.ark_bn254_g1_proj_add" }, 19574],
        [algebra_ark_bn254_g1_proj_double: InternalGas, { 12.. => "algebra.ark_bn254_g1_proj_double" }, 11704],
        [algebra_ark_bn254_g1_proj_eq: InternalGas, { 12.. => "algebra.ark_bn254_g1_proj_eq" }, 9745],
        [algebra_ark_bn254_g1_proj_generator: InternalGas, { 12.. => "algebra.ark_bn254_g1_proj_generator" }, 38],
        [algebra_ark_bn254_g1_proj_infinity: InternalGas, { 12.. => "algebra.ark_bn254_g1_proj_infinity" }, 38],
        [algebra_ark_bn254_g1_proj_neg: InternalGas, { 12.. => "algebra.ark_bn254_g1_proj_neg" }, 38],
        [algebra_ark_bn254_g1_proj_scalar_mul: InternalGas, { 12.. => "algebra.ark_bn254_g1_proj_scalar_mul" }, 4862683],
        [algebra_ark_bn254_g1_proj_sub: InternalGas, { 12.. => "algebra.ark_bn254_g1_proj_sub" }, 19648],
        [algebra_ark_bn254_g1_proj_to_affine: InternalGas, { 12.. => "algebra.ark_bn254_g1_proj_to_affine" }, 1165],
        [algebra_ark_bn254_g2_affine_deser_comp: InternalGas, { 12.. => "algebra.ark_bn254_g2_affine_deser_comp" }, 12445138],
        [algebra_ark_bn254_g2_affine_deser_uncomp: InternalGas, { 12.. => "algebra.ark_bn254_g2_affine_deser_uncomp" }, 11152541],
        [algebra_ark_bn254_g2_affine_serialize_comp: InternalGas, { 12.. => "algebra.ark_bn254_g2_affine_serialize_comp" }, 12721],
        [algebra_ark_bn254_g2_affine_serialize_uncomp: InternalGas, { 12.. => "algebra.ark_bn254_g2_affine_serialize_uncomp" }, 18105],
        [algebra_ark_bn254_g2_proj_add: InternalGas, { 12.. => "algebra.ark_bn254_g2_proj_add" }, 58491],
        [algebra_ark_bn254_g2_proj_double: InternalGas, { 12.. => "algebra.ark_bn254_g2_proj_double" }, 29201],
        [algebra_ark_bn254_g2_proj_eq: InternalGas, { 12.. => "algebra.ark_bn254_g2_proj_eq" }, 25981],
        [algebra_ark_bn254_g2_proj_generator: InternalGas, { 12.. => "algebra.ark_bn254_g2_proj_generator" }, 38],
        [algebra_ark_bn254_g2_proj_infinity: InternalGas, { 12.. => "algebra.ark_bn254_g2_proj_infinity" }, 38],
        [algebra_ark_bn254_g2_proj_neg: InternalGas, { 12.. => "algebra.ark_bn254_g2_proj_neg" }, 38],
        [algebra_ark_bn254_g2_proj_scalar_mul: InternalGas, { 12.. => "algebra.ark_bn254_g2_proj_scalar_mul" }, 14041548],
        [algebra_ark_bn254_g2_proj_sub: InternalGas, { 12.. => "algebra.ark_bn254_g2_proj_sub" }, 59133],
        [algebra_ark_bn254_g2_proj_to_affine: InternalGas, { 12.. => "algebra.ark_bn254_g2_proj_to_affine" }, 230100],
        [algebra_ark_bn254_multi_pairing_base: InternalGas, { 12.. => "algebra.ark_bn254_multi_pairing_base" }, 23488646],
        [algebra_ark_bn254_multi_pairing_per_pair: InternalGasPerArg, { 12.. => "algebra.ark_bn254_multi_pairing_per_pair" }, 12429399],
        [algebra_ark_bn254_pairing: InternalGas, { 12.. => "algebra.ark_bn254_pairing" }, 38543565],
        // BN254 algebra gas parameters end.

        // BLS12-381 algebra gas parameters begin.
        // Generated at time 1680606720.0709136 by `scripts/algebra-gas/update_algebra_gas_params.py` with gas_per_ns=204.6.
        [algebra_ark_bls12_381_fq12_add: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_add" }, 6686],
        [algebra_ark_bls12_381_fq12_clone: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_clone" }, 775],
        [algebra_ark_bls12_381_fq12_deser: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_deser" }, 41097],
        [algebra_ark_bls12_381_fq12_div: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_div" }, 921988],
        [algebra_ark_bls12_381_fq12_eq: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_eq" }, 2668],
        [algebra_ark_bls12_381_fq12_from_u64: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_from_u64" }, 3312],
        [algebra_ark_bls12_381_fq12_inv: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_inv" }, 737122],
        [algebra_ark_bls12_381_fq12_mul: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_mul" }, 183380],
        [algebra_ark_bls12_381_fq12_neg: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_neg" }, 4341],
        [algebra_ark_bls12_381_fq12_one: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_one" }, 40],
        [algebra_ark_bls12_381_fq12_pow_u256: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_pow_u256" }, 53905624],
        [algebra_ark_bls12_381_fq12_serialize: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_serialize" }, 29694],
        [algebra_ark_bls12_381_fq12_square: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_square" }, 129193],
        [algebra_ark_bls12_381_fq12_sub: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_sub" }, 6462],
        [algebra_ark_bls12_381_fq12_zero: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_zero" }, 775],
        [algebra_ark_bls12_381_fr_add: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_add" }, 775],
        [algebra_ark_bls12_381_fr_deser: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_deser" }, 2764],
        [algebra_ark_bls12_381_fr_div: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_div" }, 218501],
        [algebra_ark_bls12_381_fr_eq: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_eq" }, 779],
        [algebra_ark_bls12_381_fr_from_u64: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_from_u64" }, 1815],
        [algebra_ark_bls12_381_fr_inv: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_inv" }, 215450],
        [algebra_ark_bls12_381_fr_mul: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_mul" }, 1845],
        [algebra_ark_bls12_381_fr_neg: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_neg" }, 782],
        [algebra_ark_bls12_381_fr_one: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_one" }, 775],
        [algebra_ark_bls12_381_fr_serialize: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_serialize" }, 4054],
        [algebra_ark_bls12_381_fr_square: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_square" }, 1746],
        [algebra_ark_bls12_381_fr_sub: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_sub" }, 1066],
        [algebra_ark_bls12_381_fr_zero: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_zero" }, 775],
        [algebra_ark_bls12_381_g1_affine_deser_comp: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_affine_deser_comp" }, 3784805],
        [algebra_ark_bls12_381_g1_affine_deser_uncomp: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_affine_deser_uncomp" }, 2649065],
        [algebra_ark_bls12_381_g1_affine_serialize_comp: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_affine_serialize_comp" }, 7403],
        [algebra_ark_bls12_381_g1_affine_serialize_uncomp: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_affine_serialize_uncomp" }, 8943],
        [algebra_ark_bls12_381_g1_proj_add: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_proj_add" }, 39722],
        [algebra_ark_bls12_381_g1_proj_double: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_proj_double" }, 19350],
        [algebra_ark_bls12_381_g1_proj_eq: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_proj_eq" }, 18508],
        [algebra_ark_bls12_381_g1_proj_generator: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_proj_generator" }, 40],
        [algebra_ark_bls12_381_g1_proj_infinity: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_proj_infinity" }, 40],
        [algebra_ark_bls12_381_g1_proj_neg: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_proj_neg" }, 40],
        [algebra_ark_bls12_381_g1_proj_scalar_mul: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_proj_scalar_mul" }, 9276463],
        [algebra_ark_bls12_381_g1_proj_sub: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_proj_sub" }, 40976],
        [algebra_ark_bls12_381_g1_proj_to_affine: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_proj_to_affine" }, 444924],
        [algebra_ark_bls12_381_g2_affine_deser_comp: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_affine_deser_comp" }, 7572809],
        [algebra_ark_bls12_381_g2_affine_deser_uncomp: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_affine_deser_uncomp" }, 3742090],
        [algebra_ark_bls12_381_g2_affine_serialize_comp: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_affine_serialize_comp" }, 12417],
        [algebra_ark_bls12_381_g2_affine_serialize_uncomp: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_affine_serialize_uncomp" }, 15501],
        [algebra_ark_bls12_381_g2_proj_add: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_proj_add" }, 119106],
        [algebra_ark_bls12_381_g2_proj_double: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_proj_double" }, 54548],
        [algebra_ark_bls12_381_g2_proj_eq: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_proj_eq" }, 55709],
        [algebra_ark_bls12_381_g2_proj_generator: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_proj_generator" }, 40],
        [algebra_ark_bls12_381_g2_proj_infinity: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_proj_infinity" }, 40],
        [algebra_ark_bls12_381_g2_proj_neg: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_proj_neg" }, 40],
        [algebra_ark_bls12_381_g2_proj_scalar_mul: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_proj_scalar_mul" }, 27667443],
        [algebra_ark_bls12_381_g2_proj_sub: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_proj_sub" }, 120826],
        [algebra_ark_bls12_381_g2_proj_to_affine: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_proj_to_affine" }, 473678],
        [algebra_ark_bls12_381_multi_pairing_base: InternalGas, { 8.. => "algebra.ark_bls12_381_multi_pairing_base" }, 33079033],
        [algebra_ark_bls12_381_multi_pairing_per_pair: InternalGasPerArg, { 8.. => "algebra.ark_bls12_381_multi_pairing_per_pair" }, 16919311],
        [algebra_ark_bls12_381_pairing: InternalGas, { 8.. => "algebra.ark_bls12_381_pairing" }, 54523240],
        [algebra_ark_h2c_bls12381g1_xmd_sha256_sswu_base: InternalGas, { 8.. => "algebra.ark_h2c_bls12381g1_xmd_sha256_sswu_base" }, 11954142],
        [algebra_ark_h2c_bls12381g1_xmd_sha256_sswu_per_msg_byte: InternalGasPerByte, { 8.. => "algebra.ark_h2c_bls12381g1_xmd_sha256_sswu_per_msg_byte" }, 176],
        [algebra_ark_h2c_bls12381g2_xmd_sha256_sswu_base: InternalGas, { 8.. => "algebra.ark_h2c_bls12381g2_xmd_sha256_sswu_base" }, 24897555],
        [algebra_ark_h2c_bls12381g2_xmd_sha256_sswu_per_msg_byte: InternalGasPerByte, { 8.. => "algebra.ark_h2c_bls12381g2_xmd_sha256_sswu_per_msg_byte" }, 176],
        // BLS12-381 algebra gas parameters end.

        [bls12381_base: InternalGas, "bls12381.base", 551],

        [bls12381_per_pubkey_deserialize: InternalGasPerArg, "bls12381.per_pubkey_deserialize", 400684],
        [bls12381_per_pubkey_aggregate: InternalGasPerArg, "bls12381.per_pubkey_aggregate", 15439],
        [bls12381_per_pubkey_subgroup_check: InternalGasPerArg, "bls12381.per_pubkey_subgroup_check", 1360120],

        [bls12381_per_sig_deserialize: InternalGasPerArg, "bls12381.per_sig_deserialize", 816072],
        [bls12381_per_sig_aggregate: InternalGasPerArg, "bls12381.per_sig_aggregate", 42825],
        [bls12381_per_sig_subgroup_check: InternalGasPerArg, "bls12381.per_sig_subgroup_check", 1692798],

        [bls12381_per_sig_verify: InternalGasPerArg, "bls12381.per_sig_verify", 31190860],
        [bls12381_per_pop_verify: InternalGasPerArg, "bls12381.per_pop_verify", 37862800],

        [bls12381_per_pairing: InternalGasPerArg, "bls12381.per_pairing", 14751788],

        [bls12381_per_msg_hashing: InternalGasPerArg, "bls12381.per_msg_hashing", 5661040],
        [bls12381_per_byte_hashing: InternalGasPerByte, "bls12381.per_byte_hashing", 183],

        [ed25519_base: InternalGas, "signature.base", 551],
        [ed25519_per_pubkey_deserialize: InternalGasPerArg, "signature.per_pubkey_deserialize", 139688],
        [ed25519_per_pubkey_small_order_check: InternalGasPerArg, "signature.per_pubkey_small_order_check", 23342],
        [ed25519_per_sig_deserialize: InternalGasPerArg, "signature.per_sig_deserialize", 1378],
        [ed25519_per_sig_strict_verify: InternalGasPerArg, "signature.per_sig_strict_verify", 981492],
        [ed25519_per_msg_hashing_base: InternalGasPerArg, "signature.per_msg_hashing_base", 11910],
        [ed25519_per_msg_byte_hashing: InternalGasPerByte, "signature.per_msg_byte_hashing", 220],

        [secp256k1_base: InternalGas, "secp256k1.base", 551],
        [secp256k1_ecdsa_recover: InternalGasPerArg, "secp256k1.ecdsa_recover", 5918360],

        [ristretto255_basepoint_mul: InternalGasPerArg, "ristretto255.basepoint_mul", 470528],
        [ristretto255_basepoint_double_mul: InternalGasPerArg, "ristretto255.basepoint_double_mul", 1617440],

        [ristretto255_point_add: InternalGasPerArg, "ristretto255.point_add", 7848],
        [ristretto255_point_clone: InternalGasPerArg, { 11.. => "ristretto255.point_clone" }, 551],
        [ristretto255_point_compress: InternalGasPerArg, "ristretto255.point_compress", 147040],
        [ristretto255_point_decompress: InternalGasPerArg, "ristretto255.point_decompress", 148878],
        [ristretto255_point_equals: InternalGasPerArg, "ristretto255.point_equals", 8454],
        [ristretto255_point_from_64_uniform_bytes: InternalGasPerArg, "ristretto255.point_from_64_uniform_bytes", 299594],
        [ristretto255_point_identity: InternalGasPerArg, "ristretto255.point_identity", 551],
        [ristretto255_point_mul: InternalGasPerArg, "ristretto255.point_mul", 1731396],
        [ristretto255_point_double_mul: InternalGasPerArg, { 11.. => "ristretto255.point_double_mul" }, 1869907],
        [ristretto255_point_neg: InternalGasPerArg, "ristretto255.point_neg", 1323],
        [ristretto255_point_sub: InternalGasPerArg, "ristretto255.point_sub", 7829],
        [ristretto255_point_parse_arg: InternalGasPerArg, "ristretto255.point_parse_arg", 551],


        // TODO(Alin): These SHA512 gas costs could be unified with the costs in our future SHA512 module
        // (assuming same implementation complexity, which might not be the case
        [ristretto255_sha512_per_byte: InternalGasPerByte, "ristretto255.scalar_sha512_per_byte", 220],
        [ristretto255_sha512_per_hash: InternalGasPerArg, "ristretto255.scalar_sha512_per_hash", 11910],

        [ristretto255_scalar_add: InternalGasPerArg, "ristretto255.scalar_add", 2830],
        [ristretto255_scalar_reduced_from_32_bytes: InternalGasPerArg, "ristretto255.scalar_reduced_from_32_bytes", 2609],
        [ristretto255_scalar_uniform_from_64_bytes: InternalGasPerArg, "ristretto255.scalar_uniform_from_64_bytes", 4576],
        [ristretto255_scalar_from_u128: InternalGasPerArg, "ristretto255.scalar_from_u128", 643],
        [ristretto255_scalar_from_u64: InternalGasPerArg, "ristretto255.scalar_from_u64", 643],
        [ristretto255_scalar_invert: InternalGasPerArg, "ristretto255.scalar_invert", 404360],
        [ristretto255_scalar_is_canonical: InternalGasPerArg, "ristretto255.scalar_is_canonical", 4227],
        [ristretto255_scalar_mul: InternalGasPerArg, "ristretto255.scalar_mul", 3914],
        [ristretto255_scalar_neg: InternalGasPerArg, "ristretto255.scalar_neg", 2665],
        [ristretto255_scalar_sub: InternalGasPerArg, "ristretto255.scalar_sub", 3896],
        [ristretto255_scalar_parse_arg: InternalGasPerArg, "ristretto255.scalar_parse_arg", 551],

        [hash_sip_hash_base: InternalGas, "hash.sip_hash.base", 3676],
        [hash_sip_hash_per_byte: InternalGasPerByte, "hash.sip_hash.per_byte", 73],

        [hash_keccak256_base: InternalGas, { 1.. => "hash.keccak256.base" }, 14704],
        [hash_keccak256_per_byte: InternalGasPerByte, { 1.. => "hash.keccak256.per_byte" }, 165],

        [eth_trie_proof_base: InternalGas, { RELEASE_V1_16_SUPRA_V1_6_0.. => "eth.trie.proof.base" }, 15000],
        [eth_trie_proof_hash_base: InternalGasPerArg, { RELEASE_V1_16_SUPRA_V1_6_0.. => "eth.trie.proof.hash.base" }, 14704],
        [eth_trie_proof_hash_per_byte: InternalGasPerByte, { RELEASE_V1_16_SUPRA_V1_6_0.. => "eth.trie.proof.hash.per_byte" }, 165],
        [eth_trie_proof_decode_base: InternalGasPerArg, { RELEASE_V1_16_SUPRA_V1_6_0.. => "eth.trie.proof.decode.base" }, 1102],
        [eth_trie_proof_decode_per_byte: InternalGasPerByte, { RELEASE_V1_16_SUPRA_V1_6_0.. => "eth.trie.proof.decode.per_byte"}, 18],

        [rlp_encode_decode_base: InternalGas, { RELEASE_V1_16_SUPRA_V1_7_14.. => "rlp.encode.decode.base" }, 1102],
        [rlp_encode_decode_per_byte: InternalGasPerByte, { RELEASE_V1_16_SUPRA_V1_7_14.. => "rlp.encode.decode.per_byte"}, 18],

        // Bulletproofs gas parameters begin.
        // Generated at time 1683148919.0628748 by `scripts/algebra-gas/update_bulletproofs_gas_params.py` with gas_per_ns=10.0.
        [bulletproofs_base: InternalGas, { 11.. => "bulletproofs.base" }, 11794651],
        [bulletproofs_per_bit_rangeproof_verify: InternalGasPerArg, { 11.. => "bulletproofs.per_bit_rangeproof_verify" }, 1004253],
        [bulletproofs_per_byte_rangeproof_deserialize: InternalGasPerByte, { 11.. => "bulletproofs.per_byte_rangeproof_deserialize" }, 121],
        // Bulletproofs gas parameters end.

        [type_info_type_of_base: InternalGas, "type_info.type_of.base", 1102],
        // TODO(Gas): the on-chain name is wrong...
        [type_info_type_of_per_byte_in_str: InternalGasPerByte, "type_info.type_of.per_abstract_memory_unit", 18],
        [type_info_type_name_base: InternalGas, "type_info.type_name.base", 1102],
        // TODO(Gas): the on-chain name is wrong...
        [type_info_type_name_per_byte_in_str: InternalGasPerByte, "type_info.type_name.per_abstract_memory_unit", 18],
        [type_info_chain_id_base: InternalGas, { 4.. => "type_info.chain_id.base" }, 551],

        // TODO(Gas): Fix my cost
        [function_info_check_is_identifier_base: InternalGas, { RELEASE_V1_13.. => "function_info.is_identifier.base" }, 551],
        [function_info_check_is_identifier_per_byte: InternalGasPerByte, { RELEASE_V1_13.. => "function_info.is_identifier.per_byte" }, 3],
        [function_info_check_dispatch_type_compatibility_impl_base: InternalGas, { RELEASE_V1_13.. => "function_info.check_dispatch_type_compatibility_impl.base" }, 1002],
        [function_info_load_function_base: InternalGas, { RELEASE_V1_13.. => "function_info.load_function.base" }, 551],
        [dispatchable_fungible_asset_dispatch_base: InternalGas, { RELEASE_V1_13.. => "dispatchable_fungible_asset.dispatch.base" }, 551],

        // Reusing SHA2-512's cost from Ristretto
        [hash_sha2_512_base: InternalGas, { 4.. => "hash.sha2_512.base" }, 11910],  // 3_240 * 20
        [hash_sha2_512_per_byte: InternalGasPerByte, { 4.. => "hash.sha2_512.per_byte" }, 220], // 60 * 20
        // Back-of-the-envelope approximation from SHA3-256's costs (4000 base, 45 per-byte)
        [hash_sha3_512_base: InternalGas, { 4.. => "hash.sha3_512.base" }, 16542], // 4_500 * 20
        [hash_sha3_512_per_byte: InternalGasPerByte, { 4.. => "hash.sha3_512.per_byte" }, 183], // 50 * 20
        // Using SHA2-256's cost
        [hash_ripemd160_base: InternalGas, { 4.. => "hash.ripemd160.base" }, 11028], // 3000 * 20
        [hash_ripemd160_per_byte: InternalGasPerByte, { 4.. => "hash.ripemd160.per_byte" }, 183], // 50 * 20
        [hash_blake2b_256_base: InternalGas, { 6.. => "hash.blake2b_256.base" }, 6433], // 1750 * 20
        [hash_blake2b_256_per_byte: InternalGasPerByte, { 6.. => "hash.blake2b_256.per_byte" }, 55], // 15 * 20

        [util_from_bytes_base: InternalGas, "util.from_bytes.base", 1102],
        [util_from_bytes_per_byte: InternalGasPerByte, "util.from_bytes.per_byte", 18],

        [transaction_context_get_txn_hash_base: InternalGas, { 10.. => "transaction_context.get_txn_hash.base" }, 735],
        [transaction_context_get_script_hash_base: InternalGas, "transaction_context.get_script_hash.base", 735],
        // Based on SHA3-256's cost
        [transaction_context_generate_unique_address_base: InternalGas, { 10.. => "transaction_context.generate_unique_address.base" }, 14704],
        [transaction_context_sender_base: InternalGas, {RELEASE_V1_12.. => "transaction_context.sender.base"}, 735],
        [transaction_context_secondary_signers_base: InternalGas, {RELEASE_V1_12.. => "transaction_context.secondary_signers.base"}, 735],
        [transaction_context_secondary_signers_per_signer: InternalGasPerArg, {RELEASE_V1_12.. => "transaction_context.secondary_signers.per_signer"}, 576], // 18 * 32
        [transaction_context_fee_payer_base: InternalGas, {RELEASE_V1_12.. => "transaction_context.fee_payer.base"}, 735],
        [transaction_context_max_gas_amount_base: InternalGas, {RELEASE_V1_12.. => "transaction_context.max_gas_amount.base"}, 735],
        [transaction_context_gas_unit_price_base: InternalGas, {RELEASE_V1_12.. => "transaction_context.gas_unit_price.base"}, 735],
        [transaction_context_chain_id_base: InternalGas, {RELEASE_V1_12.. => "transaction_context.chain_id.base"}, 735],
        [transaction_context_entry_function_payload_base: InternalGas, {RELEASE_V1_12.. => "transaction_context.entry_function_payload.base"}, 735],
        [transaction_context_entry_function_payload_per_byte_in_str: InternalGasPerByte, {RELEASE_V1_12.. => "transaction_context.entry_function_payload.per_abstract_memory_unit"}, 18],
        [transaction_context_multisig_payload_base: InternalGas, {RELEASE_V1_12.. => "transaction_context.multisig_payload.base"}, 735],
        [transaction_context_multisig_payload_per_byte_in_str: InternalGasPerByte, {RELEASE_V1_12.. => "transaction_context.multisig_payload.per_abstract_memory_unit"}, 18],

        [code_request_publish_base: InternalGas, "code.request_publish.base", 1838],
        [code_request_publish_per_byte: InternalGasPerByte, "code.request_publish.per_byte", 7],

        [event_write_to_event_store_base: InternalGas, "event.write_to_event_store.base", 20006],
        // TODO(Gas): the on-chain name is wrong...
        [event_write_to_event_store_per_abstract_value_unit: InternalGasPerAbstractValueUnit, "event.write_to_event_store.per_abstract_memory_unit", 61],

        [state_storage_get_usage_base_cost: InternalGas, "state_storage.get_usage.base", 1838],

        [aggregator_add_base: InternalGas, "aggregator.add.base", 1102],
        [aggregator_read_base: InternalGas, "aggregator.read.base", 1102],
        [aggregator_sub_base: InternalGas, "aggregator.sub.base", 1102],
        [aggregator_destroy_base: InternalGas, "aggregator.destroy.base", 1838],
        [aggregator_factory_new_aggregator_base: InternalGas, "aggregator_factory.new_aggregator.base", 1838],

        [aggregator_v2_create_aggregator_base: InternalGas, {RELEASE_V1_9_SKIPPED.. => "aggregator_v2.create_aggregator.base"}, 1838],
        [aggregator_v2_try_add_base: InternalGas, {RELEASE_V1_9_SKIPPED.. => "aggregator_v2.try_add.base"}, 1102],
        [aggregator_v2_try_sub_base: InternalGas, {RELEASE_V1_9_SKIPPED.. => "aggregator_v2.try_sub.base"}, 1102],
        [aggregator_v2_is_at_least_base: InternalGas, {RELEASE_V1_14.. => "aggregator_v2.is_at_least.base"}, 500],

        [aggregator_v2_read_base: InternalGas, {RELEASE_V1_9_SKIPPED.. => "aggregator_v2.read.base"}, 2205],
        [aggregator_v2_snapshot_base: InternalGas, {RELEASE_V1_9_SKIPPED.. => "aggregator_v2.snapshot.base"}, 1102],

        [aggregator_v2_create_snapshot_base: InternalGas, {RELEASE_V1_8.. => "aggregator_v2.create_snapshot.base"}, 1102],
        [aggregator_v2_create_snapshot_per_byte: InternalGasPerByte, { RELEASE_V1_9_SKIPPED.. =>"aggregator_v2.create_snapshot.per_byte" }, 3],
        [aggregator_v2_copy_snapshot_base: InternalGas, {RELEASE_V1_8.. => "aggregator_v2.copy_snapshot.base"}, 1102],
        [aggregator_v2_read_snapshot_base: InternalGas, {RELEASE_V1_8.. => "aggregator_v2.read_snapshot.base"}, 2205],
        [aggregator_v2_string_concat_base: InternalGas, {RELEASE_V1_8.. => "aggregator_v2.string_concat.base"}, 1102],
        [aggregator_v2_string_concat_per_byte: InternalGasPerByte, { RELEASE_V1_9_SKIPPED.. =>"aggregator_v2.string_concat.per_byte" }, 3],

        [object_exists_at_base: InternalGas, { 7.. => "object.exists_at.base" }, 919],
        // Based on SHA3-256's cost
        [object_user_derived_address_base: InternalGas, { RELEASE_V1_12.. => "object.user_derived_address.base" }, 14704],

        // These are dummy value, they copied from storage gas in aptos-core/aptos-vm/src/aptos_vm_impl.rs
        [object_exists_at_per_byte_loaded: InternalGasPerByte, { 7.. => "object.exists_at.per_byte_loaded" }, 183],
        [object_exists_at_per_item_loaded: InternalGas, { 7.. => "object.exists_at.per_item_loaded" }, 1470],
        [string_utils_base: InternalGas, { 8.. => "string_utils.format.base" }, 1102],
        [string_utils_per_byte: InternalGasPerByte, { 8.. =>"string_utils.format.per_byte" }, 3],
    ]
);
