// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, assert_vm_status, MoveHarness};
use velor_cached_packages::velor_stdlib;
use velor_framework::{BuildOptions, BuiltPackage};
use velor_package_builder::PackageBuilder;
use velor_types::{
    account_address::AccountAddress,
    on_chain_config::FeatureFlag,
    vm::module_metadata::{KnownAttribute, RuntimeModuleMetadataV1, VELOR_METADATA_KEY_V1},
};
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
fn test_view_attribute_with_signer() {
    let mut h = MoveHarness::new();
    let account = h.new_account_at(AccountAddress::from_hex_literal("0xf00d").unwrap());

    let mut builder = PackageBuilder::new("Package");
    builder.add_source(
        "m.move",
        r#"
        module 0xf00d::M {
            #[view]
            fun view(_:signer,value: u64): u64 { value }
        }
        "#,
    );
    let path = builder.write_to_temp().unwrap();
    h.create_publish_package(&account, path.path(), None, |_| {});
}

#[test]
#[should_panic]
fn test_view_attribute_with_ref_signer() {
    let mut h = MoveHarness::new();
    let account = h.new_account_at(AccountAddress::from_hex_literal("0xf00d").unwrap());

    let mut builder = PackageBuilder::new("Package");
    builder.add_source(
        "m.move",
        r#"
        module 0xf00d::M {
            #[view]
            fun view(_:&signer,value: u64): u64 { value }
        }
        "#,
    );
    let path = builder.write_to_temp().unwrap();
    h.create_publish_package(&account, path.path(), None, |_| {});
}

#[test]
#[should_panic]
fn test_view_attribute_with_mut_ref_signer() {
    let mut h = MoveHarness::new();
    let account = h.new_account_at(AccountAddress::from_hex_literal("0xf00d").unwrap());

    let mut builder = PackageBuilder::new("Package");
    builder.add_source(
        "m.move",
        r#"
        module 0xf00d::M {
            #[view]
            fun view(_:&mut signer,value: u64): u64 { value }
        }
        "#,
    );
    let path = builder.write_to_temp().unwrap();
    h.create_publish_package(&account, path.path(), None, |_| {});
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

    let mut value = RuntimeModuleMetadataV1 {
        error_map: BTreeMap::new(),
        struct_attributes: BTreeMap::new(),
        fun_attributes: BTreeMap::new(),
    };
    let fake_attribute = bcs::to_bytes(&FakeKnownAttribute {
        kind: 5,
        args: vec![],
    })
    .unwrap();
    let known_attribute = bcs::from_bytes::<KnownAttribute>(&fake_attribute).unwrap();
    value
        .fun_attributes
        .insert("view".to_string(), vec![known_attribute]);

    let metadata = Metadata {
        key: VELOR_METADATA_KEY_V1.to_vec(),
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
        velor_stdlib::code_publish_package_txn(
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
    let source = r#"
        module 0xf00d::M {
            fun view(_value: u64) { }
        }
        "#;
    let fake_attribute = FakeKnownAttribute {
        kind: 1,
        args: vec![],
    };
    let (code, metadata) =
        build_package_and_insert_attribute(source, None, Some(("view", fake_attribute)));
    let result = h.run_transaction_payload(
        &account,
        velor_stdlib::code_publish_package_txn(metadata, code),
    );

    assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED);
}

#[test]
fn verify_resource_group_member_fails_when_not_enabled() {
    let mut h = MoveHarness::new_with_features(vec![], vec![FeatureFlag::RESOURCE_GROUPS]);
    let account = h.new_account_at(AccountAddress::from_hex_literal("0xf00d").unwrap());
    let source = r#"
        module 0xf00d::M {
            struct ResourceGroupMember has key { }
        }
        "#;
    let fake_attribute = FakeKnownAttribute {
        kind: 3,
        args: vec!["0xf00d::M::ResourceGroup".to_string()],
    };
    let (code, metadata) = build_package_and_insert_attribute(
        source,
        Some(("ResourceGroupMember", fake_attribute)),
        None,
    );
    let result = h.run_transaction_payload(
        &account,
        velor_stdlib::code_publish_package_txn(metadata, code),
    );

    assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED);
}

#[test]
fn verify_resource_groups_fail_when_not_enabled() {
    let mut h = MoveHarness::new_with_features(vec![], vec![FeatureFlag::RESOURCE_GROUPS]);
    let account = h.new_account_at(AccountAddress::from_hex_literal("0xf00d").unwrap());
    let source = r#"
        module 0xf00d::M {
            struct ResourceGroup { }
        }
        "#;
    let fake_attribute = FakeKnownAttribute {
        kind: 2,
        args: vec!["address".to_string()],
    };
    let (code, metadata) =
        build_package_and_insert_attribute(source, Some(("ResourceGroup", fake_attribute)), None);
    let result = h.run_transaction_payload(
        &account,
        velor_stdlib::code_publish_package_txn(metadata, code),
    );

    assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED);
}

#[test]
fn verify_module_events_fail_when_not_enabled() {
    let mut h = MoveHarness::new_with_features(vec![], vec![FeatureFlag::MODULE_EVENT]);
    let account = h.new_account_at(AccountAddress::from_hex_literal("0xf00d").unwrap());
    let source = r#"
        module 0xf00d::M {
            struct Event { }
        }
        "#;
    let fake_attribute = FakeKnownAttribute {
        kind: 4,
        args: vec![],
    };
    let (code, metadata) =
        build_package_and_insert_attribute(source, Some(("Event", fake_attribute)), None);
    let result = h.run_transaction_payload(
        &account,
        velor_stdlib::code_publish_package_txn(metadata, code),
    );

    assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED);
}

fn build_package_and_insert_attribute(
    source: &str,
    struct_attr: Option<(&str, FakeKnownAttribute)>,
    func_attr: Option<(&str, FakeKnownAttribute)>,
) -> (Vec<Vec<u8>>, Vec<u8>) {
    let mut builder = PackageBuilder::new("Package");
    builder.add_source("m.move", source);
    let path = builder.write_to_temp().unwrap();

    let package = BuiltPackage::build(path.path().to_path_buf(), BuildOptions::default())
        .expect("building package must succeed");
    let code = package.extract_code();
    // There should only be one module
    assert!(code.len() == 1);
    let mut compiled_module = CompiledModule::deserialize(&code[0]).unwrap();
    let mut value = RuntimeModuleMetadataV1 {
        error_map: BTreeMap::new(),
        struct_attributes: BTreeMap::new(),
        fun_attributes: BTreeMap::new(),
    };

    if let Some((name, attr)) = struct_attr {
        let fake_attribute = bcs::to_bytes(&attr).unwrap();
        let known_attribute = bcs::from_bytes(&fake_attribute).unwrap();
        value
            .struct_attributes
            .insert(name.to_string(), vec![known_attribute]);
    };
    if let Some((name, attr)) = func_attr {
        let fake_attribute = bcs::to_bytes(&attr).unwrap();
        let known_attribute = bcs::from_bytes(&fake_attribute).unwrap();
        value
            .fun_attributes
            .insert(name.to_string(), vec![known_attribute]);
    }

    let metadata = Metadata {
        key: VELOR_METADATA_KEY_V1.to_vec(),
        value: bcs::to_bytes(&value).unwrap(),
    };

    compiled_module.metadata = vec![metadata];
    let mut code = vec![];
    compiled_module.serialize(&mut code).unwrap();
    let metadata = package
        .extract_metadata()
        .expect("extracting package metadata must succeed");
    (vec![code], bcs::to_bytes(&metadata).unwrap())
}

// We need this because we cannot produce a KnownAttribute directly.
#[derive(Serialize)]
pub struct FakeKnownAttribute {
    kind: u8,
    args: Vec<String>,
}
