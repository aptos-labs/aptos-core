// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

pub mod account;

pub mod account_abstraction;
pub mod aggregator_natives;
pub mod code;
pub mod consensus_config;
pub mod create_signer;
pub mod cryptography;
pub mod debug;
pub mod dispatchable_fungible_asset;
pub mod event;
pub mod function_info;
pub mod hash;
pub mod object;
pub mod object_code_deployment;
pub mod permissioned_signer;
pub mod randomness;
pub mod state_storage;
pub mod string_utils;
pub mod transaction_context;
pub mod type_info;
pub mod util;

use crate::natives::cryptography::multi_ed25519;
use aggregator_natives::{aggregator, aggregator_factory, aggregator_v2};
use aptos_native_interface::{RawSafeNative, SafeNativeBuilder};
use cryptography::ed25519;
use move_core_types::account_address::AccountAddress;
use move_vm_runtime::native_functions::{make_table_from_iter, NativeFunctionTable};

pub mod status {
    // Failure in parsing a struct type tag
    pub const NFE_EXPECTED_STRUCT_TYPE_TAG: u64 = 0x1;
    // Failure in address parsing (likely no correct length)
    pub const NFE_UNABLE_TO_PARSE_ADDRESS: u64 = 0x2;
}

pub fn all_natives(
    framework_addr: AccountAddress,
    builder: &SafeNativeBuilder,
    inject_create_signer_for_gov_sim: bool,
) -> NativeFunctionTable {
    let mut natives = vec![];

    macro_rules! add_natives_from_module {
        ($module_name:expr_2021, $natives:expr_2021) => {
            natives.extend(
                $natives.map(|(func_name, func)| ($module_name.to_string(), func_name, func)),
            );
        };
    }

    add_natives_from_module!("account", account::make_all(builder));
    add_natives_from_module!("create_signer", create_signer::make_all(builder));
    add_natives_from_module!("ed25519", ed25519::make_all(builder));
    add_natives_from_module!("crypto_algebra", cryptography::algebra::make_all(builder));
    add_natives_from_module!("genesis", create_signer::make_all(builder));
    add_natives_from_module!("multi_ed25519", multi_ed25519::make_all(builder));
    add_natives_from_module!("bls12381", cryptography::bls12381::make_all(builder));
    add_natives_from_module!("secp256k1", cryptography::secp256k1::make_all(builder));
    add_natives_from_module!("aptos_hash", hash::make_all(builder));
    add_natives_from_module!(
        "ristretto255",
        cryptography::ristretto255::make_all(builder)
    );
    add_natives_from_module!("type_info", type_info::make_all(builder));
    add_natives_from_module!("util", util::make_all(builder));
    add_natives_from_module!("from_bcs", util::make_all(builder));
    add_natives_from_module!("randomness", randomness::make_all(builder));
    add_natives_from_module!(
        "ristretto255_bulletproofs",
        cryptography::bulletproofs::make_all(builder)
    );
    add_natives_from_module!(
        "transaction_context",
        transaction_context::make_all(builder)
    );
    add_natives_from_module!("code", code::make_all(builder));
    add_natives_from_module!("event", event::make_all(builder));
    add_natives_from_module!("state_storage", state_storage::make_all(builder));
    add_natives_from_module!("aggregator", aggregator::make_all(builder));
    add_natives_from_module!("aggregator_factory", aggregator_factory::make_all(builder));
    add_natives_from_module!("aggregator_v2", aggregator_v2::make_all(builder));
    add_natives_from_module!("object", object::make_all(builder));
    add_natives_from_module!("debug", debug::make_all(builder));
    add_natives_from_module!("string_utils", string_utils::make_all(builder));
    add_natives_from_module!("consensus_config", consensus_config::make_all(builder));
    add_natives_from_module!("function_info", function_info::make_all(builder));
    add_natives_from_module!(
        "dispatchable_fungible_asset",
        dispatchable_fungible_asset::make_all(builder)
    );
    add_natives_from_module!(
        "permissioned_signer",
        permissioned_signer::make_all(builder)
    );
    add_natives_from_module!(
        "account_abstraction",
        account_abstraction::make_all(builder)
    );

    if inject_create_signer_for_gov_sim {
        add_natives_from_module!(
            "aptos_governance",
            builder.make_named_natives([(
                "create_signer",
                create_signer::native_create_signer as RawSafeNative
            )])
        );
    }

    make_table_from_iter(framework_addr, natives)
}
