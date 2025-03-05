// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Note[Orderless]: Done
use crate::{assert_success, assert_vm_status, build_package, MoveHarness};
use aptos_cached_packages::aptos_stdlib;
use aptos_framework::BuildOptions;
use aptos_package_builder::PackageBuilder;
use aptos_types::{account_address::AccountAddress, on_chain_config::FeatureFlag};
use move_binary_format::CompiledModule;
use move_core_types::{metadata::Metadata, vm_status::StatusCode};
use serde::Serialize;
use std::collections::BTreeMap;
use rstest::rstest;

#[rstest(stateless_account, use_txn_payload_v2_format, use_orderless_transactions, 
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true),
)]
fn test_view_attribute(stateless_account: bool, use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    let account = h.new_account_at(AccountAddress::from_hex_literal("0xf00d").unwrap(), if stateless_account { None } else { Some(0) });

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

#[rstest(stateless_account, use_txn_payload_v2_format, use_orderless_transactions, 
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true),
)]
#[should_panic]
fn test_view_attribute_with_signer(stateless_account: bool, use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    let seq_num = if stateless_account { None } else { Some(0) };
    let account = h.new_account_at(AccountAddress::from_hex_literal("0xf00d").unwrap(), seq_num);

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

#[rstest(stateless_account, use_txn_payload_v2_format, use_orderless_transactions, 
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true),
)]
#[should_panic]
fn test_view_attribute_with_ref_signer(stateless_account: bool, use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    let seq_num = if stateless_account { None } else { Some(0) };
    let account = h.new_account_at(AccountAddress::from_hex_literal("0xf00d").unwrap(), seq_num);

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

#[rstest(stateless_account, use_txn_payload_v2_format, use_orderless_transactions, 
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true),
)]
#[should_panic]
fn test_view_attribute_with_mut_ref_signer(stateless_account: bool, use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    let seq_num = if stateless_account { None } else { Some(0) };
    let account = h.new_account_at(AccountAddress::from_hex_literal("0xf00d").unwrap(), seq_num);

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

#[rstest(stateless_account, use_txn_payload_v2_format, use_orderless_transactions, 
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true),
)]
#[should_panic]
fn test_view_attribute_on_non_view(stateless_account: bool, use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    let seq_num = if stateless_account { None } else { Some(0) };
    let account = h.new_account_at(AccountAddress::from_hex_literal("0xf00d").unwrap(), seq_num);

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

#[rstest(stateless_account, use_txn_payload_v2_format, use_orderless_transactions, 
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true),
)]
fn test_bad_attribute_in_code(stateless_account: bool, use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    let seq_num = if stateless_account { None } else { Some(0) };
    let account = h.new_account_at(AccountAddress::from_hex_literal("0xf00d").unwrap(), seq_num);

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

#[rstest(stateless_account, use_txn_payload_v2_format, use_orderless_transactions, 
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true),
)]
fn test_bad_fun_attribute_in_compiled_module(stateless_account: bool, use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    let seq_num = if stateless_account { None } else { Some(0) };
    let account = h.new_account_at(AccountAddress::from_hex_literal("0xf00d").unwrap(), seq_num);

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

    let package = build_package(path.path().to_path_buf(), BuildOptions::default())
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

#[rstest(stateless_account, use_txn_payload_v2_format, use_orderless_transactions, 
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true),
)]
fn test_bad_view_attribute_in_compiled_module(stateless_account: bool, use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    let seq_num = if stateless_account { None } else { Some(0) };
    let account = h.new_account_at(AccountAddress::from_hex_literal("0xf00d").unwrap(), seq_num);
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
        aptos_stdlib::code_publish_package_txn(metadata, code),
    );

    assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED);
}

#[rstest(stateless_account, use_txn_payload_v2_format, use_orderless_transactions, 
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true),
)]
fn verify_resource_group_member_fails_when_not_enabled(stateless_account: bool, use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    h.enable_features(vec![], vec![FeatureFlag::RESOURCE_GROUPS]);
    let seq_num = if stateless_account { None } else { Some(0) };
    let account = h.new_account_at(AccountAddress::from_hex_literal("0xf00d").unwrap(), seq_num);
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
        aptos_stdlib::code_publish_package_txn(metadata, code),
    );

    assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED);
}

#[rstest(stateless_account, use_txn_payload_v2_format, use_orderless_transactions, 
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true),
)]
fn verify_resource_groups_fail_when_not_enabled(stateless_account: bool, use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    h.enable_features(vec![], vec![FeatureFlag::RESOURCE_GROUPS]);
    let seq_num = if stateless_account { None } else { Some(0) };
    let account = h.new_account_at(AccountAddress::from_hex_literal("0xf00d").unwrap(), seq_num);
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
        aptos_stdlib::code_publish_package_txn(metadata, code),
    );

    assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED);
}

#[rstest(stateless_account, use_txn_payload_v2_format, use_orderless_transactions, 
    case(true, false, false),
    case(true, true, false),
    case(true, true, true),
    case(false, false, false),
    case(false, true, false),
    case(false, true, true),
)]
fn verify_module_events_fail_when_not_enabled(stateless_account: bool, use_txn_payload_v2_format: bool, use_orderless_transactions: bool) {
    let mut h = MoveHarness::new_with_flags(use_txn_payload_v2_format, use_orderless_transactions);
    h.enable_features(vec![], vec![FeatureFlag::MODULE_EVENT]);
    let seq_num = if stateless_account { None } else { Some(0) };
    let account = h.new_account_at(AccountAddress::from_hex_literal("0xf00d").unwrap(), seq_num);
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
        aptos_stdlib::code_publish_package_txn(metadata, code),
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

    let package = build_package(path.path().to_path_buf(), BuildOptions::default())
        .expect("building package must succeed");
    let code = package.extract_code();
    // There should only be one module
    assert!(code.len() == 1);
    let mut compiled_module = CompiledModule::deserialize(&code[0]).unwrap();
    let mut value = aptos_framework::RuntimeModuleMetadataV1 {
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
        key: aptos_framework::APTOS_METADATA_KEY_V1.to_vec(),
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
