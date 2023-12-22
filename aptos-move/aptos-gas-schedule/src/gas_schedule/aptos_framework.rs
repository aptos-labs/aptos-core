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
        [account_create_address_base: InternalGas, "account.create_address.base", 1254],
        [account_create_signer_base: InternalGas, "account.create_signer.base", 1254],

        // BN254 algebra gas parameters begin.
        // Generated at time 1701559125.5498126 by `scripts/algebra-gas/update_bn254_algebra_gas_params.py` with gas_per_ns=209.10511688369482.
        [algebra_ark_bn254_fq12_add: InternalGas, { 12.. => "algebra.ark_bn254_fq12_add" }, 920],
        [algebra_ark_bn254_fq12_clone: InternalGas, { 12.. => "algebra.ark_bn254_fq12_clone" }, 917],
        [algebra_ark_bn254_fq12_deser: InternalGas, { 12.. => "algebra.ark_bn254_fq12_deser" }, 26974],
        [algebra_ark_bn254_fq12_div: InternalGas, { 12.. => "algebra.ark_bn254_fq12_div" }, 588042],
        [algebra_ark_bn254_fq12_eq: InternalGas, { 12.. => "algebra.ark_bn254_fq12_eq" }, 2537],
        [algebra_ark_bn254_fq12_from_u64: InternalGas, { 12.. => "algebra.ark_bn254_fq12_from_u64" }, 3022],
        [algebra_ark_bn254_fq12_inv: InternalGas, { 12.. => "algebra.ark_bn254_fq12_inv" }, 453199],
        [algebra_ark_bn254_fq12_mul: InternalGas, { 12.. => "algebra.ark_bn254_fq12_mul" }, 134578],
        [algebra_ark_bn254_fq12_neg: InternalGas, { 12.. => "algebra.ark_bn254_fq12_neg" }, 2781],
        [algebra_ark_bn254_fq12_one: InternalGas, { 12.. => "algebra.ark_bn254_fq12_one" }, 43],
        [algebra_ark_bn254_fq12_pow_u256: InternalGas, { 12.. => "algebra.ark_bn254_fq12_pow_u256" }, 40310194],
        [algebra_ark_bn254_fq12_serialize: InternalGas, { 12.. => "algebra.ark_bn254_fq12_serialize" }, 24523],
        [algebra_ark_bn254_fq12_square: InternalGas, { 12.. => "algebra.ark_bn254_fq12_square" }, 98011],
        [algebra_ark_bn254_fq12_sub: InternalGas, { 12.. => "algebra.ark_bn254_fq12_sub" }, 6373],
        [algebra_ark_bn254_fq12_zero: InternalGas, { 12.. => "algebra.ark_bn254_fq12_zero" }, 43],
        [algebra_ark_bn254_fq_add: InternalGas, { 12.. => "algebra.ark_bn254_fq_add" }, 913],
        [algebra_ark_bn254_fq_clone: InternalGas, { 12.. => "algebra.ark_bn254_fq_clone" }, 901],
        [algebra_ark_bn254_fq_deser: InternalGas, { 12.. => "algebra.ark_bn254_fq_deser" }, 3675],
        [algebra_ark_bn254_fq_div: InternalGas, { 12.. => "algebra.ark_bn254_fq_div" }, 238373],
        [algebra_ark_bn254_fq_eq: InternalGas, { 12.. => "algebra.ark_bn254_fq_eq" }, 913],
        [algebra_ark_bn254_fq_from_u64: InternalGas, { 12.. => "algebra.ark_bn254_fq_from_u64" }, 2954],
        [algebra_ark_bn254_fq_inv: InternalGas, { 12.. => "algebra.ark_bn254_fq_inv" }, 237544],
        [algebra_ark_bn254_fq_mul: InternalGas, { 12.. => "algebra.ark_bn254_fq_mul" }, 2100],
        [algebra_ark_bn254_fq_neg: InternalGas, { 12.. => "algebra.ark_bn254_fq_neg" }, 901],
        [algebra_ark_bn254_fq_one: InternalGas, { 12.. => "algebra.ark_bn254_fq_one" }, 43],
        [algebra_ark_bn254_fq_pow_u256: InternalGas, { 12.. => "algebra.ark_bn254_fq_pow_u256" }, 435023],
        [algebra_ark_bn254_fq_serialize: InternalGas, { 12.. => "algebra.ark_bn254_fq_serialize" }, 5421],
        [algebra_ark_bn254_fq_square: InternalGas, { 12.. => "algebra.ark_bn254_fq_square" }, 901],
        [algebra_ark_bn254_fq_sub: InternalGas, { 12.. => "algebra.ark_bn254_fq_sub" }, 1284],
        [algebra_ark_bn254_fq_zero: InternalGas, { 12.. => "algebra.ark_bn254_fq_zero" }, 43],
        [algebra_ark_bn254_fr_add: InternalGas, { 12.. => "algebra.ark_bn254_fr_add" }, 914],
        [algebra_ark_bn254_fr_deser: InternalGas, { 12.. => "algebra.ark_bn254_fr_deser" }, 3494],
        [algebra_ark_bn254_fr_div: InternalGas, { 12.. => "algebra.ark_bn254_fr_div" }, 254550],
        [algebra_ark_bn254_fr_eq: InternalGas, { 12.. => "algebra.ark_bn254_fr_eq" }, 918],
        [algebra_ark_bn254_fr_from_u64: InternalGas, { 12.. => "algebra.ark_bn254_fr_from_u64" }, 2818],
        [algebra_ark_bn254_fr_inv: InternalGas, { 12.. => "algebra.ark_bn254_fr_inv" }, 252684],
        [algebra_ark_bn254_fr_mul: InternalGas, { 12.. => "algebra.ark_bn254_fr_mul" }, 2062],
        [algebra_ark_bn254_fr_neg: InternalGas, { 12.. => "algebra.ark_bn254_fr_neg" }, 901],
        [algebra_ark_bn254_fr_one: InternalGas, { 12.. => "algebra.ark_bn254_fr_one" }, 0],
        [algebra_ark_bn254_fr_serialize: InternalGas, { 12.. => "algebra.ark_bn254_fr_serialize" }, 5381],
        [algebra_ark_bn254_fr_square: InternalGas, { 12.. => "algebra.ark_bn254_fr_square" }, 900],
        [algebra_ark_bn254_fr_sub: InternalGas, { 12.. => "algebra.ark_bn254_fr_sub" }, 2167],
        [algebra_ark_bn254_fr_zero: InternalGas, { 12.. => "algebra.ark_bn254_fr_zero" }, 43],
        [algebra_ark_bn254_g1_affine_deser_comp: InternalGas, { 12.. => "algebra.ark_bn254_g1_affine_deser_comp" }, 4910942],
        [algebra_ark_bn254_g1_affine_deser_uncomp: InternalGas, { 12.. => "algebra.ark_bn254_g1_affine_deser_uncomp" }, 4499499],
        [algebra_ark_bn254_g1_affine_serialize_comp: InternalGas, { 12.. => "algebra.ark_bn254_g1_affine_serialize_comp" }, 9389],
        [algebra_ark_bn254_g1_affine_serialize_uncomp: InternalGas, { 12.. => "algebra.ark_bn254_g1_affine_serialize_uncomp" }, 12293],
        [algebra_ark_bn254_g1_proj_add: InternalGas, { 12.. => "algebra.ark_bn254_g1_proj_add" }, 22258],
        [algebra_ark_bn254_g1_proj_double: InternalGas, { 12.. => "algebra.ark_bn254_g1_proj_double" }, 13309],
        [algebra_ark_bn254_g1_proj_eq: InternalGas, { 12.. => "algebra.ark_bn254_g1_proj_eq" }, 11081],
        [algebra_ark_bn254_g1_proj_generator: InternalGas, { 12.. => "algebra.ark_bn254_g1_proj_generator" }, 43],
        [algebra_ark_bn254_g1_proj_infinity: InternalGas, { 12.. => "algebra.ark_bn254_g1_proj_infinity" }, 43],
        [algebra_ark_bn254_g1_proj_neg: InternalGas, { 12.. => "algebra.ark_bn254_g1_proj_neg" }, 43],
        [algebra_ark_bn254_g1_proj_scalar_mul: InternalGas, { 12.. => "algebra.ark_bn254_g1_proj_scalar_mul" }, 5529384],
        [algebra_ark_bn254_g1_proj_sub: InternalGas, { 12.. => "algebra.ark_bn254_g1_proj_sub" }, 22342],
        [algebra_ark_bn254_g1_proj_to_affine: InternalGas, { 12.. => "algebra.ark_bn254_g1_proj_to_affine" }, 1325],
        [algebra_ark_bn254_g2_affine_deser_comp: InternalGas, { 12.. => "algebra.ark_bn254_g2_affine_deser_comp" }, 14151436],
        [algebra_ark_bn254_g2_affine_deser_uncomp: InternalGas, { 12.. => "algebra.ark_bn254_g2_affine_deser_uncomp" }, 12681616],
        [algebra_ark_bn254_g2_affine_serialize_comp: InternalGas, { 12.. => "algebra.ark_bn254_g2_affine_serialize_comp" }, 14465],
        [algebra_ark_bn254_g2_affine_serialize_uncomp: InternalGas, { 12.. => "algebra.ark_bn254_g2_affine_serialize_uncomp" }, 20587],
        [algebra_ark_bn254_g2_proj_add: InternalGas, { 12.. => "algebra.ark_bn254_g2_proj_add" }, 66510],
        [algebra_ark_bn254_g2_proj_double: InternalGas, { 12.. => "algebra.ark_bn254_g2_proj_double" }, 33204],
        [algebra_ark_bn254_g2_proj_eq: InternalGas, { 12.. => "algebra.ark_bn254_g2_proj_eq" }, 29544],
        [algebra_ark_bn254_g2_proj_generator: InternalGas, { 12.. => "algebra.ark_bn254_g2_proj_generator" }, 43],
        [algebra_ark_bn254_g2_proj_infinity: InternalGas, { 12.. => "algebra.ark_bn254_g2_proj_infinity" }, 43],
        [algebra_ark_bn254_g2_proj_neg: InternalGas, { 12.. => "algebra.ark_bn254_g2_proj_neg" }, 43],
        [algebra_ark_bn254_g2_proj_scalar_mul: InternalGas, { 12.. => "algebra.ark_bn254_g2_proj_scalar_mul" }, 15966722],
        [algebra_ark_bn254_g2_proj_sub: InternalGas, { 12.. => "algebra.ark_bn254_g2_proj_sub" }, 67240],
        [algebra_ark_bn254_g2_proj_to_affine: InternalGas, { 12.. => "algebra.ark_bn254_g2_proj_to_affine" }, 261648],
        [algebra_ark_bn254_multi_pairing_base: InternalGas, { 12.. => "algebra.ark_bn254_multi_pairing_base" }, 26709070],
        [algebra_ark_bn254_multi_pairing_per_pair: InternalGasPerArg, { 12.. => "algebra.ark_bn254_multi_pairing_per_pair" }, 14133538],
        [algebra_ark_bn254_pairing: InternalGas, { 12.. => "algebra.ark_bn254_pairing" }, 43828102],
        // BN254 algebra gas parameters end.

        // BLS12-381 algebra gas parameters begin.
        // Generated at time 1680606720.0709136 by `scripts/algebra-gas/update_algebra_gas_params.py` with gas_per_ns=204.6.
        [algebra_ark_bls12_381_fq12_add: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_add" }, 7603],
        [algebra_ark_bls12_381_fq12_clone: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_clone" }, 881],
        [algebra_ark_bls12_381_fq12_deser: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_deser" }, 46732],
        [algebra_ark_bls12_381_fq12_div: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_div" }, 1048398],
        [algebra_ark_bls12_381_fq12_eq: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_eq" }, 3034],
        [algebra_ark_bls12_381_fq12_from_u64: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_from_u64" }, 3766],
        [algebra_ark_bls12_381_fq12_inv: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_inv" }, 838186],
        [algebra_ark_bls12_381_fq12_mul: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_mul" }, 208523],
        [algebra_ark_bls12_381_fq12_neg: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_neg" }, 4936],
        [algebra_ark_bls12_381_fq12_one: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_one" }, 45],
        [algebra_ark_bls12_381_fq12_pow_u256: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_pow_u256" }, 61296385],
        [algebra_ark_bls12_381_fq12_serialize: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_serialize" }, 33766],
        [algebra_ark_bls12_381_fq12_square: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_square" }, 146906],
        [algebra_ark_bls12_381_fq12_sub: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_sub" }, 7348],
        [algebra_ark_bls12_381_fq12_zero: InternalGas, { 8.. => "algebra.ark_bls12_381_fq12_zero" }, 881],
        [algebra_ark_bls12_381_fr_add: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_add" }, 881],
        [algebra_ark_bls12_381_fr_deser: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_deser" }, 3143],
        [algebra_ark_bls12_381_fr_div: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_div" }, 248459],
        [algebra_ark_bls12_381_fr_eq: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_eq" }, 886],
        [algebra_ark_bls12_381_fr_from_u64: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_from_u64" }, 2064],
        [algebra_ark_bls12_381_fr_inv: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_inv" }, 244989],
        [algebra_ark_bls12_381_fr_mul: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_mul" }, 2098],
        [algebra_ark_bls12_381_fr_neg: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_neg" }, 890],
        [algebra_ark_bls12_381_fr_one: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_one" }, 881],
        [algebra_ark_bls12_381_fr_serialize: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_serialize" }, 4610],
        [algebra_ark_bls12_381_fr_square: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_square" }, 1985],
        [algebra_ark_bls12_381_fr_sub: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_sub" }, 1212],
        [algebra_ark_bls12_381_fr_zero: InternalGas, { 8.. => "algebra.ark_bls12_381_fr_zero" }, 881],
        [algebra_ark_bls12_381_g1_affine_deser_comp: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_affine_deser_comp" }, 4303723],
        [algebra_ark_bls12_381_g1_affine_deser_uncomp: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_affine_deser_uncomp" }, 3012266],
        [algebra_ark_bls12_381_g1_affine_serialize_comp: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_affine_serialize_comp" }, 8418],
        [algebra_ark_bls12_381_g1_affine_serialize_uncomp: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_affine_serialize_uncomp" }, 10169],
        [algebra_ark_bls12_381_g1_proj_add: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_proj_add" }, 45169],
        [algebra_ark_bls12_381_g1_proj_double: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_proj_double" }, 22003],
        [algebra_ark_bls12_381_g1_proj_eq: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_proj_eq" }, 21046],
        [algebra_ark_bls12_381_g1_proj_generator: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_proj_generator" }, 45],
        [algebra_ark_bls12_381_g1_proj_infinity: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_proj_infinity" }, 45],
        [algebra_ark_bls12_381_g1_proj_neg: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_proj_neg" }, 45],
        [algebra_ark_bls12_381_g1_proj_scalar_mul: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_proj_scalar_mul" }, 10548317],
        [algebra_ark_bls12_381_g1_proj_sub: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_proj_sub" }, 46594],
        [algebra_ark_bls12_381_g1_proj_to_affine: InternalGas, { 8.. => "algebra.ark_bls12_381_g1_proj_to_affine" }, 505926],
        [algebra_ark_bls12_381_g2_affine_deser_comp: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_affine_deser_comp" }, 8611084],
        [algebra_ark_bls12_381_g2_affine_deser_uncomp: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_affine_deser_uncomp" }, 4255152],
        [algebra_ark_bls12_381_g2_affine_serialize_comp: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_affine_serialize_comp" }, 14120],
        [algebra_ark_bls12_381_g2_affine_serialize_uncomp: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_affine_serialize_uncomp" }, 17627],
        [algebra_ark_bls12_381_g2_proj_add: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_proj_add" }, 135436],
        [algebra_ark_bls12_381_g2_proj_double: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_proj_double" }, 62027],
        [algebra_ark_bls12_381_g2_proj_eq: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_proj_eq" }, 63347],
        [algebra_ark_bls12_381_g2_proj_generator: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_proj_generator" }, 45],
        [algebra_ark_bls12_381_g2_proj_infinity: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_proj_infinity" }, 45],
        [algebra_ark_bls12_381_g2_proj_neg: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_proj_neg" }, 45],
        [algebra_ark_bls12_381_g2_proj_scalar_mul: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_proj_scalar_mul" }, 31460803],
        [algebra_ark_bls12_381_g2_proj_sub: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_proj_sub" }, 137392],
        [algebra_ark_bls12_381_g2_proj_to_affine: InternalGas, { 8.. => "algebra.ark_bls12_381_g2_proj_to_affine" }, 538622],
        [algebra_ark_bls12_381_multi_pairing_base: InternalGas, { 8.. => "algebra.ark_bls12_381_multi_pairing_base" }, 37614352],
        [algebra_ark_bls12_381_multi_pairing_per_pair: InternalGasPerArg, { 8.. => "algebra.ark_bls12_381_multi_pairing_per_pair" }, 19239043],
        [algebra_ark_bls12_381_pairing: InternalGas, { 8.. => "algebra.ark_bls12_381_pairing" }, 61998679],
        [algebra_ark_h2c_bls12381g1_xmd_sha256_sswu_base: InternalGas, { 8.. => "algebra.ark_h2c_bls12381g1_xmd_sha256_sswu_base" }, 13593121],
        [algebra_ark_h2c_bls12381g1_xmd_sha256_sswu_per_msg_byte: InternalGasPerByte, { 8.. => "algebra.ark_h2c_bls12381g1_xmd_sha256_sswu_per_msg_byte" }, 200],
        [algebra_ark_h2c_bls12381g2_xmd_sha256_sswu_base: InternalGas, { 8.. => "algebra.ark_h2c_bls12381g2_xmd_sha256_sswu_base" }, 28311148],
        [algebra_ark_h2c_bls12381g2_xmd_sha256_sswu_per_msg_byte: InternalGasPerByte, { 8.. => "algebra.ark_h2c_bls12381g2_xmd_sha256_sswu_per_msg_byte" }, 200],
        // BLS12-381 algebra gas parameters end.

        [bls12381_base: InternalGas, "bls12381.base", 627],

        [bls12381_per_pubkey_deserialize: InternalGasPerArg, "bls12381.per_pubkey_deserialize", 455620],
        [bls12381_per_pubkey_aggregate: InternalGasPerArg, "bls12381.per_pubkey_aggregate", 17556],
        [bls12381_per_pubkey_subgroup_check: InternalGasPerArg, "bls12381.per_pubkey_subgroup_check", 1546600],

        [bls12381_per_sig_deserialize: InternalGasPerArg, "bls12381.per_sig_deserialize", 927960],
        [bls12381_per_sig_aggregate: InternalGasPerArg, "bls12381.per_sig_aggregate", 48697],
        [bls12381_per_sig_subgroup_check: InternalGasPerArg, "bls12381.per_sig_subgroup_check", 1924890],

        [bls12381_per_sig_verify: InternalGasPerArg, "bls12381.per_sig_verify", 35467300],
        [bls12381_per_pop_verify: InternalGasPerArg, "bls12381.per_pop_verify", 43054000],

        [bls12381_per_pairing: InternalGasPerArg, "bls12381.per_pairing", 16774340],

        [bls12381_per_msg_hashing: InternalGasPerArg, "bls12381.per_msg_hashing", 6437200],
        [bls12381_per_byte_hashing: InternalGasPerByte, "bls12381.per_byte_hashing", 209],

        [ed25519_base: InternalGas, "signature.base", 627],
        [ed25519_per_pubkey_deserialize: InternalGasPerArg, "signature.per_pubkey_deserialize", 158840],
        [ed25519_per_pubkey_small_order_check: InternalGasPerArg, "signature.per_pubkey_small_order_check", 26543],
        [ed25519_per_sig_deserialize: InternalGasPerArg, "signature.per_sig_deserialize", 1567],
        [ed25519_per_sig_strict_verify: InternalGasPerArg, "signature.per_sig_strict_verify", 1116060],
        [ed25519_per_msg_hashing_base: InternalGasPerArg, "signature.per_msg_hashing_base", 13543],
        [ed25519_per_msg_byte_hashing: InternalGasPerByte, "signature.per_msg_byte_hashing", 250],

        [secp256k1_base: InternalGas, "secp256k1.base", 627],
        [secp256k1_ecdsa_recover: InternalGasPerArg, "secp256k1.ecdsa_recover", 6729800],

        [ristretto255_basepoint_mul: InternalGasPerArg, "ristretto255.basepoint_mul", 535040],
        [ristretto255_basepoint_double_mul: InternalGasPerArg, "ristretto255.basepoint_double_mul", 1839200],

        [ristretto255_point_add: InternalGasPerArg, "ristretto255.point_add", 8924],
        [ristretto255_point_clone: InternalGasPerArg, { 11.. => "ristretto255.point_clone" }, 627],
        [ristretto255_point_compress: InternalGasPerArg, "ristretto255.point_compress", 167200],
        [ristretto255_point_decompress: InternalGasPerArg, "ristretto255.point_decompress", 169290],
        [ristretto255_point_equals: InternalGasPerArg, "ristretto255.point_equals", 9614],
        [ristretto255_point_from_64_uniform_bytes: InternalGasPerArg, "ristretto255.point_from_64_uniform_bytes", 340670],
        [ristretto255_point_identity: InternalGasPerArg, "ristretto255.point_identity", 627],
        [ristretto255_point_mul: InternalGasPerArg, "ristretto255.point_mul", 1968780],
        [ristretto255_point_double_mul: InternalGasPerArg, { 11.. => "ristretto255.point_double_mul" }, 2126282],
        [ristretto255_point_neg: InternalGasPerArg, "ristretto255.point_neg", 1504],
        [ristretto255_point_sub: InternalGasPerArg, "ristretto255.point_sub", 8903],
        [ristretto255_point_parse_arg: InternalGasPerArg, "ristretto255.point_parse_arg", 627],


        // TODO(Alin): These SHA512 gas costs could be unified with the costs in our future SHA512 module
        // (assuming same implementation complexity, which might not be the case
        [ristretto255_sha512_per_byte: InternalGasPerByte, "ristretto255.scalar_sha512_per_byte", 250],
        [ristretto255_sha512_per_hash: InternalGasPerArg, "ristretto255.scalar_sha512_per_hash", 13543],

        [ristretto255_scalar_add: InternalGasPerArg, "ristretto255.scalar_add", 3218],
        [ristretto255_scalar_reduced_from_32_bytes: InternalGasPerArg, "ristretto255.scalar_reduced_from_32_bytes", 2967],
        [ristretto255_scalar_uniform_from_64_bytes: InternalGasPerArg, "ristretto255.scalar_uniform_from_64_bytes", 5204],
        [ristretto255_scalar_from_u128: InternalGasPerArg, "ristretto255.scalar_from_u128", 731],
        [ristretto255_scalar_from_u64: InternalGasPerArg, "ristretto255.scalar_from_u64", 731],
        [ristretto255_scalar_invert: InternalGasPerArg, "ristretto255.scalar_invert", 459800],
        [ristretto255_scalar_is_canonical: InternalGasPerArg, "ristretto255.scalar_is_canonical", 4807],
        [ristretto255_scalar_mul: InternalGasPerArg, "ristretto255.scalar_mul", 4451],
        [ristretto255_scalar_neg: InternalGasPerArg, "ristretto255.scalar_neg", 3030],
        [ristretto255_scalar_sub: InternalGasPerArg, "ristretto255.scalar_sub", 4430],
        [ristretto255_scalar_parse_arg: InternalGasPerArg, "ristretto255.scalar_parse_arg", 627],

        [hash_sip_hash_base: InternalGas, "hash.sip_hash.base", 4180],
        [hash_sip_hash_per_byte: InternalGasPerByte, "hash.sip_hash.per_byte", 83],

        [hash_keccak256_base: InternalGas, { 1.. => "hash.keccak256.base" }, 16720],
        [hash_keccak256_per_byte: InternalGasPerByte, { 1.. => "hash.keccak256.per_byte" }, 188],

        // Bulletproofs gas parameters begin.
        // Generated at time 1683148919.0628748 by `scripts/algebra-gas/update_bulletproofs_gas_params.py` with gas_per_ns=10.0.
        [bulletproofs_base: InternalGas, { 11.. => "bulletproofs.base" }, 13411764],
        [bulletproofs_per_bit_rangeproof_verify: InternalGasPerArg, { 11.. => "bulletproofs.per_bit_rangeproof_verify" }, 1141942],
        [bulletproofs_per_byte_rangeproof_deserialize: InternalGasPerByte, { 11.. => "bulletproofs.per_byte_rangeproof_deserialize" }, 137],
        // Bulletproofs gas parameters end.

        [type_info_type_of_base: InternalGas, "type_info.type_of.base", 1254],
        // TODO(Gas): the on-chain name is wrong...
        [type_info_type_of_per_byte_in_str: InternalGasPerByte, "type_info.type_of.per_abstract_memory_unit", 20],
        [type_info_type_name_base: InternalGas, "type_info.type_name.base", 1254],
        // TODO(Gas): the on-chain name is wrong...
        [type_info_type_name_per_byte_in_str: InternalGasPerByte, "type_info.type_name.per_abstract_memory_unit", 20],
        [type_info_chain_id_base: InternalGas, { 4.. => "type_info.chain_id.base" }, 627],

        // Reusing SHA2-512's cost from Ristretto
        [hash_sha2_512_base: InternalGas, { 4.. => "hash.sha2_512.base" }, 13543],  // 3_240 * 20
        [hash_sha2_512_per_byte: InternalGasPerByte, { 4.. => "hash.sha2_512.per_byte" }, 250], // 60 * 20
        // Back-of-the-envelope approximation from SHA3-256's costs (4000 base, 45 per-byte)
        [hash_sha3_512_base: InternalGas, { 4.. => "hash.sha3_512.base" }, 18810], // 4_500 * 20
        [hash_sha3_512_per_byte: InternalGasPerByte, { 4.. => "hash.sha3_512.per_byte" }, 209], // 50 * 20
        // Using SHA2-256's cost
        [hash_ripemd160_base: InternalGas, { 4.. => "hash.ripemd160.base" }, 12540], // 3000 * 20
        [hash_ripemd160_per_byte: InternalGasPerByte, { 4.. => "hash.ripemd160.per_byte" }, 209], // 50 * 20
        [hash_blake2b_256_base: InternalGas, { 6.. => "hash.blake2b_256.base" }, 7315], // 1750 * 20
        [hash_blake2b_256_per_byte: InternalGasPerByte, { 6.. => "hash.blake2b_256.per_byte" }, 62], // 15 * 20

        [util_from_bytes_base: InternalGas, "util.from_bytes.base", 1254],
        [util_from_bytes_per_byte: InternalGasPerByte, "util.from_bytes.per_byte", 20],

        [transaction_context_get_txn_hash_base: InternalGas, { 10.. => "transaction_context.get_txn_hash.base" }, 836],
        [transaction_context_get_script_hash_base: InternalGas, "transaction_context.get_script_hash.base", 836],
        // Based on SHA3-256's cost
        [transaction_context_generate_unique_address_base: InternalGas, { 10.. => "transaction_context.generate_unique_address.base" }, 16720],

        [code_request_publish_base: InternalGas, "code.request_publish.base", 2090],
        [code_request_publish_per_byte: InternalGasPerByte, "code.request_publish.per_byte", 8],

        // Note(Gas): These are storage operations so the values should not be multiplied.
        [event_write_to_event_store_base: InternalGas, "event.write_to_event_store.base", 300000],
        // TODO(Gas): the on-chain name is wrong...
        [event_write_to_event_store_per_abstract_value_unit: InternalGasPerAbstractValueUnit, "event.write_to_event_store.per_abstract_memory_unit", 5000],

        [state_storage_get_usage_base_cost: InternalGas, "state_storage.get_usage.base", 2090],

        [aggregator_add_base: InternalGas, "aggregator.add.base", 1254],
        [aggregator_read_base: InternalGas, "aggregator.read.base", 1254],
        [aggregator_sub_base: InternalGas, "aggregator.sub.base", 1254],
        [aggregator_destroy_base: InternalGas, "aggregator.destroy.base", 2090],
        [aggregator_factory_new_aggregator_base: InternalGas, "aggregator_factory.new_aggregator.base", 2090],

        [aggregator_v2_create_aggregator_base: InternalGas, {12.. => "aggregator_v2.create_aggregator.base"}, 2090],
        [aggregator_v2_try_add_base: InternalGas, {12.. => "aggregator_v2.try_add.base"}, 1254],
        [aggregator_v2_try_sub_base: InternalGas, {12.. => "aggregator_v2.try_sub.base"}, 1254],
        [aggregator_v2_read_base: InternalGas, {12.. => "aggregator_v2.read.base"}, 2508],
        [aggregator_v2_snapshot_base: InternalGas, {12.. => "aggregator_v2.snapshot.base"}, 1254],

        [aggregator_v2_create_snapshot_base: InternalGas, {11.. => "aggregator_v2.create_snapshot.base"}, 1254],
        [aggregator_v2_create_snapshot_per_byte: InternalGasPerByte, { 12.. =>"aggregator_v2.create_snapshot.per_byte" }, 4],
        [aggregator_v2_copy_snapshot_base: InternalGas, {11.. => "aggregator_v2.copy_snapshot.base"}, 1254],
        [aggregator_v2_read_snapshot_base: InternalGas, {11.. => "aggregator_v2.read_snapshot.base"}, 2508],
        [aggregator_v2_string_concat_base: InternalGas, {11.. => "aggregator_v2.string_concat.base"}, 1254],
        [aggregator_v2_string_concat_per_byte: InternalGasPerByte, { 12.. =>"aggregator_v2.string_concat.per_byte" }, 4],

        [object_exists_at_base: InternalGas, { 7.. => "object.exists_at.base" }, 1045],
        // These are dummy value, they copied from storage gas in aptos-core/aptos-vm/src/aptos_vm_impl.rs
        [object_exists_at_per_byte_loaded: InternalGasPerByte, { 7.. => "object.exists_at.per_byte_loaded" }, 209],
        [object_exists_at_per_item_loaded: InternalGas, { 7.. => "object.exists_at.per_item_loaded" }, 1672],
        [string_utils_base: InternalGas, { 8.. => "string_utils.format.base" }, 1254],
        [string_utils_per_byte: InternalGasPerByte, { 8.. =>"string_utils.format.per_byte" }, 4],
    ]
);
