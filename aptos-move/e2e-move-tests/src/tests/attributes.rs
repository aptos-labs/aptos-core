// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, assert_vm_status, MoveHarness};
use aptos_cached_packages::aptos_stdlib;
use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_package_builder::PackageBuilder;
use aptos_types::account_address::AccountAddress;
use move_binary_format::CompiledModule;
use move_core_types::{metadata::Metadata, vm_status::StatusCode};
use serde::Serialize;
use std::collections::BTreeMap;

#[test]
fn test_view_attribute() {
    let mut h = MoveHarness::new();
    let account = h.new_account_at(AccountAddress::from_hex_literal("0xf00d").unwrap());

    let mut builder = PackageBuilder::new("Package");
    builder.add_source(
        "m.move",
        r#"
        module 0xf00d::M {
            #[view]
            fun view(value: u64): u64 { value }
        }
        "#,
    );
    let path = builder.write_to_temp().unwrap();
    assert_success!(h.publish_package(&account, path.path()));
}

#[test]
#[should_panic]
fn test_view_attribute_on_non_view() {
    let mut h = MoveHarness::new();
    let account = h.new_account_at(AccountAddress::from_hex_literal("0xf00d").unwrap());

    let mut builder = PackageBuilder::new("Package");
    builder.add_source(
        "m.move",
        r#"
        module 0xf00d::M {
            #[view]
            fun view(_value: u64) { }
        }
        "#,
    );
    let path = builder.write_to_temp().unwrap();
    assert_success!(h.publish_package(&account, path.path()));
}

#[test]
fn test_bad_attribute_in_code() {
    let mut h = MoveHarness::new();
    let account = h.new_account_at(AccountAddress::from_hex_literal("0xf00d").unwrap());

    let mut builder = PackageBuilder::new("Package");
    builder.add_source(
        "m.move",
        r#"
        module 0xf00d::M {
            #[not_an_attribute]
            fun view(value: u64): u64 { value }
        }
        "#,
    );
    let path = builder.write_to_temp().unwrap();

    // unknown attributes are not compiled into the code
    assert_success!(h.publish_package(&account, path.path()));
}

#[test]
fn test_bad_fun_attribute_in_compiled_module() {
    let mut h = MoveHarness::new();
    let account = h.new_account_at(AccountAddress::from_hex_literal("0xf00d").unwrap());

    let mut builder = PackageBuilder::new("Package");
    builder.add_source(
        "m.move",
        r#"
        module 0xf00d::M {
            fun view(value: u64): u64 { value }
        }
        "#,
    );
    let path = builder.write_to_temp().unwrap();

    let package = BuiltPackage::build(path.path().to_path_buf(), BuildOptions::default())
        .expect("building package must succeed");
    let code = package.extract_code();
    // There should only be the above module
    assert!(code.len() == 1);
    let mut compiled_module = CompiledModule::deserialize(&code[0]).unwrap();

    let mut value = aptos_framework::RuntimeModuleMetadataV1 {
        error_map: BTreeMap::new(),
        struct_attributes: BTreeMap::new(),
        fun_attributes: BTreeMap::new(),
    };
    let fake_attribute = bcs::to_bytes(&FakeKnownAttribute {
        kind: 5,
        args: vec![],
    })
    .unwrap();
    let known_attribute =
        bcs::from_bytes::<aptos_framework::KnownAttribute>(&fake_attribute).unwrap();
    value
        .fun_attributes
        .insert("view".to_string(), vec![known_attribute]);

    let metadata = Metadata {
        key: aptos_framework::APTOS_METADATA_KEY_V1.to_vec(),
        value: bcs::to_bytes(&value).unwrap(),
    };

    let mut code = vec![];
    compiled_module.metadata = vec![metadata];
    compiled_module.serialize(&mut code).unwrap();
    let metadata = package
        .extract_metadata()
        .expect("extracting package metadata must succeed");
    let result = h.run_transaction_payload(
        &account,
        aptos_stdlib::code_publish_package_txn(
            bcs::to_bytes(&metadata).expect("PackageMetadata has BCS"),
            vec![code],
        ),
    );

    assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED);
}

#[test]
fn test_bad_view_attribute_in_compiled_module() {
    let mut h = MoveHarness::new();
    let account = h.new_account_at(AccountAddress::from_hex_literal("0xf00d").unwrap());

    let mut builder = PackageBuilder::new("Package");
    builder.add_source(
        "m.move",
        r#"
        module 0xf00d::M {
            fun view(_value: u64) { }
        }
        "#,
    );
    let path = builder.write_to_temp().unwrap();

    let package = BuiltPackage::build(path.path().to_path_buf(), BuildOptions::default())
        .expect("building package must succeed");
    let code = package.extract_code();
    // There should only be the above module
    assert!(code.len() == 1);
    let mut compiled_module = CompiledModule::deserialize(&code[0]).unwrap();
    let mut value = aptos_framework::RuntimeModuleMetadataV1 {
        error_map: BTreeMap::new(),
        struct_attributes: BTreeMap::new(),
        fun_attributes: BTreeMap::new(),
    };
    let fake_attribute = bcs::to_bytes(&FakeKnownAttribute {
        kind: 1,
        args: vec![],
    })
    .unwrap();
    let known_attribute =
        bcs::from_bytes::<aptos_framework::KnownAttribute>(&fake_attribute).unwrap();
    value
        .fun_attributes
        .insert("view".to_string(), vec![known_attribute]);

    let metadata = Metadata {
        key: aptos_framework::APTOS_METADATA_KEY_V1.to_vec(),
        value: bcs::to_bytes(&value).unwrap(),
    };

    compiled_module.metadata = vec![metadata];
    let mut code = vec![];
    compiled_module.serialize(&mut code).unwrap();
    let metadata = package
        .extract_metadata()
        .expect("extracting package metadata must succeed");
    let result = h.run_transaction_payload(
        &account,
        aptos_stdlib::code_publish_package_txn(
            bcs::to_bytes(&metadata).expect("PackageMetadata has BCS"),
            vec![code],
        ),
    );

    assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED);
}

// We need this because we cannot produce a KnownAttribute directly.
#[derive(Serialize)]
pub struct FakeKnownAttribute {
    kind: u16,
    args: Vec<String>,
}
