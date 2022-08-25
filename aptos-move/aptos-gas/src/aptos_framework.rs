// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use framework::natives::GasParameters;

crate::natives::define_gas_parameters_for_natives!(GasParameters, "aptos_framework", [
    [.account.create_address.base, "account.create_address.base", 1],
    [.account.create_signer.base, "account.create_signer.base", 1],

    [.bls12381.base, "bls12381.base", 1],

    [.bls12381.per_pubkey_deserialize, "bls12381.per_pubkey_deserialize", 1],
    [.bls12381.per_pubkey_aggregate, "bls12381.per_pubkey_aggregate", 1],
    [.bls12381.per_pubkey_subgroup_check, "bls12381.per_pubkey_subgroup_check", 1],

    [.bls12381.per_sig_deserialize, "bls12381.per_sig_deserialize", 1],
    [.bls12381.per_sig_aggregate, "bls12381.per_sig_aggregate", 1],
    [.bls12381.per_sig_subgroup_check, "bls12381.per_sig_subgroup_check", 1],

    [.bls12381.per_sig_verify, "bls12381.per_sig_verify", 1],
    [.bls12381.per_pop_verify, "bls12381.per_pop_verify", 1],

    [.bls12381.per_pairing, "bls12381.per_pairing", 1],

    [.bls12381.per_msg_hashing, "bls12381.per_msg_hashing", 1],
    [.bls12381.per_byte_hashing, "bls12381.per_byte_hashing", 1],

    [.ed25519.base, "signature.base", 1],
    [.ed25519.per_pubkey_deserialize, "signature.per_pubkey_deserialize", 1],
    [.ed25519.per_pubkey_small_order_check, "signature.per_pubkey_small_order_check", 1],
    [.ed25519.per_sig_deserialize, "signature.per_sig_deserialize", 1],
    [.ed25519.per_sig_strict_verify, "signature.per_sig_strict_verify", 1],
    [.ed25519.per_msg_hashing_base, "signature.per_msg_hashing_base", 1],
    [.ed25519.per_msg_byte_hashing, "signature.per_msg_byte_hashing", 1],

    [.secp256k1.base, "secp256k1.base", 1],
    [.secp256k1.ecdsa_recover, "secp256k1.ecdsa_recover", 1],

    [.ristretto255.basepoint_mul, "ristretto255.basepoint_mul", 1],
    [.ristretto255.basepoint_double_mul, "ristretto255.basepoint_double_mul", 1],

    [.ristretto255.point_add, "ristretto255.point_add", 1],
    [.ristretto255.point_clone, "ristretto255.point_clone", 1],
    [.ristretto255.point_compress, "ristretto255.point_compress", 1],
    [.ristretto255.point_decompress, "ristretto255.point_decompress", 1],
    [.ristretto255.point_equals, "ristretto255.point_equals", 1],
    [.ristretto255.point_from_64_uniform_bytes, "ristretto255.point_from_64_uniform_bytes", 1],
    [.ristretto255.point_identity, "ristretto255.point_identity", 1],
    [.ristretto255.point_mul, "ristretto255.point_mul", 1],
    [.ristretto255.point_neg, "ristretto255.point_neg", 1],
    [.ristretto255.point_sub, "ristretto255.point_sub", 1],
    [.ristretto255.point_parse_arg, "ristretto255.point_parse_arg", 1],


    // TODO(Alin): These SHA512 gas costs could be unified with the costs in our future SHA512 module
    // (assuming same implementation complexity, which might not be the case

    // DEPRECATED
    [.ristretto255.sha512_per_byte, "ristretto255.scalar_sha512_per_byte", 1],
    // DEPRECATED
    [.ristretto255.sha512_per_hash, "ristretto255.scalar_sha512_per_hash", 1],

    [.ristretto255.sha2_512_per_byte, "ristretto255.sha2_512_per_byte", 1],
    [.ristretto255.sha2_512_per_hash, "ristretto255.sha2_512_per_hash", 1],

    [.ristretto255.scalar_add, "ristretto255.scalar_add", 1],
    [.ristretto255.scalar_reduced_from_32_bytes, "ristretto255.scalar_reduced_from_32_bytes", 1],
    [.ristretto255.scalar_uniform_from_64_bytes, "ristretto255.scalar_uniform_from_64_bytes", 1],
    [.ristretto255.scalar_from_u128, "ristretto255.scalar_from_u128", 1],
    [.ristretto255.scalar_from_u64, "ristretto255.scalar_from_u64", 1],
    [.ristretto255.scalar_invert, "ristretto255.scalar_invert", 1],
    [.ristretto255.scalar_is_canonical, "ristretto255.scalar_is_canonical", 1],
    [.ristretto255.scalar_mul, "ristretto255.scalar_mul", 1],
    [.ristretto255.scalar_neg, "ristretto255.scalar_neg", 1],
    [.ristretto255.scalar_sub, "ristretto255.scalar_sub", 1],
    [.ristretto255.scalar_parse_arg, "ristretto255.scalar_parse_arg", 1],

    [.bulletproofs.per_rangeproof_deserialize, "bulletproofs.per_rangeproof_deserialize", 1],
    [.bulletproofs.per_bit_rangeproof_verify, "bulletproofs.per_bit_rangeproof_verify", 1],

    [.hash.sip_hash.base, "hash.sip_hash.base", 1],
    [.hash.sip_hash.per_byte, "hash.sip_hash.per_byte", 1],

    [.hash.keccak256.base, optional "hash.keccak256.base", 1],
    [.hash.keccak256.per_byte, optional "hash.keccak256.per_byte", 1],

    [.type_info.type_of.base, "type_info.type_of.base", 1],
    [.type_info.type_of.per_byte_in_str, "type_info.type_of.per_abstract_memory_unit", 1],
    [.type_info.type_name.base, "type_info.type_name.base", 1],
    [.type_info.type_name.per_byte_in_str, "type_info.type_name.per_abstract_memory_unit", 1],

    [.util.from_bytes.base, "util.from_bytes.base", 1],
    [.util.from_bytes.per_byte, "util.from_bytes.per_byte", 1],

    [.transaction_context.get_script_hash.base, "transaction_context.get_script_hash.base", 1],

    [.code.request_publish.base, "code.request_publish.base", 1],
    [.code.request_publish.per_byte, "code.request_publish.per_byte", 1],

    [.event.write_to_event_store.base, "event.write_to_event_store.base", 1],
    [.event.write_to_event_store.per_abstract_value_unit, "event.write_to_event_store.per_abstract_memory_unit", 1],

    [.state_storage.get_usage.base_cost, "state_storage.get_usage.base", 1],

    [.aggregator.add.base, "aggregator.add.base", 1],
    [.aggregator.read.base, "aggregator.read.base", 1],
    [.aggregator.sub.base, "aggregator.sub.base", 1],
    [.aggregator.destroy.base, "aggregator.destroy.base", 1],
    [.aggregator_factory.new_aggregator.base, "aggregator_factory.new_aggregator.base", 1]
]);
