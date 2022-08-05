// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

pub mod account;
pub mod code;
pub mod event;
pub mod hash;
mod helpers;
pub mod signature;
pub mod transaction_context;
pub mod type_info;
pub mod util;

use move_deps::{
    move_core_types::{account_address::AccountAddress, identifier::Identifier},
    move_vm_runtime::native_functions::{make_table_from_iter, NativeFunctionTable},
};

pub mod status {
    // Failure in parsing a struct type tag
    pub const NFE_EXPECTED_STRUCT_TYPE_TAG: u64 = 0x1;
    // Failure in address parsing (likely no correct length)
    pub const NFE_UNABLE_TO_PARSE_ADDRESS: u64 = 0x2;
}

#[derive(Debug, Clone)]
pub struct GasParameters {
    pub account: account::GasParameters,
    pub signature: signature::GasParameters,
    pub hash: hash::GasParameters,
    pub type_info: type_info::GasParameters,
    pub util: util::GasParameters,
    pub transaction_context: transaction_context::GasParameters,
    pub code: code::GasParameters,
    pub event: event::GasParameters,
}

impl GasParameters {
    pub fn zeros() -> Self {
        Self {
            account: account::GasParameters {
                create_address: account::CreateAddressGasParameters { base_cost: 0 },
                create_signer: account::CreateSignerGasParameters { base_cost: 0 },
            },
            signature: signature::GasParameters {
                // BLS signatures over BLS12-381 curves
                bls12381_aggregate_pop_verified_pubkeys:
                    signature::Bls12381AggregatePopVerifiedPubkeysGasParameters {
                        base_cost: 0,
                        per_pubkey_deserialize_cost: 0,
                        per_pubkey_aggregate_cost: 0,
                    },
                bls12381_aggregate_signatures:
                    signature::Bls12381AggregateSignaturesGasParameters {
                        base_cost: 0,
                        per_sig_deserialize_cost: 0,
                        per_sig_aggregate_cost: 0,
                    },
                bls12381_signature_subgroup_check:
                    signature::Bls12381SignatureSubgroupCheckGasParameters {
                        base_cost: 0,
                        per_sig_deserialize_cost: 0,
                        per_sig_subgroup_check_cost: 0,
                    },
                bls12381_validate_pubkey: signature::Bls12381ValidatePubkeyGasParameters {
                    base_cost: 0,
                    per_pubkey_deserialize_cost: 0,
                    per_pubkey_subgroup_check_cost: 0,
                },
                bls12381_verify_aggregate_signature:
                    signature::Bls12381VerifyAggregateSignatureGasParameters {
                        base_cost: 0,
                        per_pubkey_deserialize_cost: 0,
                        per_sig_deserialize_cost: 0,
                        per_pairing_cost: 0,
                        per_msg_hashing_base_cost: 0,
                        per_msg_byte_hashing_cost: 0,
                    },
                bls12381_verify_multisignature:
                    signature::Bls12381VerifyMultisignatureGasParameters {
                        base_cost: 0,
                        per_pubkey_deserialize_cost: 0,
                        per_pubkey_subgroup_check_cost: 0,
                        per_sig_deserialize_cost: 0,
                        per_sig_verify_cost: 0,
                        per_msg_hashing_base_cost: 0,
                        per_msg_byte_hashing_cost: 0,
                    },
                bls12381_verify_normal_signature:
                    signature::Bls12381VerifyNormalSignatureGasParameters {
                        base_cost: 0,
                        per_pubkey_deserialize_cost: 0,
                        per_pubkey_subgroup_check_cost: 0,
                        per_sig_deserialize_cost: 0,
                        per_sig_verify_cost: 0,
                        per_msg_hashing_base_cost: 0,
                        per_msg_byte_hashing_cost: 0,
                    },
                bls12381_verify_proof_of_possession:
                    signature::Bls12381VerifyProofOfPosessionGasParameters {
                        base_cost: 0,
                        per_pubkey_deserialize_cost: 0,
                        per_sig_deserialize_cost: 0,
                        per_pop_verify_cost: 0,
                    },
                bls12381_verify_signature_share:
                    signature::Bls12381VerifySignatureShareGasParameters {
                        base_cost: 0,
                        per_pubkey_deserialize_cost: 0,
                        per_pubkey_subgroup_check_cost: 0,
                        per_sig_deserialize_cost: 0,
                        per_sig_verify_cost: 0,
                        per_msg_hashing_base_cost: 0,
                        per_msg_byte_hashing_cost: 0,
                    },

                // Ed25519
                ed25519_validate_pubkey: signature::Ed25519ValidatePubkeyGasParameters {
                    base_cost: 0,
                    per_pubkey_deserialize_cost: 0,
                    per_pubkey_small_order_check_cost: 0,
                },
                ed25519_verify: signature::Ed25519VerifyGasParameters {
                    base_cost: 0,
                    per_pubkey_deserialize_cost: 0,
                    per_sig_deserialize_cost: 0,
                    per_sig_strict_verify_cost: 0,
                    per_msg_hashing_base_cost: 0,
                    per_msg_byte_hashing_cost: 0,
                },

                // secp256k1
                secp256k1_ecdsa_recover: signature::Secp256k1ECDSARecoverGasParameters {
                    base_cost: 0,
                },
            },
            hash: hash::GasParameters {
                sip_hash: hash::SipHashGasParameters {
                    base_cost: 0,
                    unit_cost: 0,
                },
            },
            type_info: type_info::GasParameters {
                type_of: type_info::TypeOfGasParameters {
                    base_cost: 0,
                    unit_cost: 0,
                },
                type_name: type_info::TypeNameGasParameters {
                    base_cost: 0,
                    unit_cost: 0,
                },
            },
            util: util::GasParameters {
                from_bytes: util::FromBytesGasParameters {
                    base_cost: 0,
                    unit_cost: 0,
                },
            },
            transaction_context: transaction_context::GasParameters {
                get_script_hash: transaction_context::GetScriptHashGasParameters { base_cost: 0 },
            },
            code: code::GasParameters {
                request_publish: code::RequestPublishGasParameters {
                    base_cost: 0,
                    unit_cost: 0,
                },
            },
            event: event::GasParameters {
                write_to_event_store: event::WriteToEventStoreGasParameters { unit_cost: 0 },
            },
        }
    }
}

pub fn all_natives(
    framework_addr: AccountAddress,
    gas_params: GasParameters,
) -> NativeFunctionTable {
    let mut natives = vec![];

    macro_rules! add_natives_from_module {
        ($module_name: expr, $natives: expr) => {
            natives.extend(
                $natives.map(|(func_name, func)| ($module_name.to_string(), func_name, func)),
            );
        };
    }

    add_natives_from_module!("account", account::make_all(gas_params.account));
    add_natives_from_module!("signature", signature::make_all(gas_params.signature));
    add_natives_from_module!("hash", hash::make_all(gas_params.hash));
    add_natives_from_module!("type_info", type_info::make_all(gas_params.type_info));
    add_natives_from_module!("util", util::make_all(gas_params.util));
    add_natives_from_module!(
        "transaction_context",
        transaction_context::make_all(gas_params.transaction_context)
    );
    add_natives_from_module!("code", code::make_all(gas_params.code));
    add_natives_from_module!("event", event::make_all(gas_params.event));

    make_table_from_iter(framework_addr, natives)
}

/// A temporary hack to patch Table -> table module name as long as it is not upgraded
/// in the Move repo.
pub fn patch_table_module(table: NativeFunctionTable) -> NativeFunctionTable {
    table
        .into_iter()
        .map(|(m, _, f, i)| (m, Identifier::new("table").unwrap(), f, i))
        .collect()
}
