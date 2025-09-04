// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(unused_imports)]

pub use crate::{
    velor_framework_sdk_builder::*, velor_token_objects_sdk_builder as velor_token_objects_stdlib,
    velor_token_sdk_builder as velor_token_stdlib,
};
use velor_framework::{BuildOptions, BuiltPackage};
use velor_package_builder::PackageBuilder;
use velor_types::{
    account_address::AccountAddress,
    transaction::{EntryFunction, TransactionPayload},
    VelorCoinType, CoinType,
};
use move_core_types::{ident_str, language_storage::ModuleId};

pub fn velor_coin_transfer(to: AccountAddress, amount: u64) -> TransactionPayload {
    coin_transfer(VelorCoinType::type_tag(), to, amount)
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
