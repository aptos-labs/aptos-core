// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module defines the gas parameters for Aptos Framework & Stdlib.

use crate::gas_schedule::NativeGasParameters;
use aptos_gas_algebra::{
    InternalGas, InternalGasPerAbstractValueUnit, InternalGasPerArg, InternalGasPerByte,
};

crate::gas_schedule::macros::define_gas_parameters!(
    AptosFrameworkGasParameters,
    "aptos_framework",
    NativeGasParameters => .aptos_framework,
    [
        [account_create_address_base: InternalGas, "account.create_address.base", 6000],
        [account_create_signer_base: InternalGas, "account.create_signer.base", 6000],

        // BN254 algebra gas parameters begin.
        // Generated at time 1701559125.5498126 by `scripts/algebra-gas/update_bn254_algebra_gas_params.py` with gas_per_ns=209.10511688369482.
        [algebra_ark_bn254_fq12_add: InternalGas, { 12.. => "algebra.ark_bn254_fq12_add" }, 4_406],
        [algebra_ark_bn254_fq12_clone: InternalGas, { 12.. => "algebra.ark_bn254_fq12_clone" }, 4_392],
        [algebra_ark_bn254_fq12_deser: InternalGas, { 12.. => "algebra.ark_bn254_fq12_deser" }, 129_063],
        [algebra_ark_bn254_fq12_div: InternalGas, { 12.. => "algebra.ark_bn254_fq12_div" }, 2_813_602],
        [algebra_ark_bn254_fq12_eq: InternalGas, { 12.. => "algebra.ark_bn254_fq12_eq" }, 12_142],
        [algebra_ark_bn254_fq12_from_u64: InternalGas, { 12.. => "algebra.ark_bn254_fq12_from_u64" }, 14_463],
        [algebra_ark_bn254_fq12_inv: InternalGas, { 12.. => "algebra.ark_bn254_fq12_inv" }, 2_168_418],
        [algebra_ark_bn254_fq12_mul: InternalGas, { 12.. => "algebra.ark_bn254_fq12_mul" }, 643_914],
        [algebra_ark_bn254_fq12_neg: InternalGas, { 12.. => "algebra.ark_bn254_fq12_neg" }, 13_311],
        [algebra_ark_bn254_fq12_one: InternalGas, { 12.. => "algebra.ark_bn254_fq12_one" }, 209],
        [algebra_ark_bn254_fq12_pow_u256: InternalGas, { 12.. => "algebra.ark_bn254_fq12_pow_u256" }, 192_871_746],
        [algebra_ark_bn254_fq12_serialize: InternalGas, { 12.. => "algebra.ark_bn254_fq12_serialize" }, 117_336],
        [algebra_ark_bn254_fq12_square: InternalGas, { 12.. => "algebra.ark_bn254_fq12_square" }, 468_955],
        [algebra_ark_bn254_fq12_sub: InternalGas, { 12.. => "algebra.ark_bn254_fq12_sub" }, 30_497],
        [algebra_ark_bn254_fq12_zero: InternalGas, { 12.. => "algebra.ark_bn254_fq12_zero" }, 209],
        [algebra_ark_bn254_fq2_add: InternalGas, { 12.. => "algebra.ark_bn254_fq2_add" }, 4_417],
        [algebra_ark_bn254_fq2_clone: InternalGas, { 12.. => "algebra.ark_bn254_fq2_clone" }, 4_318],
        [algebra_ark_bn254_fq2_deser: InternalGas, { 12.. => "algebra.ark_bn254_fq2_deser" }, 25_524],
        [algebra_ark_bn254_fq2_div: InternalGas, { 12.. => "algebra.ark_bn254_fq2_div" }, 1_183_329],
        [algebra_ark_bn254_fq2_eq: InternalGas, { 12.. => "algebra.ark_bn254_fq2_eq" }, 4_393],
        [algebra_ark_bn254_fq2_from_u64: InternalGas, { 12.. => "algebra.ark_bn254_fq2_from_u64" }, 14_227],
        [algebra_ark_bn254_fq2_inv: InternalGas, { 12.. => "algebra.ark_bn254_fq2_inv" }, 1_161_471],
        [algebra_ark_bn254_fq2_mul: InternalGas, { 12.. => "algebra.ark_bn254_fq2_mul" }, 22_085],
        [algebra_ark_bn254_fq2_neg: InternalGas, { 12.. => "algebra.ark_bn254_fq2_neg" }, 4_319],
        [algebra_ark_bn254_fq2_one: InternalGas, { 12.. => "algebra.ark_bn254_fq2_one" }, 209],
        [algebra_ark_bn254_fq2_pow_u256: InternalGas, { 12.. => "algebra.ark_bn254_fq2_pow_u256" }, 6_265_467],
        [algebra_ark_bn254_fq2_serialize: InternalGas, { 12.. => "algebra.ark_bn254_fq2_serialize" }, 44_735],
        [algebra_ark_bn254_fq2_square: InternalGas, { 12.. => "algebra.ark_bn254_fq2_square" }, 23_962],
        [algebra_ark_bn254_fq2_sub: InternalGas, { 12.. => "algebra.ark_bn254_fq2_sub" }, 8_116],
        [algebra_ark_bn254_fq2_zero: InternalGas, { 12.. => "algebra.ark_bn254_fq2_zero" }, 209],
        [algebra_ark_bn254_fq_add: InternalGas, { 12.. => "algebra.ark_bn254_fq_add" }, 4_373],
        [algebra_ark_bn254_fq_clone: InternalGas, { 12.. => "algebra.ark_bn254_fq_clone" }, 4_313],
        [algebra_ark_bn254_fq_deser: InternalGas, { 12.. => "algebra.ark_bn254_fq_deser" }, 17_588],
        [algebra_ark_bn254_fq_div: InternalGas, { 12.. => "algebra.ark_bn254_fq_div" }, 1_140_544],
        [algebra_ark_bn254_fq_eq: InternalGas, { 12.. => "algebra.ark_bn254_fq_eq" }, 4_373],
        [algebra_ark_bn254_fq_from_u64: InternalGas, { 12.. => "algebra.ark_bn254_fq_from_u64" }, 14_137],
        [algebra_ark_bn254_fq_inv: InternalGas, { 12.. => "algebra.ark_bn254_fq_inv" }, 1_136_577],
        [algebra_ark_bn254_fq_mul: InternalGas, { 12.. => "algebra.ark_bn254_fq_mul" }, 10_050],
        [algebra_ark_bn254_fq_neg: InternalGas, { 12.. => "algebra.ark_bn254_fq_neg" }, 4_314],
        [algebra_ark_bn254_fq_one: InternalGas, { 12.. => "algebra.ark_bn254_fq_one" }, 209],
        [algebra_ark_bn254_fq_pow_u256: InternalGas, { 12.. => "algebra.ark_bn254_fq_pow_u256" }, 2_081_451],
        [algebra_ark_bn254_fq_serialize: InternalGas, { 12.. => "algebra.ark_bn254_fq_serialize" }, 25_938],
        [algebra_ark_bn254_fq_square: InternalGas, { 12.. => "algebra.ark_bn254_fq_square" }, 4_314],
        [algebra_ark_bn254_fq_sub: InternalGas, { 12.. => "algebra.ark_bn254_fq_sub" }, 6_148],
        [algebra_ark_bn254_fq_zero: InternalGas, { 12.. => "algebra.ark_bn254_fq_zero" }, 209],
        [algebra_ark_bn254_fr_add: InternalGas, { 12.. => "algebra.ark_bn254_fr_add" }, 4_377],
        [algebra_ark_bn254_fr_deser: InternalGas, { 12.. => "algebra.ark_bn254_fr_deser" }, 16_722],
        [algebra_ark_bn254_fr_div: InternalGas, { 12.. => "algebra.ark_bn254_fr_div" }, 1_217_943],
        [algebra_ark_bn254_fr_eq: InternalGas, { 12.. => "algebra.ark_bn254_fr_eq" }, 4_396],
        [algebra_ark_bn254_fr_from_u64: InternalGas, { 12.. => "algebra.ark_bn254_fr_from_u64" }, 13_485],
        [algebra_ark_bn254_fr_inv: InternalGas, { 12.. => "algebra.ark_bn254_fr_inv" }, 1_209_015],
        [algebra_ark_bn254_fr_mul: InternalGas, { 12.. => "algebra.ark_bn254_fr_mul" }, 9_867],
        [algebra_ark_bn254_fr_neg: InternalGas, { 12.. => "algebra.ark_bn254_fr_neg" }, 4_314],
        [algebra_ark_bn254_fr_one: InternalGas, { 12.. => "algebra.ark_bn254_fr_one" }, 0],
        [algebra_ark_bn254_fr_serialize: InternalGas, { 12.. => "algebra.ark_bn254_fr_serialize" }, 25_749],
        [algebra_ark_bn254_fr_square: InternalGas, { 12.. => "algebra.ark_bn254_fr_square" }, 4_311],
        [algebra_ark_bn254_fr_sub: InternalGas, { 12.. => "algebra.ark_bn254_fr_sub" }, 10_370],
        [algebra_ark_bn254_fr_zero: InternalGas, { 12.. => "algebra.ark_bn254_fr_zero" }, 209],
        [algebra_ark_bn254_g1_affine_deser_comp: InternalGas, { 12.. => "algebra.ark_bn254_g1_affine_deser_comp" }, 23_497_333],
        [algebra_ark_bn254_g1_affine_deser_uncomp: InternalGas, { 12.. => "algebra.ark_bn254_g1_affine_deser_uncomp" }, 21_528_706],
        [algebra_ark_bn254_g1_affine_serialize_comp: InternalGas, { 12.. => "algebra.ark_bn254_g1_affine_serialize_comp" }, 44_924],
        [algebra_ark_bn254_g1_affine_serialize_uncomp: InternalGas, { 12.. => "algebra.ark_bn254_g1_affine_serialize_uncomp" }, 58_820],
        [algebra_ark_bn254_g1_proj_add: InternalGas, { 12.. => "algebra.ark_bn254_g1_proj_add" }, 106_501],
        [algebra_ark_bn254_g1_proj_double: InternalGas, { 12.. => "algebra.ark_bn254_g1_proj_double" }, 63_682],
        [algebra_ark_bn254_g1_proj_eq: InternalGas, { 12.. => "algebra.ark_bn254_g1_proj_eq" }, 53_021],
        [algebra_ark_bn254_g1_proj_generator: InternalGas, { 12.. => "algebra.ark_bn254_g1_proj_generator" }, 209],
        [algebra_ark_bn254_g1_proj_infinity: InternalGas, { 12.. => "algebra.ark_bn254_g1_proj_infinity" }, 209],
        [algebra_ark_bn254_g1_proj_neg: InternalGas, { 12.. => "algebra.ark_bn254_g1_proj_neg" }, 209],
        [algebra_ark_bn254_g1_proj_scalar_mul: InternalGas, { 12.. => "algebra.ark_bn254_g1_proj_scalar_mul" }, 26_456_386],
        [algebra_ark_bn254_g1_proj_sub: InternalGas, { 12.. => "algebra.ark_bn254_g1_proj_sub" }, 106_903],
        [algebra_ark_bn254_g1_proj_to_affine: InternalGas, { 12.. => "algebra.ark_bn254_g1_proj_to_affine" }, 6_340],
        [algebra_ark_bn254_g2_affine_deser_comp: InternalGas, { 12.. => "algebra.ark_bn254_g2_affine_deser_comp" }, 67_710_223],
        [algebra_ark_bn254_g2_affine_deser_uncomp: InternalGas, { 12.. => "algebra.ark_bn254_g2_affine_deser_uncomp" }, 60_677_591],
        [algebra_ark_bn254_g2_affine_serialize_comp: InternalGas, { 12.. => "algebra.ark_bn254_g2_affine_serialize_comp" }, 69_214],
        [algebra_ark_bn254_g2_affine_serialize_uncomp: InternalGas, { 12.. => "algebra.ark_bn254_g2_affine_serialize_uncomp" }, 98_505],
        [algebra_ark_bn254_g2_proj_add: InternalGas, { 12.. => "algebra.ark_bn254_g2_proj_add" }, 318_234],
        [algebra_ark_bn254_g2_proj_double: InternalGas, { 12.. => "algebra.ark_bn254_g2_proj_double" }, 158_874],
        [algebra_ark_bn254_g2_proj_eq: InternalGas, { 12.. => "algebra.ark_bn254_g2_proj_eq" }, 141_359],
        [algebra_ark_bn254_g2_proj_generator: InternalGas, { 12.. => "algebra.ark_bn254_g2_proj_generator" }, 209],
        [algebra_ark_bn254_g2_proj_infinity: InternalGas, { 12.. => "algebra.ark_bn254_g2_proj_infinity" }, 209],
        [algebra_ark_bn254_g2_proj_neg: InternalGas, { 12.. => "algebra.ark_bn254_g2_proj_neg" }, 209],
        [algebra_ark_bn254_g2_proj_scalar_mul: InternalGas, { 12.. => "algebra.ark_bn254_g2_proj_scalar_mul" }, 76_395_801],
        [algebra_ark_bn254_g2_proj_sub: InternalGas, { 12.. => "algebra.ark_bn254_g2_proj_sub" }, 321_727],
        [algebra_ark_bn254_g2_proj_to_affine: InternalGas, { 12.. => "algebra.ark_bn254_g2_proj_to_affine" }, 1_251_909],
        [algebra_ark_bn254_multi_pairing_base: InternalGas, { 12.. => "algebra.ark_bn254_multi_pairing_base" }, 127_794_596],
        [algebra_ark_bn254_multi_pairing_per_pair: InternalGasPerArg, { 12.. => "algebra.ark_bn254_multi_pairing_per_pair" }, 67_624_587],
        [algebra_ark_bn254_pairing: InternalGas, { 12.. => "algebra.ark_bn254_pairing" }, 209_703_839],
        // BN254 algebra gas parameters end.

        // BLS12-381 algebra gas parameters begin.
        // Generated at time 1680606720.0709136 by `scripts/algebra-gas/update_algebra_gas_params.py` with gas_per_ns=10.23.
        [algebra_ark_bls12_381_fq12_add: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_add" }, 36380],
        [algebra_ark_bls12_381_fq12_clone: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_clone" }, 4220],
        [algebra_ark_bls12_381_fq12_deser: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_deser" }, 223600],
        [algebra_ark_bls12_381_fq12_div: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_div" }, 5016260],
        [algebra_ark_bls12_381_fq12_eq: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_eq" }, 14520],
        [algebra_ark_bls12_381_fq12_from_u64: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_from_u64" }, 18020],
        [algebra_ark_bls12_381_fq12_inv: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_inv" }, 4010460],
        [algebra_ark_bls12_381_fq12_mul: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_mul" }, 997720],
        [algebra_ark_bls12_381_fq12_neg: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_neg" }, 23620],
        [algebra_ark_bls12_381_fq12_one: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_one" }, 220],
        [algebra_ark_bls12_381_fq12_pow_u256: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_pow_u256" }, 293284140],
        [algebra_ark_bls12_381_fq12_serialize: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_serialize" }, 161560],
        [algebra_ark_bls12_381_fq12_square: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_square" }, 702900],
        [algebra_ark_bls12_381_fq12_sub: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_sub" }, 35160],
        [algebra_ark_bls12_381_fq12_zero: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_zero" }, 4220],
        [algebra_ark_bls12_381_fr_add: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_add" }, 4220],
        [algebra_ark_bls12_381_fr_deser: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_deser" }, 15040],
        [algebra_ark_bls12_381_fr_div: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_div" }, 1188800],
        [algebra_ark_bls12_381_fr_eq: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_eq" }, 4240],
        [algebra_ark_bls12_381_fr_from_u64: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_from_u64" }, 9880],
        [algebra_ark_bls12_381_fr_inv: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_inv" }, 1172200],
        [algebra_ark_bls12_381_fr_mul: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_mul" }, 10040],
        [algebra_ark_bls12_381_fr_neg: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_neg" }, 4260],
        [algebra_ark_bls12_381_fr_one: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_one" }, 4220],
        [algebra_ark_bls12_381_fr_serialize: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_serialize" }, 22060],
        [algebra_ark_bls12_381_fr_square: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_square" }, 9500],
        [algebra_ark_bls12_381_fr_sub: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_sub" }, 5800],
        [algebra_ark_bls12_381_fr_zero: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_zero" }, 4220],
        [algebra_ark_bls12_381_g1_affine_deser_comp: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_affine_deser_comp" }, 20591980],
        [algebra_ark_bls12_381_g1_affine_deser_uncomp: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_affine_deser_uncomp" }, 14412760],
        [algebra_ark_bls12_381_g1_affine_serialize_comp: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_affine_serialize_comp" }, 40280],
        [algebra_ark_bls12_381_g1_affine_serialize_uncomp: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_affine_serialize_uncomp" }, 48660],
        [algebra_ark_bls12_381_g1_proj_add: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_proj_add" }, 216120],
        [algebra_ark_bls12_381_g1_proj_double: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_proj_double" }, 105280],
        [algebra_ark_bls12_381_g1_proj_eq: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_proj_eq" }, 100700],
        [algebra_ark_bls12_381_g1_proj_generator: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_proj_generator" }, 220],
        [algebra_ark_bls12_381_g1_proj_infinity: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_proj_infinity" }, 220],
        [algebra_ark_bls12_381_g1_proj_neg: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_proj_neg" }, 220],
        [algebra_ark_bls12_381_g1_proj_scalar_mul: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_proj_scalar_mul" }, 50470420],
        [algebra_ark_bls12_381_g1_proj_sub: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_proj_sub" }, 222940],
        [algebra_ark_bls12_381_g1_proj_to_affine: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_proj_to_affine" }, 2420700],
        [algebra_ark_bls12_381_g2_affine_deser_comp: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_affine_deser_comp" }, 41201360],
        [algebra_ark_bls12_381_g2_affine_deser_uncomp: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_affine_deser_uncomp" }, 20359580],
        [algebra_ark_bls12_381_g2_affine_serialize_comp: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_affine_serialize_comp" }, 67560],
        [algebra_ark_bls12_381_g2_affine_serialize_uncomp: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_affine_serialize_uncomp" }, 84340],
        [algebra_ark_bls12_381_g2_proj_add: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_proj_add" }, 648020],
        [algebra_ark_bls12_381_g2_proj_double: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_proj_double" }, 296780],
        [algebra_ark_bls12_381_g2_proj_eq: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_proj_eq" }, 303100],
        [algebra_ark_bls12_381_g2_proj_generator: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_proj_generator" }, 220],
        [algebra_ark_bls12_381_g2_proj_infinity: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_proj_infinity" }, 220],
        [algebra_ark_bls12_381_g2_proj_neg: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_proj_neg" }, 220],
        [algebra_ark_bls12_381_g2_proj_scalar_mul: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_proj_scalar_mul" }, 150530160],
        [algebra_ark_bls12_381_g2_proj_sub: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_proj_sub" }, 657380],
        [algebra_ark_bls12_381_g2_proj_to_affine: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_proj_to_affine" }, 2577140],
        [algebra_ark_bls12_381_multi_pairing_base: InternalGas, { 8.. => "algebra.ark_bls12_381_multi_pairing_base" }, 179972980],
        [algebra_ark_bls12_381_multi_pairing_per_pair: InternalGasPerArg, { 8.. => "algebra.ark_bls12_381_multi_pairing_per_pair" }, 92052840],
        [algebra_ark_bls12_381_pairing: InternalGas, { 8.. => "algebra.ark_bls12_381_pairing" }, 296644400],
        [algebra_ark_h2c_bls12381g1_xmd_sha256_sswu_base: InternalGas, { 8.. => "algebra.ark_h2c_bls12381g1_xmd_sha256_sswu_base" }, 65038860],
        [algebra_ark_h2c_bls12381g1_xmd_sha256_sswu_per_msg_byte: InternalGasPerByte, { 8.. => "algebra.ark_h2c_bls12381g1_xmd_sha256_sswu_per_msg_byte" }, 960],
        [algebra_ark_h2c_bls12381g2_xmd_sha256_sswu_base: InternalGas, { 8.. => "algebra.ark_h2c_bls12381g2_xmd_sha256_sswu_base" }, 135460040],
        [algebra_ark_h2c_bls12381g2_xmd_sha256_sswu_per_msg_byte: InternalGasPerByte, { 8.. => "algebra.ark_h2c_bls12381g2_xmd_sha256_sswu_per_msg_byte" }, 960],
        // BLS12-381 algebra gas parameters end.

        [bls12381_base: InternalGas, "bls12381.base", 3000],

        [bls12381_per_pubkey_deserialize: InternalGasPerArg, "bls12381.per_pubkey_deserialize", 2180000],
        [bls12381_per_pubkey_aggregate: InternalGasPerArg, "bls12381.per_pubkey_aggregate", 84000],
        [bls12381_per_pubkey_subgroup_check: InternalGasPerArg, "bls12381.per_pubkey_subgroup_check", 7400000],

        [bls12381_per_sig_deserialize: InternalGasPerArg, "bls12381.per_sig_deserialize", 4440000],
        [bls12381_per_sig_aggregate: InternalGasPerArg, "bls12381.per_sig_aggregate", 233000],
        [bls12381_per_sig_subgroup_check: InternalGasPerArg, "bls12381.per_sig_subgroup_check", 9210000],

        [bls12381_per_sig_verify: InternalGasPerArg, "bls12381.per_sig_verify", 169700000],
        [bls12381_per_pop_verify: InternalGasPerArg, "bls12381.per_pop_verify", 206000000],

        [bls12381_per_pairing: InternalGasPerArg, "bls12381.per_pairing", 80260000],

        [bls12381_per_msg_hashing: InternalGasPerArg, "bls12381.per_msg_hashing", 30800000],
        [bls12381_per_byte_hashing: InternalGasPerByte, "bls12381.per_byte_hashing", 1000],

        [ed25519_base: InternalGas, "signature.base", 3000],
        [ed25519_per_pubkey_deserialize: InternalGasPerArg, "signature.per_pubkey_deserialize", 760000],
        [ed25519_per_pubkey_small_order_check: InternalGasPerArg, "signature.per_pubkey_small_order_check", 127000],
        [ed25519_per_sig_deserialize: InternalGasPerArg, "signature.per_sig_deserialize", 7500],
        [ed25519_per_sig_strict_verify: InternalGasPerArg, "signature.per_sig_strict_verify", 5340000],
        [ed25519_per_msg_hashing_base: InternalGasPerArg, "signature.per_msg_hashing_base", 64800],
        [ed25519_per_msg_byte_hashing: InternalGasPerByte, "signature.per_msg_byte_hashing", 1200],

        [secp256k1_base: InternalGas, "secp256k1.base", 3000],
        [secp256k1_ecdsa_recover: InternalGasPerArg, "secp256k1.ecdsa_recover", 32200000],

        [ristretto255_basepoint_mul: InternalGasPerArg, "ristretto255.basepoint_mul", 2560000],
        [ristretto255_basepoint_double_mul: InternalGasPerArg, "ristretto255.basepoint_double_mul", 8800000],

        [ristretto255_point_add: InternalGasPerArg, "ristretto255.point_add", 42700],
        [ristretto255_point_clone: InternalGasPerArg, { 11.. => "ristretto255.point_clone" }, 3000],
        [ristretto255_point_compress: InternalGasPerArg, "ristretto255.point_compress", 800000],
        [ristretto255_point_decompress: InternalGasPerArg, "ristretto255.point_decompress", 810000],
        [ristretto255_point_equals: InternalGasPerArg, "ristretto255.point_equals", 46000],
        [ristretto255_point_from_64_uniform_bytes: InternalGasPerArg, "ristretto255.point_from_64_uniform_bytes", 1630000],
        [ristretto255_point_identity: InternalGasPerArg, "ristretto255.point_identity", 3000],
        [ristretto255_point_mul: InternalGasPerArg, "ristretto255.point_mul", 9420000],
        [ristretto255_point_double_mul: InternalGasPerArg, { 11.. => "ristretto255.point_double_mul" }, 10173600],
        [ristretto255_point_neg: InternalGasPerArg, "ristretto255.point_neg", 7200],
        [ristretto255_point_sub: InternalGasPerArg, "ristretto255.point_sub", 42600],
        [ristretto255_point_parse_arg: InternalGasPerArg, "ristretto255.point_parse_arg", 3000],


        // TODO(Alin): These SHA512 gas costs could be unified with the costs in our future SHA512 module
        // (assuming same implementation complexity, which might not be the case
        [ristretto255_sha512_per_byte: InternalGasPerByte, "ristretto255.scalar_sha512_per_byte", 1200],
        [ristretto255_sha512_per_hash: InternalGasPerArg, "ristretto255.scalar_sha512_per_hash", 64800],

        [ristretto255_scalar_add: InternalGasPerArg, "ristretto255.scalar_add", 15400],
        [ristretto255_scalar_reduced_from_32_bytes: InternalGasPerArg, "ristretto255.scalar_reduced_from_32_bytes", 14200],
        [ristretto255_scalar_uniform_from_64_bytes: InternalGasPerArg, "ristretto255.scalar_uniform_from_64_bytes", 24900],
        [ristretto255_scalar_from_u128: InternalGasPerArg, "ristretto255.scalar_from_u128", 3500],
        [ristretto255_scalar_from_u64: InternalGasPerArg, "ristretto255.scalar_from_u64", 3500],
        [ristretto255_scalar_invert: InternalGasPerArg, "ristretto255.scalar_invert", 2200000],
        [ristretto255_scalar_is_canonical: InternalGasPerArg, "ristretto255.scalar_is_canonical", 23000],
        [ristretto255_scalar_mul: InternalGasPerArg, "ristretto255.scalar_mul", 21300],
        [ristretto255_scalar_neg: InternalGasPerArg, "ristretto255.scalar_neg", 14500],
        [ristretto255_scalar_sub: InternalGasPerArg, "ristretto255.scalar_sub", 21200],
        [ristretto255_scalar_parse_arg: InternalGasPerArg, "ristretto255.scalar_parse_arg", 3000],

        [hash_sip_hash_base: InternalGas, "hash.sip_hash.base", 20000],
        [hash_sip_hash_per_byte: InternalGasPerByte, "hash.sip_hash.per_byte", 400],

        [hash_keccak256_base: InternalGas, { 1.. => "hash.keccak256.base" }, 80000],
        [hash_keccak256_per_byte: InternalGasPerByte, { 1.. => "hash.keccak256.per_byte" }, 900],

        // Bulletproofs gas parameters begin.
        // Generated at time 1683148919.0628748 by `scripts/algebra-gas/update_bulletproofs_gas_params.py` with gas_per_ns=10.0.
        [bulletproofs_base: InternalGas, { 11.. => "bulletproofs.base" }, 64171120],
        [bulletproofs_per_bit_rangeproof_verify: InternalGasPerArg, { 11.. => "bulletproofs.per_bit_rangeproof_verify" }, 5463840],
        [bulletproofs_per_byte_rangeproof_deserialize: InternalGasPerByte, { 11.. => "bulletproofs.per_byte_rangeproof_deserialize" }, 660],
        // Bulletproofs gas parameters end.

        [type_info_type_of_base: InternalGas, "type_info.type_of.base", 6000],
        // TODO(Gas): the on-chain name is wrong...
        [type_info_type_of_per_byte_in_str: InternalGasPerByte, "type_info.type_of.per_abstract_memory_unit", 100],
        [type_info_type_name_base: InternalGas, "type_info.type_name.base", 6000],
        // TODO(Gas): the on-chain name is wrong...
        [type_info_type_name_per_byte_in_str: InternalGasPerByte, "type_info.type_name.per_abstract_memory_unit", 100],
        [type_info_chain_id_base: InternalGas, { 4.. => "type_info.chain_id.base" }, 3000],

        // Reusing SHA2-512's cost from Ristretto
        [hash_sha2_512_base: InternalGas, { 4.. => "hash.sha2_512.base" }, 64_800],  // 3_240 * 20
        [hash_sha2_512_per_byte: InternalGasPerByte, { 4.. => "hash.sha2_512.per_byte" }, 1_200], // 60 * 20
        // Back-of-the-envelope approximation from SHA3-256's costs (4000 base, 45 per-byte)
        [hash_sha3_512_base: InternalGas, { 4.. => "hash.sha3_512.base" }, 90_000], // 4_500 * 20
        [hash_sha3_512_per_byte: InternalGasPerByte, { 4.. => "hash.sha3_512.per_byte" }, 1_000], // 50 * 20
        // Using SHA2-256's cost
        [hash_ripemd160_base: InternalGas, { 4.. => "hash.ripemd160.base" }, 60_000], // 3000 * 20
        [hash_ripemd160_per_byte: InternalGasPerByte, { 4.. => "hash.ripemd160.per_byte" }, 1_000], // 50 * 20
        [hash_blake2b_256_base: InternalGas, { 6.. => "hash.blake2b_256.base" }, 35_000], // 1750 * 20
        [hash_blake2b_256_per_byte: InternalGasPerByte, { 6.. => "hash.blake2b_256.per_byte" }, 300], // 15 * 20

        [util_from_bytes_base: InternalGas, "util.from_bytes.base", 6000],
        [util_from_bytes_per_byte: InternalGasPerByte, "util.from_bytes.per_byte", 100],

        [transaction_context_get_txn_hash_base: InternalGas, { 10.. => "transaction_context.get_txn_hash.base" }, 4000],
        [transaction_context_get_script_hash_base: InternalGas, "transaction_context.get_script_hash.base", 4000],
        // Based on SHA3-256's cost
        [transaction_context_generate_unique_address_base: InternalGas, { 10.. => "transaction_context.generate_unique_address.base" }, 80000],

        [code_request_publish_base: InternalGas, "code.request_publish.base", 10000],
        [code_request_publish_per_byte: InternalGasPerByte, "code.request_publish.per_byte", 40],

        // Note(Gas): These are storage operations so the values should not be multiplied.
        [event_write_to_event_store_base: InternalGas, "event.write_to_event_store.base", 300_000],
        // TODO(Gas): the on-chain name is wrong...
        [event_write_to_event_store_per_abstract_value_unit: InternalGasPerAbstractValueUnit, "event.write_to_event_store.per_abstract_memory_unit", 5_000],

        [state_storage_get_usage_base_cost: InternalGas, "state_storage.get_usage.base", 10000],

        [aggregator_add_base: InternalGas, "aggregator.add.base", 6000],
        [aggregator_read_base: InternalGas, "aggregator.read.base", 6000],
        [aggregator_sub_base: InternalGas, "aggregator.sub.base", 6000],
        [aggregator_destroy_base: InternalGas, "aggregator.destroy.base", 10000],
        [aggregator_factory_new_aggregator_base: InternalGas, "aggregator_factory.new_aggregator.base", 10000],

        [aggregator_v2_create_aggregator_base: InternalGas, {12.. => "aggregator_v2.create_aggregator.base"}, 10000],
        [aggregator_v2_try_add_base: InternalGas, {12.. => "aggregator_v2.try_add.base"}, 6000],
        [aggregator_v2_try_sub_base: InternalGas, {12.. => "aggregator_v2.try_sub.base"}, 6000],
        [aggregator_v2_read_base: InternalGas, {12.. => "aggregator_v2.read.base"}, 12000],
        [aggregator_v2_snapshot_base: InternalGas, {12.. => "aggregator_v2.snapshot.base"}, 6000],

        [aggregator_v2_create_snapshot_base: InternalGas, {11.. => "aggregator_v2.create_snapshot.base"}, 6000],
        [aggregator_v2_create_snapshot_per_byte: InternalGasPerByte, { 12.. =>"aggregator_v2.create_snapshot.per_byte" }, 20],
        [aggregator_v2_copy_snapshot_base: InternalGas, {11.. => "aggregator_v2.copy_snapshot.base"}, 6000],
        [aggregator_v2_read_snapshot_base: InternalGas, {11.. => "aggregator_v2.read_snapshot.base"}, 12000],
        [aggregator_v2_string_concat_base: InternalGas, {11.. => "aggregator_v2.string_concat.base"}, 6000],
        [aggregator_v2_string_concat_per_byte: InternalGasPerByte, { 12.. =>"aggregator_v2.string_concat.per_byte" }, 20],

        [object_exists_at_base: InternalGas, { 7.. => "object.exists_at.base" }, 5000],
        // These are dummy value, they copied from storage gas in aptos-core/aptos-vm/src/aptos_vm_impl.rs
        [object_exists_at_per_byte_loaded: InternalGasPerByte, { 7.. => "object.exists_at.per_byte_loaded" }, 1000],
        [object_exists_at_per_item_loaded: InternalGas, { 7.. => "object.exists_at.per_item_loaded" }, 8000],
        [string_utils_base: InternalGas, { 8.. => "string_utils.format.base" }, 6000],
        [string_utils_per_byte: InternalGasPerByte, { 8.. =>"string_utils.format.per_byte" }, 20],
    ]
);
