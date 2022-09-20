// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use framework::natives::GasParameters;

crate::natives::define_gas_parameters_for_natives!(GasParameters, "aptos_framework", [
    [.account.create_address.base, "account.create_address.base", 300],
    [.account.create_signer.base, "account.create_signer.base", 300],

    [.bls12381.base, "bls12381.base", 3000],

    [.bls12381.per_pubkey_deserialize, "bls12381.per_pubkey_deserialize", 300],
    [.bls12381.per_pubkey_aggregate, "bls12381.per_pubkey_aggregate", 300],
    [.bls12381.per_pubkey_subgroup_check, "bls12381.per_pubkey_subgroup_check", 300],

    [.bls12381.per_sig_deserialize, "bls12381.per_sig_deserialize", 300],
    [.bls12381.per_sig_aggregate, "bls12381.per_sig_aggregate", 300],
    [.bls12381.per_sig_subgroup_check, "bls12381.per_sig_subgroup_check", 300],

    [.bls12381.per_sig_verify, "bls12381.per_sig_verify", 300],
    [.bls12381.per_pop_verify, "bls12381.per_pop_verify", 300],

    [.bls12381.per_pairing, "bls12381.per_pairing", 300],

    [.bls12381.per_msg_hashing, "bls12381.per_msg_hashing", 300],
    [.bls12381.per_byte_hashing, "bls12381.per_byte_hashing", 50],

    [.ed25519.base, "signature.base", 3000],
    [.ed25519.per_pubkey_deserialize, "signature.per_pubkey_deserialize", 300],
    [.ed25519.per_pubkey_small_order_check, "signature.per_pubkey_small_order_check", 300],
    [.ed25519.per_sig_deserialize, "signature.per_sig_deserialize", 300],
    [.ed25519.per_sig_strict_verify, "signature.per_sig_strict_verify", 300],
    [.ed25519.per_msg_hashing_base, "signature.per_msg_hashing_base", 100],
    [.ed25519.per_msg_byte_hashing, "signature.per_msg_byte_hashing", 50],

    [.secp256k1.base, "secp256k1.base", 1000],
    [.secp256k1.ecdsa_recover, "secp256k1.ecdsa_recover", 300],

    [.ristretto255.basepoint_mul, "ristretto255.basepoint_mul", 300],
    [.ristretto255.basepoint_double_mul, "ristretto255.basepoint_double_mul", 300],

    [.ristretto255.point_add, "ristretto255.point_add", 300],
    [.ristretto255.point_compress, "ristretto255.point_compress", 300],
    [.ristretto255.point_decompress, "ristretto255.point_decompress", 300],
    [.ristretto255.point_equals, "ristretto255.point_equals", 300],
    [.ristretto255.point_from_64_uniform_bytes, "ristretto255.point_from_64_uniform_bytes", 300],
    [.ristretto255.point_identity, "ristretto255.point_identity", 300],
    [.ristretto255.point_mul, "ristretto255.point_mul", 300],
    [.ristretto255.point_neg, "ristretto255.point_neg", 300],
    [.ristretto255.point_sub, "ristretto255.point_sub", 300],
    [.ristretto255.point_parse_arg, "ristretto255.point_parse_arg", 300],


    // TODO(Alin): These SHA512 gas costs could be unified with the costs in our future SHA512 module
    // (assuming same implementation complexity, which might not be the case
    [.ristretto255.sha512_per_byte, "ristretto255.scalar_sha512_per_byte", 50],
    [.ristretto255.sha512_per_hash, "ristretto255.scalar_sha512_per_hash", 300],

    [.ristretto255.scalar_add, "ristretto255.scalar_add", 300],
    [.ristretto255.scalar_reduced_from_32_bytes, "ristretto255.scalar_reduced_from_32_bytes", 300],
    [.ristretto255.scalar_uniform_from_64_bytes, "ristretto255.scalar_uniform_from_64_bytes", 300],
    [.ristretto255.scalar_from_u128, "ristretto255.scalar_from_u128", 300],
    [.ristretto255.scalar_from_u64, "ristretto255.scalar_from_u64", 300],
    [.ristretto255.scalar_invert, "ristretto255.scalar_invert", 300],
    [.ristretto255.scalar_is_canonical, "ristretto255.scalar_is_canonical", 300],
    [.ristretto255.scalar_mul, "ristretto255.scalar_mul", 300],
    [.ristretto255.scalar_neg, "ristretto255.scalar_neg", 300],
    [.ristretto255.scalar_sub, "ristretto255.scalar_sub", 300],
    [.ristretto255.scalar_parse_arg, "ristretto255.scalar_parse_arg", 300],

    [.hash.sip_hash.base, "hash.sip_hash.base", 1000],
    [.hash.sip_hash.per_byte, "hash.sip_hash.per_byte", 20],

    [.hash.keccak256.base, optional "hash.keccak256.base", 3000],
    [.hash.keccak256.per_byte, optional "hash.keccak256.per_byte", 50],

    [.type_info.type_of.base, "type_info.type_of.base", 300],
    // TODO(Gas): the on-chain name is wrong...
    [.type_info.type_of.per_byte_in_str, "type_info.type_of.per_abstract_memory_unit", 5],
    [.type_info.type_name.base, "type_info.type_name.base", 300],
    // TODO(Gas): the on-chain name is wrong...
    [.type_info.type_name.per_byte_in_str, "type_info.type_name.per_abstract_memory_unit", 5],

    [.util.from_bytes.base, "util.from_bytes.base", 300],
    [.util.from_bytes.per_byte, "util.from_bytes.per_byte", 5],

    [.transaction_context.get_script_hash.base, "transaction_context.get_script_hash.base", 200],

    [.code.request_publish.base, "code.request_publish.base", 500],
    [.code.request_publish.per_byte, "code.request_publish.per_byte", 2],

    [.event.write_to_event_store.base, "event.write_to_event_store.base", 600],
    // TODO(Gas): the on-chain name is wrong...
    [.event.write_to_event_store.per_abstract_value_unit, "event.write_to_event_store.per_abstract_memory_unit", 4],

    [.state_storage.get_usage.base_cost, "state_storage.get_usage.base", 500],

    [.aggregator.add.base, "aggregator.add.base", 300],
    [.aggregator.read.base, "aggregator.read.base", 300],
    [.aggregator.sub.base, "aggregator.sub.base", 300],
    [.aggregator.destroy.base, "aggregator.destroy.base", 500],
    [.aggregator_factory.new_aggregator.base, "aggregator_factory.new_aggregator.base", 500]
]);
