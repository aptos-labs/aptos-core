// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

pub mod account;
pub mod aggregator_natives;
pub mod any;
pub mod code;
pub mod create_signer;
pub mod cryptography;
pub mod event;
pub mod hash;
mod helpers;
pub mod state_storage;
pub mod transaction_context;
pub mod type_info;
pub mod util;

use crate::natives::cryptography::groups;
use crate::natives::cryptography::multi_ed25519;
use aggregator_natives::{aggregator, aggregator_factory};
use aptos_gas_algebra_ext::AbstractValueSize;
use cryptography::ed25519;

use crate::natives::cryptography::groups::Bls12381GasParameters;
use move_core_types::{account_address::AccountAddress, identifier::Identifier};
use move_vm_runtime::native_functions::{make_table_from_iter, NativeFunctionTable};
use move_vm_types::values::Value;

pub mod status {
    // Failure in parsing a struct type tag
    pub const NFE_EXPECTED_STRUCT_TYPE_TAG: u64 = 0x1;
    // Failure in address parsing (likely no correct length)
    pub const NFE_UNABLE_TO_PARSE_ADDRESS: u64 = 0x2;
}

#[derive(Debug, Clone)]
pub struct GasParameters {
    pub account: account::GasParameters,
    pub groups: groups::GasParameters,
    pub ed25519: ed25519::GasParameters,
    pub bls12381: cryptography::bls12381::GasParameters,
    pub secp256k1: cryptography::secp256k1::GasParameters,
    pub ristretto255: cryptography::ristretto255::GasParameters,
    pub hash: hash::GasParameters,
    pub type_info: type_info::GasParameters,
    pub util: util::GasParameters,
    pub transaction_context: transaction_context::GasParameters,
    pub code: code::GasParameters,
    pub event: event::GasParameters,
    pub state_storage: state_storage::GasParameters,
    pub aggregator: aggregator::GasParameters,
    pub aggregator_factory: aggregator_factory::GasParameters,
}

impl GasParameters {
    pub fn zeros() -> Self {
        Self {
            account: account::GasParameters {
                create_address: account::CreateAddressGasParameters { base: 0.into() },
                create_signer: create_signer::CreateSignerGasParameters { base: 0.into() },
            },
            bls12381: cryptography::bls12381::GasParameters {
                base: 0.into(),
                per_pubkey_deserialize: 0.into(),
                per_pubkey_aggregate: 0.into(),
                per_pubkey_subgroup_check: 0.into(),
                per_sig_deserialize: 0.into(),
                per_sig_aggregate: 0.into(),
                per_sig_subgroup_check: 0.into(),
                per_sig_verify: 0.into(),
                per_pop_verify: 0.into(),
                per_pairing: 0.into(),
                per_msg_hashing: 0.into(),
                per_byte_hashing: 0.into(),
            },
            groups: groups::GasParameters {
                bls12_381: Bls12381GasParameters {
                    blst_g1_msm_base: 0.into(),
                    blst_g1_msm_per_pair: 0.into(),
                    blst_g2_msm_base: 0.into(),
                    blst_g2_msm_per_pair: 0.into(),
                    blst_hash_to_g1_proj_base: 0.into(),
                    blst_hash_to_g1_proj_per_byte: 0.into(),
                    blst_hash_to_g2_proj_base: 0.into(),
                    blst_hash_to_g2_proj_per_byte: 0.into(),
                    blst_g1_proj_to_affine: 0.into(),
                    blst_g1_affine_ser: 0.into(),
                    blst_g2_proj_to_affine: 0.into(),
                    ark_fr_serialize: 0.into(),
                    ark_fr_deser: 0.into(),
                    ark_fr_from_u128: 0.into(),
                    ark_fr_neg: 0.into(),
                    ark_fr_add: 0.into(),
                    ark_fr_sub: 0.into(),
                    ark_fr_mul: 0.into(),
                    ark_fr_inv: 0.into(),
                    ark_fr_div: 0.into(),
                    ark_fr_eq: 0.into(),
                    ark_g1_affine_infinity: 0.into(),
                    ark_g1_affine_generator: 0.into(),
                    ark_g1_proj_infinity: 0.into(),
                    ark_g1_proj_generator: 0.into(),
                    ark_g1_affine_scalar_mul_to_proj: 0.into(),
                    ark_g1_affine_ser_uncomp: 0.into(),
                    ark_g1_affine_deser_uncomp: 0.into(),
                    ark_g1_affine_ser_comp: 0.into(),
                    ark_g1_affine_deser_comp: 0.into(),
                    ark_g1_affine_neg: 0.into(),
                    ark_g1_affine_add: 0.into(),
                    ark_g1_affine_to_proj: 0.into(),
                    ark_g1_proj_to_affine: 0.into(),
                    ark_g1_proj_neg: 0.into(),
                    ark_g1_proj_add: 0.into(),
                    ark_g1_proj_sub: 0.into(),
                    ark_g1_proj_scalar_mul: 0.into(),
                    ark_g1_affine_eq_proj: 0.into(),
                    ark_g1_proj_eq: 0.into(),
                    ark_g1_affine_to_prepared: 0.into(),
                    ark_g1_proj_to_prepared: 0.into(),
                    ark_g2_affine_infinity: 0.into(),
                    ark_g2_affine_generator: 0.into(),
                    ark_g2_proj_infinity: 0.into(),
                    ark_g2_proj_generator: 0.into(),
                    ark_g2_affine_scalar_mul_to_proj: 0.into(),
                    ark_g2_affine_ser_uncomp: 0.into(),
                    ark_g2_affine_deser_uncomp: 0.into(),
                    ark_g2_affine_ser_comp: 0.into(),
                    ark_g2_affine_deser_comp: 0.into(),
                    ark_g2_affine_neg: 0.into(),
                    ark_g2_affine_add: 0.into(),
                    ark_g2_affine_to_proj: 0.into(),
                    ark_g2_proj_to_affine: 0.into(),
                    ark_g2_proj_neg: 0.into(),
                    ark_g2_proj_add: 0.into(),
                    ark_g2_proj_sub: 0.into(),
                    ark_g2_proj_scalar_mul: 0.into(),
                    ark_g2_proj_eq: 0.into(),
                    ark_g2_affine_to_prepared: 0.into(),
                    ark_g2_proj_to_prepared: 0.into(),
                    ark_fq12_serialize: 0.into(),
                    ark_fq12_deserialize: 0.into(),
                    ark_fq12_pow_fr: 0.into(),
                    ark_fq12_eq: 0.into(),
                    ark_fq12_one: 0.into(),
                    ark_fq12_clone: 0.into(),
                    ark_fq12_mul: 0.into(),
                    ark_fr_to_repr: 0.into(),
                    ark_pairing_product_base: 0.into(),
                    ark_pairing_product_per_pair: 0.into(),
                    ark_fq12_inv: 0.into(),
                    ark_fq12_square: 0.into(),
                    ark_g1_proj_double: 0.into(),
                    ark_g2_proj_double: 0.into(),
                    blst_g2_affine_ser: 0.into(),
                    ark_g2_affine_eq: 0.into(),
                },
            },
            ed25519: ed25519::GasParameters {
                base: 0.into(),
                per_pubkey_deserialize: 0.into(),
                per_pubkey_small_order_check: 0.into(),
                per_sig_deserialize: 0.into(),
                per_sig_strict_verify: 0.into(),
                per_msg_hashing_base: 0.into(),
                per_msg_byte_hashing: 0.into(),
            },
            secp256k1: cryptography::secp256k1::GasParameters {
                base: 0.into(),
                ecdsa_recover: 0.into(),
            },
            ristretto255: cryptography::ristretto255::GasParameters {
                basepoint_mul: 0.into(),
                basepoint_double_mul: 0.into(),
                point_add: 0.into(),
                point_compress: 0.into(),
                point_decompress: 0.into(),
                point_equals: 0.into(),
                point_from_64_uniform_bytes: 0.into(),
                point_identity: 0.into(),
                point_mul: 0.into(),
                point_neg: 0.into(),
                point_sub: 0.into(),
                scalar_add: 0.into(),
                scalar_reduced_from_32_bytes: 0.into(),
                scalar_uniform_from_64_bytes: 0.into(),
                scalar_from_u128: 0.into(),
                scalar_from_u64: 0.into(),
                scalar_invert: 0.into(),
                scalar_is_canonical: 0.into(),
                scalar_mul: 0.into(),
                scalar_neg: 0.into(),
                sha512_per_byte: 0.into(),
                sha512_per_hash: 0.into(),
                scalar_sub: 0.into(),
                point_parse_arg: 0.into(),
                scalar_parse_arg: 0.into(),
            },
            hash: hash::GasParameters {
                sip_hash: hash::SipHashGasParameters {
                    base: 0.into(),
                    per_byte: 0.into(),
                },
                keccak256: hash::Keccak256HashGasParameters {
                    base: 0.into(),
                    per_byte: 0.into(),
                },
                sha2_512: hash::Sha2_512HashGasParameters {
                    base: 0.into(),
                    per_byte: 0.into(),
                },
                sha3_512: hash::Sha3_512HashGasParameters {
                    base: 0.into(),
                    per_byte: 0.into(),
                },
                ripemd160: hash::Ripemd160HashGasParameters {
                    base: 0.into(),
                    per_byte: 0.into(),
                },
                blake2b_256: hash::Blake2B256HashGasParameters {
                    base: 0.into(),
                    per_byte: 0.into(),
                },
            },
            type_info: type_info::GasParameters {
                type_of: type_info::TypeOfGasParameters {
                    base: 0.into(),
                    per_byte_in_str: 0.into(),
                },
                type_name: type_info::TypeNameGasParameters {
                    base: 0.into(),
                    per_byte_in_str: 0.into(),
                },
                chain_id: type_info::ChainIdGasParameters { base: 0.into() },
            },
            util: util::GasParameters {
                from_bytes: util::FromBytesGasParameters {
                    base: 0.into(),
                    per_byte: 0.into(),
                },
            },
            transaction_context: transaction_context::GasParameters {
                get_script_hash: transaction_context::GetScriptHashGasParameters { base: 0.into() },
            },
            code: code::GasParameters {
                request_publish: code::RequestPublishGasParameters {
                    base: 0.into(),
                    per_byte: 0.into(),
                },
            },
            event: event::GasParameters {
                write_to_event_store: event::WriteToEventStoreGasParameters {
                    base: 0.into(),
                    per_abstract_value_unit: 0.into(),
                },
            },
            state_storage: state_storage::GasParameters {
                get_usage: state_storage::GetUsageGasParameters {
                    base_cost: 0.into(),
                },
            },
            aggregator: aggregator::GasParameters {
                add: aggregator::AddGasParameters { base: 0.into() },
                read: aggregator::ReadGasParameters { base: 0.into() },
                sub: aggregator::SubGasParameters { base: 0.into() },
                destroy: aggregator::DestroyGasParameters { base: 0.into() },
            },
            aggregator_factory: aggregator_factory::GasParameters {
                new_aggregator: aggregator_factory::NewAggregatorGasParameters { base: 0.into() },
            },
        }
    }
}

pub fn all_natives(
    framework_addr: AccountAddress,
    gas_params: GasParameters,
    calc_abstract_val_size: impl Fn(&Value) -> AbstractValueSize + Send + Sync + 'static,
) -> NativeFunctionTable {
    let mut natives = vec![];

    macro_rules! add_natives_from_module {
        ($module_name:expr, $natives:expr) => {
            natives.extend(
                $natives.map(|(func_name, func)| ($module_name.to_string(), func_name, func)),
            );
        };
    }

    add_natives_from_module!("account", account::make_all(gas_params.account.clone()));
    add_natives_from_module!(
        "create_signer",
        create_signer::make_all(gas_params.account.create_signer.clone())
    );
    add_natives_from_module!("groups", groups::make_all(gas_params.groups.clone()));
    add_natives_from_module!("ed25519", ed25519::make_all(gas_params.ed25519.clone()));
    add_natives_from_module!(
        "genesis",
        create_signer::make_all(gas_params.account.create_signer)
    );
    add_natives_from_module!("multi_ed25519", multi_ed25519::make_all(gas_params.ed25519));
    add_natives_from_module!(
        "bls12381",
        cryptography::bls12381::make_all(gas_params.bls12381)
    );
    add_natives_from_module!(
        "secp256k1",
        cryptography::secp256k1::make_all(gas_params.secp256k1)
    );
    add_natives_from_module!("aptos_hash", hash::make_all(gas_params.hash));
    add_natives_from_module!(
        "ristretto255",
        cryptography::ristretto255::make_all(gas_params.ristretto255)
    );
    add_natives_from_module!("type_info", type_info::make_all(gas_params.type_info));
    add_natives_from_module!("util", util::make_all(gas_params.util.clone()));
    add_natives_from_module!("from_bcs", util::make_all(gas_params.util));
    add_natives_from_module!(
        "transaction_context",
        transaction_context::make_all(gas_params.transaction_context)
    );
    add_natives_from_module!("code", code::make_all(gas_params.code));
    add_natives_from_module!(
        "event",
        event::make_all(gas_params.event, calc_abstract_val_size)
    );
    add_natives_from_module!(
        "state_storage",
        state_storage::make_all(gas_params.state_storage)
    );
    add_natives_from_module!("aggregator", aggregator::make_all(gas_params.aggregator));
    add_natives_from_module!(
        "aggregator_factory",
        aggregator_factory::make_all(gas_params.aggregator_factory)
    );

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
