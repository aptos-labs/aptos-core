// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_vm_status, build_package, MoveHarness};
use aptos_cached_packages::aptos_stdlib;
use aptos_framework::{
    BuildOptions, RuntimeModuleMetadata, RuntimeModuleMetadataV1, APTOS_METADATA_KEY,
    APTOS_METADATA_KEY_V1,
};
use aptos_package_builder::PackageBuilder;
use aptos_types::transaction::TransactionStatus;
use move_binary_format::CompiledModule;
use move_core_types::{account_address::AccountAddress, metadata::Metadata, vm_status::StatusCode};
use std::collections::BTreeMap;

#[test]
fn test_unknown_metadata_key() {
    let unknown_key = || {
        let metadata = Metadata {
            key: vec![1, 2, 3, 4, 5],
            value: vec![],
        };
        vec![metadata]
    };
    let result = test_metadata_with_changes(unknown_key);
    assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED);
}

#[test]
fn test_duplicate_entries() {
    let duplicate_same_version = || {
        let metadata = Metadata {
            key: APTOS_METADATA_KEY_V1.to_vec(),
            value: bcs::to_bytes(&RuntimeModuleMetadataV1::default()).unwrap(),
        };
        vec![metadata.clone(), metadata]
    };
    let result = test_metadata_with_changes(duplicate_same_version);
    assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED);

    let duplicate_different_version = || {
        let metadata_v1 = Metadata {
            key: APTOS_METADATA_KEY_V1.to_vec(),
            value: bcs::to_bytes(&RuntimeModuleMetadataV1::default()).unwrap(),
        };
        let metadata_v0 = Metadata {
            key: APTOS_METADATA_KEY.to_vec(),
            value: bcs::to_bytes(&RuntimeModuleMetadata {
                error_map: BTreeMap::new(),
            })
            .unwrap(),
        };
        vec![metadata_v1, metadata_v0]
    };

    let result = test_metadata_with_changes(duplicate_different_version);
    assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED);
}

#[test]
fn test_malformed_metadata_value() {
    let invalid_value = || {
        let metadata = Metadata {
            key: APTOS_METADATA_KEY.to_vec(),
            value: vec![1, 2, 3],
        };
        vec![metadata]
    };
    let result = test_metadata_with_changes(invalid_value);
    assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED);

    let v0_to_v1 = || {
        let metadata = Metadata {
            key: APTOS_METADATA_KEY.to_vec(),
            value: bcs::to_bytes(&RuntimeModuleMetadataV1::default()).unwrap(),
        };
        vec![metadata]
    };
    let result = test_metadata_with_changes(v0_to_v1);
    assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED);

    let v1_to_v0 = || {
        let metadata = Metadata {
            key: APTOS_METADATA_KEY_V1.to_vec(),
            value: bcs::to_bytes(&RuntimeModuleMetadata {
                error_map: BTreeMap::new(),
            })
            .unwrap(),
        };
        vec![metadata]
    };
    let result = test_metadata_with_changes(v1_to_v0);
    assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED);
}

fn test_metadata_with_changes(f: impl Fn() -> Vec<Metadata>) -> TransactionStatus {
    let mut h = MoveHarness::new();
    let account = h.new_account_at(AccountAddress::from_hex_literal("0xf00d").unwrap());

    let mut builder = PackageBuilder::new("Package");
    builder.add_source(
        "m.move",
        r#"
        module 0xf00d::M {
            #[view]
            fun foo(value: u64): u64 { value }
        }
        "#,
    );
    let path = builder.write_to_temp().unwrap();

    let package = build_package(path.path().to_path_buf(), BuildOptions::default())
        .expect("building package must succeed");
    let origin_code = package.extract_code();
    let mut compiled_module = CompiledModule::deserialize(&origin_code[0]).unwrap();
    let metadata = f();
    let mut invalid_code = vec![];
    compiled_module.metadata = metadata;
    compiled_module.serialize(&mut invalid_code).unwrap();

    let package_metadata = package
        .extract_metadata()
        .expect("extracting package metadata must succeed");
    h.run_transaction_payload(
        &account,
        aptos_stdlib::code_publish_package_txn(
            bcs::to_bytes(&package_metadata).expect("PackageMetadata has BCS"),
            vec![invalid_code],
        ),
    )
}
