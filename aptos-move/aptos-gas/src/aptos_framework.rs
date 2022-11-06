// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::gas_meter::EXECUTION_GAS_MULTIPLIER as MUL;
use framework::natives::GasParameters;

crate::natives::define_gas_parameters_for_natives!(GasParameters, "aptos_framework", [
    [.account.create_address.base, "account.create_address.base", 300 * MUL],
    [.account.create_signer.base, "account.create_signer.base", 300 * MUL],

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

    [.hash.keccak256.base, optional "hash.keccak256.base", 4000 * MUL],
    [.hash.keccak256.per_byte, optional "hash.keccak256.per_byte", 45 * MUL],

    [.type_info.type_of.base, "type_info.type_of.base", 300 * MUL],
    // TODO(Gas): the on-chain name is wrong...
    [.type_info.type_of.per_byte_in_str, "type_info.type_of.per_abstract_memory_unit", 5 * MUL],
    [.type_info.type_name.base, "type_info.type_name.base", 300 * MUL],
    // TODO(Gas): the on-chain name is wrong...
    [.type_info.type_name.per_byte_in_str, "type_info.type_name.per_abstract_memory_unit", 5 * MUL],
    [.type_info.chain_id.base, optional "type_info.chain_id.base", 150 * MUL],

    // Reusing SHA2-512's cost from Ristretto
    [.hash.sha2_512.base, optional "hash.sha2_512.base", 3_240],
    [.hash.sha2_512.per_byte, optional "hash.sha2_512.per_byte", 60],
    // Back-of-the-envelop approximation from SHA3-256's (4000 base, 45 per-byte) costs
    [.hash.sha3_512.base, optional "hash.sha3_512.base", 4_500],
    [.hash.sha3_512.per_byte, optional "hash.sha3_512.per_byte", 50],
    // Using SHA2-256's cost
    [.hash.ripemd160.base, optional "hash.ripemd160.base", 3000],
    [.hash.ripemd160.per_byte, optional "hash.ripemd160.per_byte", 50],

    [.util.from_bytes.base, "util.from_bytes.base", 300 * MUL],
    [.util.from_bytes.per_byte, "util.from_bytes.per_byte", 5 * MUL],

    [.transaction_context.get_script_hash.base, "transaction_context.get_script_hash.base", 200 * MUL],

    [.code.request_publish.base, "code.request_publish.base", 500 * MUL],
    [.code.request_publish.per_byte, "code.request_publish.per_byte", 2 * MUL],

    // Note(Gas): These are storage operations so the values should not be multiplied.
    [.event.write_to_event_store.base, "event.write_to_event_store.base", 500_000],
    // TODO(Gas): the on-chain name is wrong...
    [.event.write_to_event_store.per_abstract_value_unit, "event.write_to_event_store.per_abstract_memory_unit", 5_000],

    [.state_storage.get_usage.base_cost, "state_storage.get_usage.base", 500 * MUL],

    [.aggregator.add.base, "aggregator.add.base", 300 * MUL],
    [.aggregator.read.base, "aggregator.read.base", 300 * MUL],
    [.aggregator.sub.base, "aggregator.sub.base", 300 * MUL],
    [.aggregator.destroy.base, "aggregator.destroy.base", 500 * MUL],
    [.aggregator_factory.new_aggregator.base, "aggregator_factory.new_aggregator.base", 500 * MUL]
]);
