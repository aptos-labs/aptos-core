// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, assert_vm_status, tests::common, MoveHarness};
use aptos_package_builder::PackageBuilder;
use aptos_types::{account_address::AccountAddress, on_chain_config::FeatureFlag};
use move_core_types::{identifier::Identifier, language_storage::StructTag, vm_status::StatusCode};
use serde::Deserialize;

#[derive(Debug, Deserialize, Eq, PartialEq)]
struct Primary {
    value: u64,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
struct Secondary {
    value: u32,
}

#[test]
fn test_resource_groups() {
    let mut h = MoveHarness::new_with_features(vec![FeatureFlag::RESOURCE_GROUPS], vec![]);

    let primary_addr = AccountAddress::from_hex_literal("0xcafe").unwrap();
    let primary_account = h.new_account_at(primary_addr);
    let secondary_addr = AccountAddress::from_hex_literal("0xf00d").unwrap();
    let secondary_account = h.new_account_at(secondary_addr);
    let user_addr = AccountAddress::from_hex_literal("0x0123").unwrap();
    let user_account = h.new_account_at(user_addr);

    let mut build_options = aptos_framework::BuildOptions::default();
    build_options
        .named_addresses
        .insert("resource_groups_primary".to_string(), primary_addr);

    let result = h.publish_package_with_options(
        &primary_account,
        &common::test_dir_path("../../../move-examples/resource_groups/primary"),
        build_options.clone(),
    );
    assert_success!(result);

    build_options
        .named_addresses
        .insert("resource_groups_secondary".to_string(), secondary_addr);
    let result = h.publish_package_with_options(
        &secondary_account,
        &common::test_dir_path("../../../move-examples/resource_groups/secondary"),
        build_options,
    );
    assert_success!(result);

    let group_tag = StructTag {
        address: primary_addr,
        module: Identifier::new("primary").unwrap(),
        name: Identifier::new("ResourceGroupContainer").unwrap(),
        type_params: vec![],
    };
    let primary_tag = StructTag {
        address: primary_addr,
        module: Identifier::new("primary").unwrap(),
        name: Identifier::new("Primary").unwrap(),
        type_params: vec![],
    };
    let secondary_tag = StructTag {
        address: secondary_addr,
        module: Identifier::new("secondary").unwrap(),
        name: Identifier::new("Secondary").unwrap(),
        type_params: vec![],
    };

    // Assert that no data exists yet
    assert!(h.read_resource_raw(&user_addr, group_tag.clone()).is_none());
    assert!(h
        .read_resource_raw(&user_addr, primary_tag.clone())
        .is_none());
    assert!(h
        .read_resource_raw(&user_addr, secondary_tag.clone())
        .is_none());

    // Initialize secondary and verify it exists and nothing else does
    let result = h.run_entry_function(
        &user_account,
        str::parse(&format!("0x{}::secondary::init", secondary_addr)).unwrap(),
        vec![],
        vec![bcs::to_bytes::<u32>(&22).unwrap()],
    );
    assert_success!(result);

    let secondary = h
        .read_resource_from_resource_group::<Secondary>(
            &user_addr,
            group_tag.clone(),
            secondary_tag.clone(),
        )
        .unwrap();
    assert_eq!(secondary.value, 22);
    let primary = h.read_resource_from_resource_group::<Primary>(
        &user_addr,
        group_tag.clone(),
        primary_tag.clone(),
    );
    assert!(primary.is_none());
    assert!(h.read_resource_raw(&user_addr, group_tag.clone()).is_none());
    assert!(h
        .read_resource_raw(&user_addr, primary_tag.clone())
        .is_none());
    assert!(h
        .read_resource_raw(&user_addr, secondary_tag.clone())
        .is_none());

    // Initialize primary and verify it exists with secondary
    let result = h.run_entry_function(
        &user_account,
        str::parse(&format!("0x{}::primary::init", primary_addr)).unwrap(),
        vec![],
        vec![bcs::to_bytes::<u64>(&11122).unwrap()],
    );
    assert_success!(result);

    let secondary = h
        .read_resource_from_resource_group::<Secondary>(
            &user_addr,
            group_tag.clone(),
            secondary_tag.clone(),
        )
        .unwrap();
    assert_eq!(secondary.value, 22);
    let primary = h
        .read_resource_from_resource_group::<Primary>(
            &user_addr,
            group_tag.clone(),
            primary_tag.clone(),
        )
        .unwrap();
    assert_eq!(primary.value, 11122);
    assert!(h.read_resource_raw(&user_addr, group_tag.clone()).is_none());
    assert!(h
        .read_resource_raw(&user_addr, primary_tag.clone())
        .is_none());
    assert!(h
        .read_resource_raw(&user_addr, secondary_tag.clone())
        .is_none());

    // Modify secondary and verify primary stays consistent
    let result = h.run_entry_function(
        &user_account,
        str::parse(&format!("0x{}::secondary::set_value", secondary_addr)).unwrap(),
        vec![],
        vec![bcs::to_bytes::<u32>(&5).unwrap()],
    );
    assert_success!(result);

    let secondary = h
        .read_resource_from_resource_group::<Secondary>(
            &user_addr,
            group_tag.clone(),
            secondary_tag.clone(),
        )
        .unwrap();
    assert_eq!(secondary.value, 5);
    let primary = h
        .read_resource_from_resource_group::<Primary>(
            &user_addr,
            group_tag.clone(),
            primary_tag.clone(),
        )
        .unwrap();
    assert_eq!(primary.value, 11122);
    assert!(h.read_resource_raw(&user_addr, group_tag.clone()).is_none());
    assert!(h
        .read_resource_raw(&user_addr, primary_tag.clone())
        .is_none());
    assert!(h
        .read_resource_raw(&user_addr, secondary_tag.clone())
        .is_none());

    // Delete the first and verify the second remains
    let result = h.run_entry_function(
        &user_account,
        str::parse(&format!("0x{}::primary::remove", primary_addr)).unwrap(),
        vec![],
        vec![],
    );
    assert_success!(result);

    let secondary = h
        .read_resource_from_resource_group::<Secondary>(
            &user_addr,
            group_tag.clone(),
            secondary_tag.clone(),
        )
        .unwrap();
    assert_eq!(secondary.value, 5);
    let primary = h.read_resource_from_resource_group::<Primary>(
        &user_addr,
        group_tag.clone(),
        primary_tag.clone(),
    );
    assert!(primary.is_none());
    assert!(h.read_resource_raw(&user_addr, group_tag.clone()).is_none());
    assert!(h
        .read_resource_raw(&user_addr, primary_tag.clone())
        .is_none());
    assert!(h
        .read_resource_raw(&user_addr, secondary_tag.clone())
        .is_none());

    // Delete the second and verify nothing remains
    let result = h.run_entry_function(
        &user_account,
        str::parse(&format!("0x{}::secondary::remove", secondary_addr)).unwrap(),
        vec![],
        vec![],
    );
    assert_success!(result);

    assert!(h
        .read_resource_group(&user_addr, group_tag.clone())
        .is_none());
    assert!(h.read_resource_raw(&user_addr, group_tag).is_none());
    assert!(h.read_resource_raw(&user_addr, primary_tag).is_none());
    assert!(h.read_resource_raw(&user_addr, secondary_tag).is_none());
}

#[test]
fn test_resource_groups_container_not_enabled() {
    let mut h = MoveHarness::new_with_features(vec![FeatureFlag::RESOURCE_GROUPS], vec![]);

    let primary_addr = AccountAddress::from_hex_literal("0xcafe").unwrap();
    let primary_account = h.new_account_at(primary_addr);

    let mut build_options = aptos_framework::BuildOptions::default();
    build_options
        .named_addresses
        .insert("resource_groups_primary".to_string(), primary_addr);

    let result = h.publish_package_with_options(
        &primary_account,
        &common::test_dir_path("../../../move-examples/resource_groups/primary"),
        build_options.clone(),
    );
    assert_success!(result);
}

#[test]
fn verify_resource_group_upgrades() {
    let mut h = MoveHarness::new_with_features(vec![FeatureFlag::RESOURCE_GROUPS], vec![]);
    let account = h.new_account_at(AccountAddress::from_hex_literal("0xf00d").unwrap());

    // Initial code
    let source = r#"
        module 0xf00d::M {
            #[resource_group_member(group = 0xf00d::M::ResourceGroup)]
            struct ResourceGroupMember has key { }

            #[resource_group(scope = address)]
            struct ResourceGroup { }

            #[resource_group(scope = address)]
            struct ResourceGroupExtra { }
        }
        "#;
    let mut builder = PackageBuilder::new("Package");
    builder.add_source("m.move", source);
    let path = builder.write_to_temp().unwrap();
    let result = h.publish_package(&account, path.path());
    assert_success!(result);

    // Compatible increase on scope
    let source = r#"
        module 0xf00d::M {
            #[resource_group_member(group = 0xf00d::M::ResourceGroup)]
            struct ResourceGroupMember has key { }

            #[resource_group(scope = global)]
            struct ResourceGroup { }

            #[resource_group(scope = address)]
            struct ResourceGroupExtra { }
        }
        "#;
    let mut builder = PackageBuilder::new("Package");
    builder.add_source("m.move", source);
    let path = builder.write_to_temp().unwrap();
    let result = h.publish_package(&account, path.path());
    assert_success!(result);

    // Incompatible decrease on scope
    let source = r#"
        module 0xf00d::M {
            #[resource_group_member(group = 0xf00d::M::ResourceGroup)]
            struct ResourceGroupMember has key { }

            #[resource_group(scope = module_)]
            struct ResourceGroup { }

            #[resource_group(scope = address)]
            struct ResourceGroupExtra { }
        }
        "#;
    let mut builder = PackageBuilder::new("Package");
    builder.add_source("m.move", source);
    let path = builder.write_to_temp().unwrap();
    let result = h.publish_package(&account, path.path());
    assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED);

    // Removal of ResourceGroupContainer is incompatible
    let source = r#"
        module 0xf00d::M {
            struct ResourceGroupMember has key { }

            #[resource_group(scope = global)]
            struct ResourceGroup { }

            struct ResourceGroupExtra { }
        }
        "#;
    let mut builder = PackageBuilder::new("Package");
    builder.add_source("m.move", source);
    let path = builder.write_to_temp().unwrap();
    let result = h.publish_package(&account, path.path());
    assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED);

    // Change of ResourceGroupMember::group is incompatible
    let source = r#"
        module 0xf00d::M {
            #[resource_group_member(group = 0xf00d::M::ResourceGroupExtra)]
            struct ResourceGroupMember has key { }

            #[resource_group(scope = global)]
            struct ResourceGroup { }

            #[resource_group(scope = address)]
            struct ResourceGroupExtra { }
        }
        "#;
    let mut builder = PackageBuilder::new("Package");
    builder.add_source("m.move", source);
    let path = builder.write_to_temp().unwrap();
    let result = h.publish_package(&account, path.path());
    assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED);
}
