// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use framework::natives::GasParameters;

crate::natives::define_gas_parameters_for_natives!(GasParameters, "aptos_framework", [
    [.account.create_address.base_cost, "account.create_address.base", 1],
    [.account.create_signer.base_cost, "account.create_signer.base", 1],

    [.bls12381.base_cost, "bls12381.base", 1],

    [.bls12381.per_pubkey_deserialize_cost, "bls12381.per_pubkey_deserialize", 1],
    [.bls12381.per_pubkey_aggregate_cost, "bls12381.per_pubkey_aggregate", 1],
    [.bls12381.per_pubkey_subgroup_check_cost, "bls12381.per_pubkey_subgroup_check", 1],

    [.bls12381.per_sig_deserialize_cost, "bls12381.per_sig_deserialize", 1],
    [.bls12381.per_sig_aggregate_cost, "bls12381.per_sig_aggregate", 1],
    [.bls12381.per_sig_subgroup_check_cost, "bls12381.per_sig_subgroup_check", 1],

    [.bls12381.per_sig_verify_cost, "bls12381.per_sig_verify", 1],
    [.bls12381.per_pop_verify_cost, "bls12381.per_pop_verify", 1],

    [.bls12381.per_pairing_cost, "bls12381.per_pairing", 1],

    [.bls12381.per_msg_hashing_cost, "bls12381.per_msg_hashing", 1],
    [.bls12381.per_byte_hashing_cost, "bls12381.per_byte_hashing", 1],

    [.ristretto255.base_cost, "ristretto255.base", 1],

    [.ristretto255.basepoint_mul_cost, "ristretto255.basepoint_mul", 1],
    [.ristretto255.basepoint_double_mul_cost, "ristretto255.basepoint_double_mul", 1],

    [.ristretto255.point_add_cost, "ristretto255.point_add", 1],
    [.ristretto255.point_compress_cost, "ristretto255.point_compress", 1],
    [.ristretto255.point_decompress_cost, "ristretto255.point_decompress", 1],
    [.ristretto255.point_equals_cost, "ristretto255.point_equals", 1],
    [.ristretto255.point_from_64_uniform_bytes_cost, "ristretto255.point_from_64_uniform_bytes", 1],
    [.ristretto255.point_identity_cost, "ristretto255.point_identity", 1],
    [.ristretto255.point_mul_cost, "ristretto255.point_mul", 1],
    [.ristretto255.point_neg_cost, "ristretto255.point_neg", 1],
    [.ristretto255.point_sub_cost, "ristretto255.point_sub", 1],

    // TODO(Alin): These SHA512 gas costs could be unified with the costs in our future SHA512 module
    // (assuming same implementation complexity, which might not be the case
    [.ristretto255.sha512_per_byte_cost, "ristretto255.scalar_sha512_per_byte", 1],
    [.ristretto255.sha512_per_hash_cost, "ristretto255.scalar_sha512_per_hash", 1],

    [.ristretto255.scalar_add_cost, "ristretto255.scalar_add", 1],
    [.ristretto255.scalar_reduced_from_32_bytes_cost, "ristretto255.scalar_reduced_from_32_bytes", 1],
    [.ristretto255.scalar_uniform_from_64_bytes_cost, "ristretto255.scalar_uniform_from_64_bytes", 1],
    [.ristretto255.scalar_from_u128_cost, "ristretto255.scalar_from_u128", 1],
    [.ristretto255.scalar_from_u64_cost, "ristretto255.scalar_from_u64", 1],
    [.ristretto255.scalar_invert_cost, "ristretto255.scalar_invert", 1],
    [.ristretto255.scalar_is_canonical_cost, "ristretto255.scalar_is_canonical", 1],
    [.ristretto255.scalar_mul_cost, "ristretto255.scalar_mul", 1],
    [.ristretto255.scalar_neg_cost, "ristretto255.scalar_neg", 1],
    [.ristretto255.scalar_sub_cost, "ristretto255.scalar_sub", 1],

    [.signature.ed25519_validate_pubkey.base_cost, "signature.ed25519_validate_pubkey.base", 1],
    [.signature.ed25519_validate_pubkey.per_pubkey_deserialize_cost, "signature.ed25519_validate_pubkey.per_pubkey_deserialize", 1],
    [.signature.ed25519_validate_pubkey.per_pubkey_small_order_check_cost, "signature.ed25519_validate_pubkey.per_pubkey_small_order_check", 1],

    [.signature.ed25519_verify.base_cost, "signature.ed25519_verify.base", 1],
    [.signature.ed25519_verify.per_pubkey_deserialize_cost, "signature.ed25519_verify.per_pubkey_deserialize", 1],
    [.signature.ed25519_verify.per_sig_deserialize_cost, "signature.ed25519_verify.per_sig_deserialize", 1],
    [.signature.ed25519_verify.per_sig_strict_verify_cost, "signature.ed25519_verify.per_sig_strict_verify", 1],
    [.signature.ed25519_verify.per_msg_hashing_base_cost, "signature.ed25519_verify.per_msg_hashing_base", 1],
    [.signature.ed25519_verify.per_msg_byte_hashing_cost, "signature.ed25519_verify.per_msg_byte_hashing", 1],

    [.signature.secp256k1_ecdsa_recover.base_cost, "signature.secp256k1_ecdsa_recover.base", 1],

    [.hash.sip_hash.base_cost, "hash.sip_hash.base", 1],
    [.hash.sip_hash.unit_cost, "hash.sip_hash.unit", 1],

    [.type_info.type_of.base_cost, "type_info.type_of.base", 1],
    [.type_info.type_of.unit_cost, "type_info.type_of.unit", 1],
    [.type_info.type_name.base_cost, "type_info.type_name.base", 1],
    [.type_info.type_name.unit_cost, "type_info.type_name.unit", 1],

    [.util.from_bytes.base_cost, "util.from_bytes.base", 1],
    [.util.from_bytes.unit_cost, "util.from_bytes.unit", 1],

    [.transaction_context.get_script_hash.base_cost, "transaction_context.get_script_hash.base", 1],

    [.code.request_publish.base_cost, "code.request_publish.base", 1],
    [.code.request_publish.unit_cost, "code.request_publish.unit", 1],

    [.event.write_to_event_store.base_cost, "event.write_to_event_store.base", 1],
    [.event.write_to_event_store.unit_cost, "event.write_to_event_store.unit", 1],
]);
