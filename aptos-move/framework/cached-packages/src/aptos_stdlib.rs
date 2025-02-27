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
    transaction::{EntryFunction, TransactionPayloadWrapper},
    AptosCoinType, CoinType,
};
use move_core_types::{ident_str, language_storage::ModuleId};

pub fn aptos_coin_transfer(to: AccountAddress, amount: u64) -> TransactionPayloadWrapper {
    coin_transfer(AptosCoinType::type_tag(), to, amount)
}

pub fn publish_module_source(module_name: &str, module_src: &str) -> TransactionPayloadWrapper {
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
) -> TransactionPayloadWrapper {
    TransactionPayloadWrapper::EntryFunction(EntryFunction::new(
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
) -> TransactionPayloadWrapper {
    TransactionPayloadWrapper::EntryFunction(EntryFunction::new(
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
