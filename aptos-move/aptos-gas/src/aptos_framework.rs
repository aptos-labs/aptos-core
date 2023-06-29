// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::gas_meter::EXECUTION_GAS_MULTIPLIER as MUL;
use aptos_framework::natives::GasParameters;

crate::natives::define_gas_parameters_for_natives!(GasParameters, "aptos_framework", [
    [.account.create_address.base, "account.create_address.base", 300 * MUL],
    [.account.create_signer.base, "account.create_signer.base", 300 * MUL],

    // Algebra gas parameters begin.
    // Generated at time 1680606720.0709136 by `scripts/algebra-gas/update_algebra_gas_params.py` with gas_per_ns=10.23.
    [.algebra.ark_bls12_381_fq12_add, { 8.. => "algebra.ark_bls12_381_fq12_add" }, 1_819 * MUL],
    [.algebra.ark_bls12_381_fq12_clone, { 8.. => "algebra.ark_bls12_381_fq12_clone" }, 211 * MUL],
    [.algebra.ark_bls12_381_fq12_deser, { 8.. => "algebra.ark_bls12_381_fq12_deser" }, 11_180 * MUL],
    [.algebra.ark_bls12_381_fq12_div, { 8.. => "algebra.ark_bls12_381_fq12_div" }, 250_813 * MUL],
    [.algebra.ark_bls12_381_fq12_eq, { 8.. => "algebra.ark_bls12_381_fq12_eq" }, 726 * MUL],
    [.algebra.ark_bls12_381_fq12_from_u64, { 8.. => "algebra.ark_bls12_381_fq12_from_u64" }, 901 * MUL],
    [.algebra.ark_bls12_381_fq12_inv, { 8.. => "algebra.ark_bls12_381_fq12_inv" }, 200_523 * MUL],
    [.algebra.ark_bls12_381_fq12_mul, { 8.. => "algebra.ark_bls12_381_fq12_mul" }, 49_886 * MUL],
    [.algebra.ark_bls12_381_fq12_neg, { 8.. => "algebra.ark_bls12_381_fq12_neg" }, 1_181 * MUL],
    [.algebra.ark_bls12_381_fq12_one, { 8.. => "algebra.ark_bls12_381_fq12_one" }, 11 * MUL],
    [.algebra.ark_bls12_381_fq12_pow_u256, { 8.. => "algebra.ark_bls12_381_fq12_pow_u256" }, 14_664_207 * MUL],
    [.algebra.ark_bls12_381_fq12_serialize, { 8.. => "algebra.ark_bls12_381_fq12_serialize" }, 8_078 * MUL],
    [.algebra.ark_bls12_381_fq12_square, { 8.. => "algebra.ark_bls12_381_fq12_square" }, 35_145 * MUL],
    [.algebra.ark_bls12_381_fq12_sub, { 8.. => "algebra.ark_bls12_381_fq12_sub" }, 1_758 * MUL],
    [.algebra.ark_bls12_381_fq12_zero, { 8.. => "algebra.ark_bls12_381_fq12_zero" }, 211 * MUL],
    [.algebra.ark_bls12_381_fr_add, { 8.. => "algebra.ark_bls12_381_fr_add" }, 211 * MUL],
    [.algebra.ark_bls12_381_fr_deser, { 8.. => "algebra.ark_bls12_381_fr_deser" }, 752 * MUL],
    [.algebra.ark_bls12_381_fr_div, { 8.. => "algebra.ark_bls12_381_fr_div" }, 59_440 * MUL],
    [.algebra.ark_bls12_381_fr_eq, { 8.. => "algebra.ark_bls12_381_fr_eq" }, 212 * MUL],
    [.algebra.ark_bls12_381_fr_from_u64, { 8.. => "algebra.ark_bls12_381_fr_from_u64" }, 494 * MUL],
    [.algebra.ark_bls12_381_fr_inv, { 8.. => "algebra.ark_bls12_381_fr_inv" }, 58_610 * MUL],
    [.algebra.ark_bls12_381_fr_mul, { 8.. => "algebra.ark_bls12_381_fr_mul" }, 502 * MUL],
    [.algebra.ark_bls12_381_fr_neg, { 8.. => "algebra.ark_bls12_381_fr_neg" }, 213 * MUL],
    [.algebra.ark_bls12_381_fr_one, { 8.. => "algebra.ark_bls12_381_fr_one" }, 211 * MUL],
    [.algebra.ark_bls12_381_fr_serialize, { 8.. => "algebra.ark_bls12_381_fr_serialize" }, 1_103 * MUL],
    [.algebra.ark_bls12_381_fr_square, { 8.. => "algebra.ark_bls12_381_fr_square" }, 475 * MUL],
    [.algebra.ark_bls12_381_fr_sub, { 8.. => "algebra.ark_bls12_381_fr_sub" }, 290 * MUL],
    [.algebra.ark_bls12_381_fr_zero, { 8.. => "algebra.ark_bls12_381_fr_zero" }, 211 * MUL],
    [.algebra.ark_bls12_381_g1_affine_deser_comp, { 8.. => "algebra.ark_bls12_381_g1_affine_deser_comp" }, 1_029_599 * MUL],
    [.algebra.ark_bls12_381_g1_affine_deser_uncomp, { 8.. => "algebra.ark_bls12_381_g1_affine_deser_uncomp" }, 720_638 * MUL],
    [.algebra.ark_bls12_381_g1_affine_serialize_comp, { 8.. => "algebra.ark_bls12_381_g1_affine_serialize_comp" }, 2_014 * MUL],
    [.algebra.ark_bls12_381_g1_affine_serialize_uncomp, { 8.. => "algebra.ark_bls12_381_g1_affine_serialize_uncomp" }, 2_433 * MUL],
    [.algebra.ark_bls12_381_g1_proj_add, { 8.. => "algebra.ark_bls12_381_g1_proj_add" }, 10_806 * MUL],
    [.algebra.ark_bls12_381_g1_proj_double, { 8.. => "algebra.ark_bls12_381_g1_proj_double" }, 5_264 * MUL],
    [.algebra.ark_bls12_381_g1_proj_eq, { 8.. => "algebra.ark_bls12_381_g1_proj_eq" }, 5_035 * MUL],
    [.algebra.ark_bls12_381_g1_proj_generator, { 8.. => "algebra.ark_bls12_381_g1_proj_generator" }, 11 * MUL],
    [.algebra.ark_bls12_381_g1_proj_infinity, { 8.. => "algebra.ark_bls12_381_g1_proj_infinity" }, 11 * MUL],
    [.algebra.ark_bls12_381_g1_proj_neg, { 8.. => "algebra.ark_bls12_381_g1_proj_neg" }, 11 * MUL],
    [.algebra.ark_bls12_381_g1_proj_scalar_mul, { 8.. => "algebra.ark_bls12_381_g1_proj_scalar_mul" }, 2_523_521 * MUL],
    [.algebra.ark_bls12_381_g1_proj_sub, { 8.. => "algebra.ark_bls12_381_g1_proj_sub" }, 11_147 * MUL],
    [.algebra.ark_bls12_381_g1_proj_to_affine, { 8.. => "algebra.ark_bls12_381_g1_proj_to_affine" }, 121_035 * MUL],
    [.algebra.ark_bls12_381_g2_affine_deser_comp, { 8.. => "algebra.ark_bls12_381_g2_affine_deser_comp" }, 2_060_068 * MUL],
    [.algebra.ark_bls12_381_g2_affine_deser_uncomp, { 8.. => "algebra.ark_bls12_381_g2_affine_deser_uncomp" }, 1_017_979 * MUL],
    [.algebra.ark_bls12_381_g2_affine_serialize_comp, { 8.. => "algebra.ark_bls12_381_g2_affine_serialize_comp" }, 3_378 * MUL],
    [.algebra.ark_bls12_381_g2_affine_serialize_uncomp, { 8.. => "algebra.ark_bls12_381_g2_affine_serialize_uncomp" }, 4_217 * MUL],
    [.algebra.ark_bls12_381_g2_proj_add, { 8.. => "algebra.ark_bls12_381_g2_proj_add" }, 32_401 * MUL],
    [.algebra.ark_bls12_381_g2_proj_double, { 8.. => "algebra.ark_bls12_381_g2_proj_double" }, 14_839 * MUL],
    [.algebra.ark_bls12_381_g2_proj_eq, { 8.. => "algebra.ark_bls12_381_g2_proj_eq" }, 15_155 * MUL],
    [.algebra.ark_bls12_381_g2_proj_generator, { 8.. => "algebra.ark_bls12_381_g2_proj_generator" }, 11 * MUL],
    [.algebra.ark_bls12_381_g2_proj_infinity, { 8.. => "algebra.ark_bls12_381_g2_proj_infinity" }, 11 * MUL],
    [.algebra.ark_bls12_381_g2_proj_neg, { 8.. => "algebra.ark_bls12_381_g2_proj_neg" }, 11 * MUL],
    [.algebra.ark_bls12_381_g2_proj_scalar_mul, { 8.. => "algebra.ark_bls12_381_g2_proj_scalar_mul" }, 7_526_508 * MUL],
    [.algebra.ark_bls12_381_g2_proj_sub, { 8.. => "algebra.ark_bls12_381_g2_proj_sub" }, 32_869 * MUL],
    [.algebra.ark_bls12_381_g2_proj_to_affine, { 8.. => "algebra.ark_bls12_381_g2_proj_to_affine" }, 128_857 * MUL],
    [.algebra.ark_bls12_381_multi_pairing_base, { 8.. => "algebra.ark_bls12_381_multi_pairing_base" }, 8_998_649 * MUL],
    [.algebra.ark_bls12_381_multi_pairing_per_pair, { 8.. => "algebra.ark_bls12_381_multi_pairing_per_pair" }, 4_602_642 * MUL],
    [.algebra.ark_bls12_381_pairing, { 8.. => "algebra.ark_bls12_381_pairing" }, 14_832_220 * MUL],
    [.algebra.ark_h2c_bls12381g1_xmd_sha256_sswu_base, { 8.. => "algebra.ark_h2c_bls12381g1_xmd_sha256_sswu_base" }, 3_251_943 * MUL],
    [.algebra.ark_h2c_bls12381g1_xmd_sha256_sswu_per_msg_byte, { 8.. => "algebra.ark_h2c_bls12381g1_xmd_sha256_sswu_per_msg_byte" }, 48 * MUL],
    [.algebra.ark_h2c_bls12381g2_xmd_sha256_sswu_base, { 8.. => "algebra.ark_h2c_bls12381g2_xmd_sha256_sswu_base" }, 6_773_002 * MUL],
    [.algebra.ark_h2c_bls12381g2_xmd_sha256_sswu_per_msg_byte, { 8.. => "algebra.ark_h2c_bls12381g2_xmd_sha256_sswu_per_msg_byte" }, 48 * MUL],
    // Algebra gas parameters end.

    [.bls12381.base, "bls12381.base", 150 * MUL],

    [.bls12381.per_pubkey_deserialize, "bls12381.per_pubkey_deserialize", 109_000 * MUL],
    [.bls12381.per_pubkey_aggregate, "bls12381.per_pubkey_aggregate", 4_200 * MUL],
    [.bls12381.per_pubkey_subgroup_check, "bls12381.per_pubkey_subgroup_check", 370_000 * MUL],

    [.bls12381.per_sig_deserialize, "bls12381.per_sig_deserialize", 222_000 * MUL],
    [.bls12381.per_sig_aggregate, "bls12381.per_sig_aggregate", 11_650 * MUL],
    [.bls12381.per_sig_subgroup_check, "bls12381.per_sig_subgroup_check", 460_500 * MUL],

    [.bls12381.per_sig_verify, "bls12381.per_sig_verify", 8_485_000 * MUL],
    [.bls12381.per_pop_verify, "bls12381.per_pop_verify", 10_300_000 * MUL],

    [.bls12381.per_pairing, "bls12381.per_pairing", 4_013_000 * MUL],

    [.bls12381.per_msg_hashing, "bls12381.per_msg_hashing", 1_540_000 * MUL],
    [.bls12381.per_byte_hashing, "bls12381.per_byte_hashing", 50 * MUL],

    [.ed25519.base, "signature.base", 150 * MUL],
    [.ed25519.per_pubkey_deserialize, "signature.per_pubkey_deserialize", 38_000 * MUL],
    [.ed25519.per_pubkey_small_order_check, "signature.per_pubkey_small_order_check", 6_350 * MUL],
    [.ed25519.per_sig_deserialize, "signature.per_sig_deserialize", 375 * MUL],
    [.ed25519.per_sig_strict_verify, "signature.per_sig_strict_verify", 267_000 * MUL],
    [.ed25519.per_msg_hashing_base, "signature.per_msg_hashing_base", 3_240 * MUL],
    [.ed25519.per_msg_byte_hashing, "signature.per_msg_byte_hashing", 60 * MUL],

    [.secp256k1.base, "secp256k1.base", 150 * MUL],
    [.secp256k1.ecdsa_recover, "secp256k1.ecdsa_recover", 1_610_000 * MUL],

    [.ristretto255.basepoint_mul, "ristretto255.basepoint_mul", 128_000 * MUL],
    [.ristretto255.basepoint_double_mul, "ristretto255.basepoint_double_mul", 440_000 * MUL],

    [.ristretto255.point_add, "ristretto255.point_add", 2_135 * MUL],
    [.ristretto255.point_compress, "ristretto255.point_compress", 40_000 * MUL],
    [.ristretto255.point_decompress, "ristretto255.point_decompress", 40_500 * MUL],
    [.ristretto255.point_equals, "ristretto255.point_equals", 2_300 * MUL],
    [.ristretto255.point_from_64_uniform_bytes, "ristretto255.point_from_64_uniform_bytes", 81_500 * MUL],
    [.ristretto255.point_identity, "ristretto255.point_identity", 150 * MUL],
    [.ristretto255.point_mul, "ristretto255.point_mul", 471_000 * MUL],
    [.ristretto255.point_neg, "ristretto255.point_neg", 360 * MUL],
    [.ristretto255.point_sub, "ristretto255.point_sub", 2_130 * MUL],
    [.ristretto255.point_parse_arg, "ristretto255.point_parse_arg", 150 * MUL],


    // TODO(Alin): These SHA512 gas costs could be unified with the costs in our future SHA512 module
    // (assuming same implementation complexity, which might not be the case
    [.ristretto255.sha512_per_byte, "ristretto255.scalar_sha512_per_byte", 60 * MUL],
    [.ristretto255.sha512_per_hash, "ristretto255.scalar_sha512_per_hash", 3_240 * MUL],

    [.ristretto255.scalar_add, "ristretto255.scalar_add", 770 * MUL],
    [.ristretto255.scalar_reduced_from_32_bytes, "ristretto255.scalar_reduced_from_32_bytes", 710 * MUL],
    [.ristretto255.scalar_uniform_from_64_bytes, "ristretto255.scalar_uniform_from_64_bytes", 1_245 * MUL],
    [.ristretto255.scalar_from_u128, "ristretto255.scalar_from_u128", 175 * MUL],
    [.ristretto255.scalar_from_u64, "ristretto255.scalar_from_u64", 175 * MUL],
    [.ristretto255.scalar_invert, "ristretto255.scalar_invert", 110_000 * MUL],
    [.ristretto255.scalar_is_canonical, "ristretto255.scalar_is_canonical", 1_150 * MUL],
    [.ristretto255.scalar_mul, "ristretto255.scalar_mul", 1_065 * MUL],
    [.ristretto255.scalar_neg, "ristretto255.scalar_neg", 725 * MUL],
    [.ristretto255.scalar_sub, "ristretto255.scalar_sub", 1_060 * MUL],
    [.ristretto255.scalar_parse_arg, "ristretto255.scalar_parse_arg", 150 * MUL],

    [.hash.sip_hash.base, "hash.sip_hash.base", 1000 * MUL],
    [.hash.sip_hash.per_byte, "hash.sip_hash.per_byte", 20 * MUL],

    [.hash.keccak256.base, { 1.. => "hash.keccak256.base" }, 4000 * MUL],
    [.hash.keccak256.per_byte, { 1.. => "hash.keccak256.per_byte" }, 45 * MUL],

    [.type_info.type_of.base, "type_info.type_of.base", 300 * MUL],
    // TODO(Gas): the on-chain name is wrong...
    [.type_info.type_of.per_byte_in_str, "type_info.type_of.per_abstract_memory_unit", 5 * MUL],
    [.type_info.type_name.base, "type_info.type_name.base", 300 * MUL],
    // TODO(Gas): the on-chain name is wrong...
    [.type_info.type_name.per_byte_in_str, "type_info.type_name.per_abstract_memory_unit", 5 * MUL],
    [.type_info.chain_id.base, { 4.. => "type_info.chain_id.base" }, 150 * MUL],

    // Reusing SHA2-512's cost from Ristretto
    [.hash.sha2_512.base, { 4.. => "hash.sha2_512.base" }, 3_240],
    [.hash.sha2_512.per_byte, { 4.. => "hash.sha2_512.per_byte" }, 60],
    // Back-of-the-envelop approximation from SHA3-256's (4000 base, 45 per-byte) costs
    [.hash.sha3_512.base, { 4.. => "hash.sha3_512.base" }, 4_500],
    [.hash.sha3_512.per_byte, { 4.. => "hash.sha3_512.per_byte" }, 50],
    // Using SHA2-256's cost
    [.hash.ripemd160.base, { 4.. => "hash.ripemd160.base" }, 3000],
    [.hash.ripemd160.per_byte, { 4.. => "hash.ripemd160.per_byte" }, 50],
    [.hash.blake2b_256.base, { 6.. => "hash.blake2b_256.base" }, 1750],
    [.hash.blake2b_256.per_byte, { 6.. => "hash.blake2b_256.per_byte" }, 15],

    [.util.from_bytes.base, "util.from_bytes.base", 300 * MUL],
    [.util.from_bytes.per_byte, "util.from_bytes.per_byte", 5 * MUL],

    [.transaction_context.get_txn_hash.base, { 10.. => "transaction_context.get_txn_hash.base" }, 200 * MUL],
    [.transaction_context.get_script_hash.base, "transaction_context.get_script_hash.base", 200 * MUL],
    // Based on SHA3-256's cost
    [.transaction_context.generate_unique_address.base, { 10.. => "transaction_context.generate_unique_address.base" }, 4000 * MUL],

    [.code.request_publish.base, "code.request_publish.base", 500 * MUL],
    [.code.request_publish.per_byte, "code.request_publish.per_byte", 2 * MUL],

    // Note(Gas): These are storage operations so the values should not be multiplied.
    [.event.write_to_event_store.base, "event.write_to_event_store.base", 300_000],
    // TODO(Gas): the on-chain name is wrong...
    [.event.write_to_event_store.per_abstract_value_unit, "event.write_to_event_store.per_abstract_memory_unit", 5_000],

    [.state_storage.get_usage.base_cost, "state_storage.get_usage.base", 500 * MUL],

    [.aggregator.add.base, "aggregator.add.base", 300 * MUL],
    [.aggregator.read.base, "aggregator.read.base", 300 * MUL],
    [.aggregator.sub.base, "aggregator.sub.base", 300 * MUL],
    [.aggregator.destroy.base, "aggregator.destroy.base", 500 * MUL],
    [.aggregator_factory.new_aggregator.base, "aggregator_factory.new_aggregator.base", 500 * MUL],

    [.object.exists_at.base, { 7.. => "object.exists_at.base" }, 250 * MUL],
    // These are dummy value, they copied from storage gas in aptos-core/aptos-vm/src/aptos_vm_impl.rs
    [.object.exists_at.per_byte_loaded, { 7.. => "object.exists_at.per_byte_loaded" }, 1000],
    [.object.exists_at.per_item_loaded, { 7.. => "object.exists_at.per_item_loaded" }, 8000],
    [.string_utils.base, {8.. => "string_utils.format.base"}, 300 * MUL],
    [.string_utils.per_byte, {8.. =>"string_utils.format.per_byte"}, MUL],
]);
