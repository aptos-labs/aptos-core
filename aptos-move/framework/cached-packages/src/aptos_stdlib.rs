// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(unused_imports)]

pub use crate::{
    aptos_framework_sdk_builder::*, aptos_token_objects_sdk_builder as aptos_token_objects_stdlib,
    aptos_token_sdk_builder as aptos_token_stdlib,
};
use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_package_builder::PackageBuilder;
use aptos_types::{
    account_address::AccountAddress,
    transaction::{
        EntryFunction, TransactionExecutable, TransactionExtraConfig, TransactionPayload,
        TransactionPayloadV2,
    },
    AptosCoinType, CoinType,
};
use move_core_types::{
    ident_str,
    language_storage::{ModuleId, TypeTag},
};

pub fn aptos_coin_transfer(to: AccountAddress, amount: u64) -> TransactionPayload {
    coin_transfer(AptosCoinType::type_tag(), to, amount)
}

pub fn coin_transfer_v2(
    coin_type: TypeTag,
    to: AccountAddress,
    amount: u64,
    replay_protection_nonce: Option<u64>,
) -> TransactionPayload {
    TransactionPayload::V2(TransactionPayloadV2::V1 {
        executable: TransactionExecutable::EntryFunction(EntryFunction::new(
            ModuleId::new(
                AccountAddress::new([
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 1,
                ]),
                ident_str!("coin").to_owned(),
            ),
            ident_str!("transfer").to_owned(),
            vec![coin_type],
            vec![bcs::to_bytes(&to).unwrap(), bcs::to_bytes(&amount).unwrap()],
        )),
        extra_config: TransactionExtraConfig::V1 {
            replay_protection_nonce,
            multisig_address: None,
        },
    })
}

pub fn aptos_account_fungible_transfer_only_v2(
    to: AccountAddress,
    amount: u64,
    replay_protection_nonce: Option<u64>,
) -> TransactionPayload {
    TransactionPayload::V2(TransactionPayloadV2::V1 {
        executable: TransactionExecutable::EntryFunction(EntryFunction::new(
            ModuleId::new(
                AccountAddress::new([
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 1,
                ]),
                ident_str!("aptos_account").to_owned(),
            ),
            ident_str!("fungible_transfer_only").to_owned(),
            vec![],
            vec![bcs::to_bytes(&to).unwrap(), bcs::to_bytes(&amount).unwrap()],
        )),
        extra_config: TransactionExtraConfig::V1 {
            replay_protection_nonce,
            multisig_address: None,
        },
    })
}

pub fn aptos_coin_transfer_v2(
    to: AccountAddress,
    amount: u64,
    nonce: Option<u64>,
) -> TransactionPayload {
    coin_transfer_v2(AptosCoinType::type_tag(), to, amount, nonce)
}

pub fn publish_module_source(module_name: &str, module_src: &str) -> TransactionPayload {
    let mut builder = PackageBuilder::new("tmp");
    builder.add_source(module_name, module_src);

    let tmp_dir = builder.write_to_temp().unwrap();
    let package = BuiltPackage::build(tmp_dir.path().to_path_buf(), BuildOptions::default())
        .expect("Should be able to build a package");
    let code = package.extract_code();
    let metadata = package
        .extract_metadata()
        .expect("Should be able to extract metadata");
    let metadata_serialized =
        bcs::to_bytes(&metadata).expect("Should be able to serialize metadata");
    code_publish_package_txn(metadata_serialized, code)
}

/// Temporary workaround as `Object<T>` as a function argument is not recognised
/// when auto generating move transaction payloads. Will address in separate PR.
pub fn object_code_deployment_upgrade(
    metadata_serialized: Vec<u8>,
    code: Vec<Vec<u8>>,
    code_object: AccountAddress,
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(
            AccountAddress::new([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 1,
            ]),
            ident_str!("object_code_deployment").to_owned(),
        ),
        ident_str!("upgrade").to_owned(),
        vec![],
        vec![
            bcs::to_bytes(&metadata_serialized).unwrap(),
            bcs::to_bytes(&code).unwrap(),
            bcs::to_bytes(&code_object).unwrap(),
        ],
    ))
}

/// Temporary workaround as `Object<T>` as a function argument is not recognised
/// when auto generating move transaction payloads. Will address in separate PR.
pub fn object_code_deployment_freeze_code_object(
    code_object: AccountAddress,
) -> TransactionPayload {
    TransactionPayload::EntryFunction(EntryFunction::new(
        ModuleId::new(
            AccountAddress::new([
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 1,
            ]),
            ident_str!("object_code_deployment").to_owned(),
        ),
        ident_str!("freeze_code_object").to_owned(),
        vec![],
        vec![bcs::to_bytes(&code_object).unwrap()],
    ))
}
