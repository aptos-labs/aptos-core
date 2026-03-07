// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This module defines the gas parameters for Aptos Framework & Stdlib.

use crate::{
    gas_feature_versions::{RELEASE_V1_14, RELEASE_V1_8, RELEASE_V1_9_SKIPPED},
    gas_schedule::NativeGasParameters,
    ver::gas_feature_versions::{
        RELEASE_V1_12, RELEASE_V1_13, RELEASE_V1_23, RELEASE_V1_26, RELEASE_V1_28, RELEASE_V1_36,
        RELEASE_V1_39,
    },
};
use aptos_gas_algebra::{
    InternalGas, InternalGasPerAbstractValueUnit, InternalGasPerArg, InternalGasPerByte,
};

crate::gas_schedule::macros::define_gas_parameters!(
    AptosFrameworkGasParameters,
    "aptos_framework",
    NativeGasParameters => .aptos_framework,
    [
        [account_create_address_base: InternalGas, "account.create_address.base", 11020],
        [account_create_signer_base: InternalGas, "account.create_signer.base", 11020],

        // Permissioned signer gas parameters
        [permission_address_base: InternalGas, { RELEASE_V1_26 => "permissioned_signer.permission_address.base"}, 11020],
        [is_permissioned_signer_base: InternalGas, { RELEASE_V1_26 => "permissioned_signer.is_permissioned_signer.base"}, 11020],
        [signer_from_permissioned_handle_base: InternalGas, { RELEASE_V1_26 => "permissioned_signer.signer_from_permissioned_handle.base"}, 11020],

        // BN254 algebra gas parameters begin.
        // Generated at time 1701559125.5498126 by `scripts/algebra-gas/update_bn254_algebra_gas_params.py` with gas_per_ns=209.10511688369482.
        [algebra_ark_bn254_fq12_add: InternalGas, { 12.. => "algebra.ark_bn254_fq12_add" }, 8090],
        [algebra_ark_bn254_fq12_clone: InternalGas, { 12.. => "algebra.ark_bn254_fq12_clone" }, 8070],
        [algebra_ark_bn254_fq12_deser: InternalGas, { 12.. => "algebra.ark_bn254_fq12_deser" }, 237210],
        [algebra_ark_bn254_fq12_div: InternalGas, { 12.. => "algebra.ark_bn254_fq12_div" }, 5171400],
        [algebra_ark_bn254_fq12_eq: InternalGas, { 12.. => "algebra.ark_bn254_fq12_eq" }, 22310],
        [algebra_ark_bn254_fq12_from_u64: InternalGas, { 12.. => "algebra.ark_bn254_fq12_from_u64" }, 26580],
        [algebra_ark_bn254_fq12_inv: InternalGas, { 12.. => "algebra.ark_bn254_fq12_inv" }, 3985550],
        [algebra_ark_bn254_fq12_mul: InternalGas, { 12.. => "algebra.ark_bn254_fq12_mul" }, 1183510],
        [algebra_ark_bn254_fq12_neg: InternalGas, { 12.. => "algebra.ark_bn254_fq12_neg" }, 24460],
        [algebra_ark_bn254_fq12_one: InternalGas, { 12.. => "algebra.ark_bn254_fq12_one" }, 380],
        [algebra_ark_bn254_fq12_pow_u256: InternalGas, { 12.. => "algebra.ark_bn254_fq12_pow_u256" }, 354498260],
        [algebra_ark_bn254_fq12_serialize: InternalGas, { 12.. => "algebra.ark_bn254_fq12_serialize" }, 215660],
        [algebra_ark_bn254_fq12_square: InternalGas, { 12.. => "algebra.ark_bn254_fq12_square" }, 861930],
        [algebra_ark_bn254_fq12_sub: InternalGas, { 12.. => "algebra.ark_bn254_fq12_sub" }, 56050],
        [algebra_ark_bn254_fq12_zero: InternalGas, { 12.. => "algebra.ark_bn254_fq12_zero" }, 380],
        [algebra_ark_bn254_fq_add: InternalGas, { 12.. => "algebra.ark_bn254_fq_add" }, 8030],
        [algebra_ark_bn254_fq_clone: InternalGas, { 12.. => "algebra.ark_bn254_fq_clone" }, 7920],
        [algebra_ark_bn254_fq_deser: InternalGas, { 12.. => "algebra.ark_bn254_fq_deser" }, 32320],
        [algebra_ark_bn254_fq_div: InternalGas, { 12.. => "algebra.ark_bn254_fq_div" }, 2096310],
        [algebra_ark_bn254_fq_eq: InternalGas, { 12.. => "algebra.ark_bn254_fq_eq" }, 8030],
        [algebra_ark_bn254_fq_from_u64: InternalGas, { 12.. => "algebra.ark_bn254_fq_from_u64" }, 25980],
        [algebra_ark_bn254_fq_inv: InternalGas, { 12.. => "algebra.ark_bn254_fq_inv" }, 2089020],
        [algebra_ark_bn254_fq_mul: InternalGas, { 12.. => "algebra.ark_bn254_fq_mul" }, 18470],
        [algebra_ark_bn254_fq_neg: InternalGas, { 12.. => "algebra.ark_bn254_fq_neg" }, 7920],
        [algebra_ark_bn254_fq_one: InternalGas, { 12.. => "algebra.ark_bn254_fq_one" }, 380],
        [algebra_ark_bn254_fq_pow_u256: InternalGas, { 12.. => "algebra.ark_bn254_fq_pow_u256" }, 3825700],
        [algebra_ark_bn254_fq_serialize: InternalGas, { 12.. => "algebra.ark_bn254_fq_serialize" }, 47670],
        [algebra_ark_bn254_fq_square: InternalGas, { 12.. => "algebra.ark_bn254_fq_square" }, 7920],
        [algebra_ark_bn254_fq_sub: InternalGas, { 12.. => "algebra.ark_bn254_fq_sub" }, 11300],
        [algebra_ark_bn254_fq_zero: InternalGas, { 12.. => "algebra.ark_bn254_fq_zero" }, 380],
        [algebra_ark_bn254_fr_add: InternalGas, { 12.. => "algebra.ark_bn254_fr_add" }, 8040],
        [algebra_ark_bn254_fr_deser: InternalGas, { 12.. => "algebra.ark_bn254_fr_deser" }, 30730],
        [algebra_ark_bn254_fr_div: InternalGas, { 12.. => "algebra.ark_bn254_fr_div" }, 2238570],
        [algebra_ark_bn254_fr_eq: InternalGas, { 12.. => "algebra.ark_bn254_fr_eq" }, 8070],
        [algebra_ark_bn254_fr_from_u64: InternalGas, { 12.. => "algebra.ark_bn254_fr_from_u64" }, 24780],
        [algebra_ark_bn254_fr_inv: InternalGas, { 12.. => "algebra.ark_bn254_fr_inv" }, 2222160],
        [algebra_ark_bn254_fr_mul: InternalGas, { 12.. => "algebra.ark_bn254_fr_mul" }, 18130],
        [algebra_ark_bn254_fr_neg: InternalGas, { 12.. => "algebra.ark_bn254_fr_neg" }, 7920],
        [algebra_ark_bn254_fr_one: InternalGas, { 12.. => "algebra.ark_bn254_fr_one" }, 0],
        [algebra_ark_bn254_fr_serialize: InternalGas, { 12.. => "algebra.ark_bn254_fr_serialize" }, 47320],
        [algebra_ark_bn254_fr_square: InternalGas, { 12.. => "algebra.ark_bn254_fr_square" }, 7920],
        [algebra_ark_bn254_fr_sub: InternalGas, { 12.. => "algebra.ark_bn254_fr_sub" }, 19060],
        [algebra_ark_bn254_fr_zero: InternalGas, { 12.. => "algebra.ark_bn254_fr_zero" }, 380],
        [algebra_ark_bn254_g1_affine_deser_comp: InternalGas, { 12.. => "algebra.ark_bn254_g1_affine_deser_comp" }, 43188090],
        [algebra_ark_bn254_g1_affine_deser_uncomp: InternalGas, { 12.. => "algebra.ark_bn254_g1_affine_deser_uncomp" }, 39569760],
        [algebra_ark_bn254_g1_affine_serialize_comp: InternalGas, { 12.. => "algebra.ark_bn254_g1_affine_serialize_comp" }, 82570],
        [algebra_ark_bn254_g1_affine_serialize_uncomp: InternalGas, { 12.. => "algebra.ark_bn254_g1_affine_serialize_uncomp" }, 108110],
        [algebra_ark_bn254_g1_proj_add: InternalGas, { 12.. => "algebra.ark_bn254_g1_proj_add" }, 195740],
        [algebra_ark_bn254_g1_proj_double: InternalGas, { 12.. => "algebra.ark_bn254_g1_proj_double" }, 117040],
        [algebra_ark_bn254_g1_proj_eq: InternalGas, { 12.. => "algebra.ark_bn254_g1_proj_eq" }, 97450],
        [algebra_ark_bn254_g1_proj_generator: InternalGas, { 12.. => "algebra.ark_bn254_g1_proj_generator" }, 380],
        [algebra_ark_bn254_g1_proj_infinity: InternalGas, { 12.. => "algebra.ark_bn254_g1_proj_infinity" }, 380],
        [algebra_ark_bn254_g1_proj_neg: InternalGas, { 12.. => "algebra.ark_bn254_g1_proj_neg" }, 380],
        [algebra_ark_bn254_g1_proj_scalar_mul: InternalGas, { 12.. => "algebra.ark_bn254_g1_proj_scalar_mul" }, 48626830],
        [algebra_ark_bn254_g1_proj_sub: InternalGas, { 12.. => "algebra.ark_bn254_g1_proj_sub" }, 196480],
        [algebra_ark_bn254_g1_proj_to_affine: InternalGas, { 12.. => "algebra.ark_bn254_g1_proj_to_affine" }, 11650],
        [algebra_ark_bn254_g2_affine_deser_comp: InternalGas, { 12.. => "algebra.ark_bn254_g2_affine_deser_comp" }, 124451380],
        [algebra_ark_bn254_g2_affine_deser_uncomp: InternalGas, { 12.. => "algebra.ark_bn254_g2_affine_deser_uncomp" }, 111525410],
        [algebra_ark_bn254_g2_affine_serialize_comp: InternalGas, { 12.. => "algebra.ark_bn254_g2_affine_serialize_comp" }, 127210],
        [algebra_ark_bn254_g2_affine_serialize_uncomp: InternalGas, { 12.. => "algebra.ark_bn254_g2_affine_serialize_uncomp" }, 181050],
        [algebra_ark_bn254_g2_proj_add: InternalGas, { 12.. => "algebra.ark_bn254_g2_proj_add" }, 584910],
        [algebra_ark_bn254_g2_proj_double: InternalGas, { 12.. => "algebra.ark_bn254_g2_proj_double" }, 292010],
        [algebra_ark_bn254_g2_proj_eq: InternalGas, { 12.. => "algebra.ark_bn254_g2_proj_eq" }, 259810],
        [algebra_ark_bn254_g2_proj_generator: InternalGas, { 12.. => "algebra.ark_bn254_g2_proj_generator" }, 380],
        [algebra_ark_bn254_g2_proj_infinity: InternalGas, { 12.. => "algebra.ark_bn254_g2_proj_infinity" }, 380],
        [algebra_ark_bn254_g2_proj_neg: InternalGas, { 12.. => "algebra.ark_bn254_g2_proj_neg" }, 380],
        [algebra_ark_bn254_g2_proj_scalar_mul: InternalGas, { 12.. => "algebra.ark_bn254_g2_proj_scalar_mul" }, 140415480],
        [algebra_ark_bn254_g2_proj_sub: InternalGas, { 12.. => "algebra.ark_bn254_g2_proj_sub" }, 591330],
        [algebra_ark_bn254_g2_proj_to_affine: InternalGas, { 12.. => "algebra.ark_bn254_g2_proj_to_affine" }, 2301000],
        [algebra_ark_bn254_multi_pairing_base: InternalGas, { 12.. => "algebra.ark_bn254_multi_pairing_base" }, 234886460],
        [algebra_ark_bn254_multi_pairing_per_pair: InternalGasPerArg, { 12.. => "algebra.ark_bn254_multi_pairing_per_pair" }, 124293990],
        [algebra_ark_bn254_pairing: InternalGas, { 12.. => "algebra.ark_bn254_pairing" }, 385435650],
        // BN254 algebra gas parameters end.

        // BLS12-381 algebra gas parameters begin.
        // Generated at time 1680606720.0709136 by `scripts/algebra-gas/update_algebra_gas_params.py` with gas_per_ns=204.6.
        [algebra_ark_bls12_381_fq12_add: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_add" }, 66860],
        [algebra_ark_bls12_381_fq12_clone: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_clone" }, 7750],
        [algebra_ark_bls12_381_fq12_deser: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_deser" }, 410970],
        [algebra_ark_bls12_381_fq12_div: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_div" }, 9219880],
        [algebra_ark_bls12_381_fq12_eq: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_eq" }, 26680],
        [algebra_ark_bls12_381_fq12_from_u64: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_from_u64" }, 33120],
        [algebra_ark_bls12_381_fq12_inv: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_inv" }, 7371220],
        [algebra_ark_bls12_381_fq12_mul: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_mul" }, 1833800],
        [algebra_ark_bls12_381_fq12_neg: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_neg" }, 43410],
        [algebra_ark_bls12_381_fq12_one: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_one" }, 400],
        [algebra_ark_bls12_381_fq12_pow_u256: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_pow_u256" }, 539056240],
        [algebra_ark_bls12_381_fq12_serialize: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_serialize" }, 296940],
        [algebra_ark_bls12_381_fq12_square: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_square" }, 1291930],
        [algebra_ark_bls12_381_fq12_sub: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_sub" }, 64620],
        [algebra_ark_bls12_381_fq12_zero: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_zero" }, 7750],
        [algebra_ark_bls12_381_fr_add: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_add" }, 7750],
        [algebra_ark_bls12_381_fr_deser: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_deser" }, 27640],
        [algebra_ark_bls12_381_fr_div: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_div" }, 2185010],
        [algebra_ark_bls12_381_fr_eq: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_eq" }, 7790],
        [algebra_ark_bls12_381_fr_from_u64: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_from_u64" }, 18150],
        [algebra_ark_bls12_381_fr_inv: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_inv" }, 2154500],
        [algebra_ark_bls12_381_fr_mul: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_mul" }, 18450],
        [algebra_ark_bls12_381_fr_neg: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_neg" }, 7820],
        [algebra_ark_bls12_381_fr_one: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_one" }, 7750],
        [algebra_ark_bls12_381_fr_serialize: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_serialize" }, 40540],
        [algebra_ark_bls12_381_fr_square: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_square" }, 17460],
        [algebra_ark_bls12_381_fr_sub: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_sub" }, 10660],
        [algebra_ark_bls12_381_fr_zero: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_zero" }, 7750],
        [algebra_ark_bls12_381_g1_affine_deser_comp: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_affine_deser_comp" }, 37848050],
        [algebra_ark_bls12_381_g1_affine_deser_uncomp: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_affine_deser_uncomp" }, 26490650],
        [algebra_ark_bls12_381_g1_affine_serialize_comp: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_affine_serialize_comp" }, 74030],
        [algebra_ark_bls12_381_g1_affine_serialize_uncomp: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_affine_serialize_uncomp" }, 89430],
        [algebra_ark_bls12_381_g1_proj_add: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_proj_add" }, 397220],
        [algebra_ark_bls12_381_g1_proj_double: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_proj_double" }, 193500],
        [algebra_ark_bls12_381_g1_proj_eq: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_proj_eq" }, 185080],
        [algebra_ark_bls12_381_g1_proj_generator: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_proj_generator" }, 400],
        [algebra_ark_bls12_381_g1_proj_infinity: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_proj_infinity" }, 400],
        [algebra_ark_bls12_381_g1_proj_neg: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_proj_neg" }, 400],
        [algebra_ark_bls12_381_g1_proj_scalar_mul: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_proj_scalar_mul" }, 92764630],
        [algebra_ark_bls12_381_g1_proj_sub: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_proj_sub" }, 409760],
        [algebra_ark_bls12_381_g1_proj_to_affine: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_proj_to_affine" }, 4449240],
        [algebra_ark_bls12_381_g2_affine_deser_comp: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_affine_deser_comp" }, 75728090],
        [algebra_ark_bls12_381_g2_affine_deser_uncomp: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_affine_deser_uncomp" }, 37420900],
        [algebra_ark_bls12_381_g2_affine_serialize_comp: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_affine_serialize_comp" }, 124170],
        [algebra_ark_bls12_381_g2_affine_serialize_uncomp: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_affine_serialize_uncomp" }, 155010],
        [algebra_ark_bls12_381_g2_proj_add: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_proj_add" }, 1191060],
        [algebra_ark_bls12_381_g2_proj_double: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_proj_double" }, 545480],
        [algebra_ark_bls12_381_g2_proj_eq: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_proj_eq" }, 557090],
        [algebra_ark_bls12_381_g2_proj_generator: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_proj_generator" }, 400],
        [algebra_ark_bls12_381_g2_proj_infinity: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_proj_infinity" }, 400],
        [algebra_ark_bls12_381_g2_proj_neg: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_proj_neg" }, 400],
        [algebra_ark_bls12_381_g2_proj_scalar_mul: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_proj_scalar_mul" }, 276674430],
        [algebra_ark_bls12_381_g2_proj_sub: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_proj_sub" }, 1208260],
        [algebra_ark_bls12_381_g2_proj_to_affine: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_proj_to_affine" }, 4736780],
        [algebra_ark_bls12_381_multi_pairing_base: InternalGas, { 8.. => "algebra.ark_bls12_381_multi_pairing_base" }, 330790330],
        [algebra_ark_bls12_381_multi_pairing_per_pair: InternalGasPerArg, { 8.. => "algebra.ark_bls12_381_multi_pairing_per_pair" }, 169193110],
        [algebra_ark_bls12_381_pairing: InternalGas, { 8.. => "algebra.ark_bls12_381_pairing" }, 545232400],
        [algebra_ark_h2c_bls12381g1_xmd_sha256_sswu_base: InternalGas, { 8.. => "algebra.ark_h2c_bls12381g1_xmd_sha256_sswu_base" }, 119541420],
        [algebra_ark_h2c_bls12381g1_xmd_sha256_sswu_per_msg_byte: InternalGasPerByte, { 8.. => "algebra.ark_h2c_bls12381g1_xmd_sha256_sswu_per_msg_byte" }, 1760],
        [algebra_ark_h2c_bls12381g2_xmd_sha256_sswu_base: InternalGas, { 8.. => "algebra.ark_h2c_bls12381g2_xmd_sha256_sswu_base" }, 248975550],
        [algebra_ark_h2c_bls12381g2_xmd_sha256_sswu_per_msg_byte: InternalGasPerByte, { 8.. => "algebra.ark_h2c_bls12381g2_xmd_sha256_sswu_per_msg_byte" }, 1760],
        // BLS12-381 algebra gas parameters end.

        [bls12381_base: InternalGas, "bls12381.base", 5510],

        [bls12381_per_pubkey_deserialize: InternalGasPerArg, "bls12381.per_pubkey_deserialize", 4006840],
        [bls12381_per_pubkey_aggregate: InternalGasPerArg, "bls12381.per_pubkey_aggregate", 154390],
        [bls12381_per_pubkey_subgroup_check: InternalGasPerArg, "bls12381.per_pubkey_subgroup_check", 13601200],

        [bls12381_per_sig_deserialize: InternalGasPerArg, "bls12381.per_sig_deserialize", 8160720],
        [bls12381_per_sig_aggregate: InternalGasPerArg, "bls12381.per_sig_aggregate", 428250],
        [bls12381_per_sig_subgroup_check: InternalGasPerArg, "bls12381.per_sig_subgroup_check", 16927980],

        [bls12381_per_sig_verify: InternalGasPerArg, "bls12381.per_sig_verify", 311908600],
        [bls12381_per_pop_verify: InternalGasPerArg, "bls12381.per_pop_verify", 378628000],

        [bls12381_per_pairing: InternalGasPerArg, "bls12381.per_pairing", 147517880],

        [bls12381_per_msg_hashing: InternalGasPerArg, "bls12381.per_msg_hashing", 56610400],
        [bls12381_per_byte_hashing: InternalGasPerByte, "bls12381.per_byte_hashing", 1830],

        [ed25519_base: InternalGas, "signature.base", 5510],
        [ed25519_per_pubkey_deserialize: InternalGasPerArg, "signature.per_pubkey_deserialize", 1396880],
        [ed25519_per_pubkey_small_order_check: InternalGasPerArg, "signature.per_pubkey_small_order_check", 233420],
        [ed25519_per_sig_deserialize: InternalGasPerArg, "signature.per_sig_deserialize", 13780],
        [ed25519_per_sig_strict_verify: InternalGasPerArg, "signature.per_sig_strict_verify", 9814920],
        [ed25519_per_msg_hashing_base: InternalGasPerArg, "signature.per_msg_hashing_base", 119100],
        [ed25519_per_msg_byte_hashing: InternalGasPerByte, "signature.per_msg_byte_hashing", 2200],

        [secp256k1_base: InternalGas, "secp256k1.base", 5510],
        [secp256k1_ecdsa_recover: InternalGasPerArg, "secp256k1.ecdsa_recover", 59183600],

        [ristretto255_basepoint_mul: InternalGasPerArg, "ristretto255.basepoint_mul", 4705280],
        [ristretto255_basepoint_double_mul: InternalGasPerArg, "ristretto255.basepoint_double_mul", 16174400],

        [ristretto255_point_add: InternalGasPerArg, "ristretto255.point_add", 78480],
        [ristretto255_point_clone: InternalGasPerArg, { 11.. => "ristretto255.point_clone" }, 5510],
        [ristretto255_point_compress: InternalGasPerArg, "ristretto255.point_compress", 1470400],
        [ristretto255_point_decompress: InternalGasPerArg, "ristretto255.point_decompress", 1488780],
        [ristretto255_point_equals: InternalGasPerArg, "ristretto255.point_equals", 84540],
        [ristretto255_point_from_64_uniform_bytes: InternalGasPerArg, "ristretto255.point_from_64_uniform_bytes", 2995940],
        [ristretto255_point_identity: InternalGasPerArg, "ristretto255.point_identity", 5510],
        [ristretto255_point_mul: InternalGasPerArg, "ristretto255.point_mul", 17313960],
        [ristretto255_point_double_mul: InternalGasPerArg, { 11.. => "ristretto255.point_double_mul" }, 18699070],
        [ristretto255_point_neg: InternalGasPerArg, "ristretto255.point_neg", 13230],
        [ristretto255_point_sub: InternalGasPerArg, "ristretto255.point_sub", 78290],
        [ristretto255_point_parse_arg: InternalGasPerArg, "ristretto255.point_parse_arg", 5510],


        // TODO(Alin): These SHA512 gas costs could be unified with the costs in our future SHA512 module
        // (assuming same implementation complexity, which might not be the case
        [ristretto255_sha512_per_byte: InternalGasPerByte, "ristretto255.scalar_sha512_per_byte", 2200],
        [ristretto255_sha512_per_hash: InternalGasPerArg, "ristretto255.scalar_sha512_per_hash", 119100],

        [ristretto255_scalar_add: InternalGasPerArg, "ristretto255.scalar_add", 28300],
        [ristretto255_scalar_reduced_from_32_bytes: InternalGasPerArg, "ristretto255.scalar_reduced_from_32_bytes", 26090],
        [ristretto255_scalar_uniform_from_64_bytes: InternalGasPerArg, "ristretto255.scalar_uniform_from_64_bytes", 45760],
        [ristretto255_scalar_from_u128: InternalGasPerArg, "ristretto255.scalar_from_u128", 6430],
        [ristretto255_scalar_from_u64: InternalGasPerArg, "ristretto255.scalar_from_u64", 6430],
        [ristretto255_scalar_invert: InternalGasPerArg, "ristretto255.scalar_invert", 4043600],
        [ristretto255_scalar_is_canonical: InternalGasPerArg, "ristretto255.scalar_is_canonical", 42270],
        [ristretto255_scalar_mul: InternalGasPerArg, "ristretto255.scalar_mul", 39140],
        [ristretto255_scalar_neg: InternalGasPerArg, "ristretto255.scalar_neg", 26650],
        [ristretto255_scalar_sub: InternalGasPerArg, "ristretto255.scalar_sub", 38960],
        [ristretto255_scalar_parse_arg: InternalGasPerArg, "ristretto255.scalar_parse_arg", 5510],

        [hash_sip_hash_base: InternalGas, "hash.sip_hash.base", 36760],
        [hash_sip_hash_per_byte: InternalGasPerByte, "hash.sip_hash.per_byte", 730],

        [hash_keccak256_base: InternalGas, { 1.. => "hash.keccak256.base" }, 147040],
        [hash_keccak256_per_byte: InternalGasPerByte, { 1.. => "hash.keccak256.per_byte" }, 1650],

        // Bulletproofs gas parameters begin.
        // Generated at time 1683148919.0628748 by `scripts/algebra-gas/update_bulletproofs_gas_params.py` with gas_per_ns=10.0.
        [bulletproofs_base: InternalGas, { 11.. => "bulletproofs.base" }, 117946510],
        [bulletproofs_per_bit_rangeproof_verify: InternalGasPerArg, { 11.. => "bulletproofs.per_bit_rangeproof_verify" }, 10042530],
        [bulletproofs_per_byte_rangeproof_deserialize: InternalGasPerByte, { 11.. => "bulletproofs.per_byte_rangeproof_deserialize" }, 1210],
        // Bulletproofs gas parameters end.

        // Bulletproofs batch verify gas parameters begin.
        // Generated at time 1738897425.2325199 by `scripts/algebra-gas/update_bulletproofs_batch_verify_gas_params.py` with gas_per_ns=37.59.
        [bulletproofs_verify_base_batch_1_bits_8: InternalGas, { RELEASE_V1_28.. => "bulletproofs.verify.base_batch_1_bits_8" }, 170_995_010],
        [bulletproofs_verify_base_batch_1_bits_16: InternalGas, { RELEASE_V1_28.. => "bulletproofs.verify.base_batch_1_bits_16" }, 250_279_620],
        [bulletproofs_verify_base_batch_1_bits_32: InternalGas, { RELEASE_V1_28.. => "bulletproofs.verify.base_batch_1_bits_32" }, 397_399_290],
        [bulletproofs_verify_base_batch_1_bits_64: InternalGas, { RELEASE_V1_28.. => "bulletproofs.verify.base_batch_1_bits_64" }, 677_482_180],
        [bulletproofs_verify_base_batch_2_bits_8: InternalGas, { RELEASE_V1_28.. => "bulletproofs.verify.base_batch_2_bits_8" }, 256_454_490],
        [bulletproofs_verify_base_batch_2_bits_16: InternalGas, { RELEASE_V1_28.. => "bulletproofs.verify.base_batch_2_bits_16" }, 402_073_830],
        [bulletproofs_verify_base_batch_2_bits_32: InternalGas, { RELEASE_V1_28.. => "bulletproofs.verify.base_batch_2_bits_32" }, 684_989_840],
        [bulletproofs_verify_base_batch_2_bits_64: InternalGas, { RELEASE_V1_28.. => "bulletproofs.verify.base_batch_2_bits_64" }, 1_180_698_990],
        [bulletproofs_verify_base_batch_4_bits_8: InternalGas, { RELEASE_V1_28.. => "bulletproofs.verify.base_batch_4_bits_8" }, 414_711_270],
        [bulletproofs_verify_base_batch_4_bits_16: InternalGas, { RELEASE_V1_28.. => "bulletproofs.verify.base_batch_4_bits_16" }, 693_597_280],
        [bulletproofs_verify_base_batch_4_bits_32: InternalGas, { RELEASE_V1_28.. => "bulletproofs.verify.base_batch_4_bits_32" }, 1_186_974_640],
        [bulletproofs_verify_base_batch_4_bits_64: InternalGas, { RELEASE_V1_28.. => "bulletproofs.verify.base_batch_4_bits_64" }, 1_966_896_380],
        [bulletproofs_verify_base_batch_8_bits_8: InternalGas, { RELEASE_V1_28.. => "bulletproofs.verify.base_batch_8_bits_8" }, 719_329_070],
        [bulletproofs_verify_base_batch_8_bits_16: InternalGas, { RELEASE_V1_28.. => "bulletproofs.verify.base_batch_8_bits_16" }, 1_209_744_780],
        [bulletproofs_verify_base_batch_8_bits_32: InternalGas, { RELEASE_V1_28.. => "bulletproofs.verify.base_batch_8_bits_32" }, 1_986_708_380],
        [bulletproofs_verify_base_batch_8_bits_64: InternalGas, { RELEASE_V1_28.. => "bulletproofs.verify.base_batch_8_bits_64" }, 3_393_916_150],
        [bulletproofs_verify_base_batch_16_bits_8: InternalGas, { RELEASE_V1_28.. => "bulletproofs.verify.base_batch_16_bits_8" }, 1_249_502_790],
        [bulletproofs_verify_base_batch_16_bits_16: InternalGas, { RELEASE_V1_28.. => "bulletproofs.verify.base_batch_16_bits_16" }, 2_023_933_570],
        [bulletproofs_verify_base_batch_16_bits_32: InternalGas, { RELEASE_V1_28.. => "bulletproofs.verify.base_batch_16_bits_32" }, 3_442_226_440],
        [bulletproofs_verify_base_batch_16_bits_64: InternalGas, { RELEASE_V1_28.. => "bulletproofs.verify.base_batch_16_bits_64" }, 6_039_527_790],
        // Bulletproofs batch verify gas parameters end.

        [type_info_type_of_base: InternalGas, "type_info.type_of.base", 11020],
        // TODO(Gas): the on-chain name is wrong...
        [type_info_type_of_per_byte_in_str: InternalGasPerByte, "type_info.type_of.per_abstract_memory_unit", 180],
        [type_info_type_name_base: InternalGas, "type_info.type_name.base", 11020],
        // TODO(Gas): the on-chain name is wrong...
        [type_info_type_name_per_byte_in_str: InternalGasPerByte, "type_info.type_name.per_abstract_memory_unit", 180],
        [type_info_chain_id_base: InternalGas, { 4.. => "type_info.chain_id.base" }, 5510],

        // TODO(Gas): Fix my cost
        [function_info_check_is_identifier_base: InternalGas, { RELEASE_V1_13.. => "function_info.is_identifier.base" }, 5510],
        [function_info_check_is_identifier_per_byte: InternalGasPerByte, { RELEASE_V1_13.. => "function_info.is_identifier.per_byte" }, 30],
        [function_info_check_dispatch_type_compatibility_impl_base: InternalGas, { RELEASE_V1_13.. => "function_info.check_dispatch_type_compatibility_impl.base" }, 10020],
        [function_info_load_function_base: InternalGas, { RELEASE_V1_13.. => "function_info.load_function.base" }, 5510],
        [dispatchable_fungible_asset_dispatch_base: InternalGas, { RELEASE_V1_13.. => "dispatchable_fungible_asset.dispatch.base" }, 5510],
        [dispatchable_authenticate_dispatch_base: InternalGas, { RELEASE_V1_26.. => "dispatchable_authenticate.dispatch.base" }, 5510],

        // Reusing SHA2-512's cost from Ristretto
        [hash_sha2_512_base: InternalGas, { 4.. => "hash.sha2_512.base" }, 119100],  // 3_240 * 20
        [hash_sha2_512_per_byte: InternalGasPerByte, { 4.. => "hash.sha2_512.per_byte" }, 2200], // 60 * 20
        // Back-of-the-envelope approximation from SHA3-256's costs (4000 base, 45 per-byte)
        [hash_sha3_512_base: InternalGas, { 4.. => "hash.sha3_512.base" }, 165420], // 4_500 * 20
        [hash_sha3_512_per_byte: InternalGasPerByte, { 4.. => "hash.sha3_512.per_byte" }, 1830], // 50 * 20
        // Using SHA2-256's cost
        [hash_ripemd160_base: InternalGas, { 4.. => "hash.ripemd160.base" }, 110280], // 3000 * 20
        [hash_ripemd160_per_byte: InternalGasPerByte, { 4.. => "hash.ripemd160.per_byte" }, 1830], // 50 * 20
        [hash_blake2b_256_base: InternalGas, { 6.. => "hash.blake2b_256.base" }, 64330], // 1750 * 20
        [hash_blake2b_256_per_byte: InternalGasPerByte, { 6.. => "hash.blake2b_256.per_byte" }, 550], // 15 * 20

        [util_from_bytes_base: InternalGas, "util.from_bytes.base", 11020],
        [util_from_bytes_per_byte: InternalGasPerByte, "util.from_bytes.per_byte", 180],

        [transaction_context_get_txn_hash_base: InternalGas, { 10.. => "transaction_context.get_txn_hash.base" }, 7350],
        [transaction_context_get_script_hash_base: InternalGas, "transaction_context.get_script_hash.base", 7350],
        // Based on SHA3-256's cost
        [transaction_context_generate_unique_address_base: InternalGas, { 10.. => "transaction_context.generate_unique_address.base" }, 147040],
        [transaction_context_monotonically_increasing_counter_base: InternalGas, {RELEASE_V1_36.. => "transaction_context.monotonically_increasing_counter.base"}, 7350],
        [transaction_context_sender_base: InternalGas, {RELEASE_V1_12.. => "transaction_context.sender.base"}, 7350],
        [transaction_context_secondary_signers_base: InternalGas, {RELEASE_V1_12.. => "transaction_context.secondary_signers.base"}, 7350],
        [transaction_context_secondary_signers_per_signer: InternalGasPerArg, {RELEASE_V1_12.. => "transaction_context.secondary_signers.per_signer"}, 5760], // 18 * 32
        [transaction_context_fee_payer_base: InternalGas, {RELEASE_V1_12.. => "transaction_context.fee_payer.base"}, 7350],
        [transaction_context_max_gas_amount_base: InternalGas, {RELEASE_V1_12.. => "transaction_context.max_gas_amount.base"}, 7350],
        [transaction_context_gas_unit_price_base: InternalGas, {RELEASE_V1_12.. => "transaction_context.gas_unit_price.base"}, 7350],
        [transaction_context_chain_id_base: InternalGas, {RELEASE_V1_12.. => "transaction_context.chain_id.base"}, 7350],
        [transaction_context_entry_function_payload_base: InternalGas, {RELEASE_V1_12.. => "transaction_context.entry_function_payload.base"}, 7350],
        [transaction_context_entry_function_payload_per_byte_in_str: InternalGasPerByte, {RELEASE_V1_12.. => "transaction_context.entry_function_payload.per_abstract_memory_unit"}, 180],
        [transaction_context_multisig_payload_base: InternalGas, {RELEASE_V1_12.. => "transaction_context.multisig_payload.base"}, 7350],
        [transaction_context_multisig_payload_per_byte_in_str: InternalGasPerByte, {RELEASE_V1_12.. => "transaction_context.multisig_payload.per_abstract_memory_unit"}, 180],

        [code_request_publish_base: InternalGas, "code.request_publish.base", 18380],
        [code_request_publish_per_byte: InternalGasPerByte, "code.request_publish.per_byte", 70],

        [event_write_to_event_store_base: InternalGas, "event.write_to_event_store.base", 200060],
        // TODO(Gas): the on-chain name is wrong...
        [event_write_to_event_store_per_abstract_value_unit: InternalGasPerAbstractValueUnit, "event.write_to_event_store.per_abstract_memory_unit", 610],

        [state_storage_get_usage_base_cost: InternalGas, "state_storage.get_usage.base", 18380],

        [aggregator_add_base: InternalGas, "aggregator.add.base", 11020],
        [aggregator_read_base: InternalGas, "aggregator.read.base", 11020],
        [aggregator_sub_base: InternalGas, "aggregator.sub.base", 11020],
        [aggregator_destroy_base: InternalGas, "aggregator.destroy.base", 18380],
        [aggregator_factory_new_aggregator_base: InternalGas, "aggregator_factory.new_aggregator.base", 18380],

        [aggregator_v2_create_aggregator_base: InternalGas, {RELEASE_V1_9_SKIPPED.. => "aggregator_v2.create_aggregator.base"}, 18380],
        [aggregator_v2_try_add_base: InternalGas, {RELEASE_V1_9_SKIPPED.. => "aggregator_v2.try_add.base"}, 11020],
        [aggregator_v2_try_sub_base: InternalGas, {RELEASE_V1_9_SKIPPED.. => "aggregator_v2.try_sub.base"}, 11020],
        [aggregator_v2_is_at_least_base: InternalGas, {RELEASE_V1_14.. => "aggregator_v2.is_at_least.base"}, 5000],

        [aggregator_v2_read_base: InternalGas, {RELEASE_V1_9_SKIPPED.. => "aggregator_v2.read.base"}, 22050],
        [aggregator_v2_snapshot_base: InternalGas, {RELEASE_V1_9_SKIPPED.. => "aggregator_v2.snapshot.base"}, 11020],

        [aggregator_v2_create_snapshot_base: InternalGas, {RELEASE_V1_8.. => "aggregator_v2.create_snapshot.base"}, 11020],
        [aggregator_v2_create_snapshot_per_byte: InternalGasPerByte, { RELEASE_V1_9_SKIPPED.. =>"aggregator_v2.create_snapshot.per_byte" }, 30],
        [aggregator_v2_copy_snapshot_base: InternalGas, {RELEASE_V1_8.. => "aggregator_v2.copy_snapshot.base"}, 11020],
        [aggregator_v2_read_snapshot_base: InternalGas, {RELEASE_V1_8.. => "aggregator_v2.read_snapshot.base"}, 22050],
        [aggregator_v2_string_concat_base: InternalGas, {RELEASE_V1_8.. => "aggregator_v2.string_concat.base"}, 11020],
        [aggregator_v2_string_concat_per_byte: InternalGasPerByte, { RELEASE_V1_9_SKIPPED.. =>"aggregator_v2.string_concat.per_byte" }, 30],

        [object_exists_at_base: InternalGas, { 7.. => "object.exists_at.base" }, 9190],
        // Based on SHA3-256's cost
        [object_user_derived_address_base: InternalGas, { RELEASE_V1_12.. => "object.user_derived_address.base" }, 147040],

        // These are dummy value, they copied from storage gas in aptos-core/aptos-vm/src/aptos_vm_impl.rs
        [object_exists_at_per_byte_loaded: InternalGasPerByte, { 7.. => "object.exists_at.per_byte_loaded" }, 1830],
        [object_exists_at_per_item_loaded: InternalGas, { 7.. => "object.exists_at.per_item_loaded" }, 14700],
        [string_utils_base: InternalGas, { 8.. => "string_utils.format.base" }, 11020],
        [string_utils_per_byte: InternalGasPerByte, { 8.. =>"string_utils.format.per_byte" }, 30],

        [randomness_fetch_and_inc_counter: InternalGas, { RELEASE_V1_23.. => "randomness.fetch_and_inc_counter" }, 10],

        // Reflection
        [reflect_resolve_base: InternalGas, { RELEASE_V1_39.. => "reflect.resolve_base" }, 40960],
    ]
);
