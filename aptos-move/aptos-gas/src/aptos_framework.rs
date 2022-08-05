// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use framework::natives::GasParameters;

crate::natives::define_gas_parameters_for_natives!(GasParameters, "aptos_framework", [
    [.account.create_address.base_cost, "account.create_address.base", 1],
    [.account.create_signer.base_cost, "account.create_signer.base", 1],

    [.signature.bls12381_validate_pubkey.base_cost, "signature.bls12381_validate_pubkey.base", 1],
    [.signature.ed25519_validate_pubkey.base_cost, "signature.ed25519_validate_pubkey.base", 1],
    [.signature.ed25519_verify.base_cost, "signature.ed25519_verify.base", 1],
    [.signature.ed25519_verify.unit_cost, "signature.ed25519_verify.unit", 1],
    [.signature.secp256k1_ecdsa_recover.base_cost, "signature.secp256k1_ecdsa_recover.base", 1],
    [.signature.bls12381_verify_signature.base_cost, "signature.bls12381_verify_signature.base", 1],
    [.signature.bls12381_verify_signature.unit_cost, "signature.bls12381_verify_signature.unit", 1],
    [.signature.bls12381_aggregate_pop_verified_pubkeys.base_cost, "signature.bls12381_aggregate_pop_verified_pubkeys.base", 1],
    [.signature.bls12381_aggregate_pop_verified_pubkeys.per_pubkey_cost, "signature.bls12381_aggregate_pop_verified_pubkeys.per_pubkey", 1],
    [.signature.bls12381_verify_proof_of_possession.base_cost, "signature.bls12381_verify_proof_of_possession.base", 1],

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

    [.event.write_to_event_store.unit_cost, "event.write_to_event_store.unit", 1],
]);
