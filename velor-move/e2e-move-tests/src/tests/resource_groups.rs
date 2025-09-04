// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    assert_success, assert_vm_status,
    resource_groups::{
        initialize, initialize_enabled_disabled_comparison, ResourceGroupsTestHarness,
    },
    tests::{aggregator_v2::arb_block_split, common},
    BlockSplit, MoveHarness, SUCCESS,
};
use velor_language_e2e_tests::executor::ExecutorMode;
use velor_package_builder::PackageBuilder;
use velor_types::{account_address::AccountAddress, on_chain_config::FeatureFlag};
use move_core_types::{identifier::Identifier, language_storage::StructTag, vm_status::StatusCode};
use proptest::prelude::*;
use serde::Deserialize;
use test_case::test_case;

// This mode describes whether to enable or disable RESOURCE_GROUPS_SPLIT_IN_VM_CHANGE_SET flag
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ResourceGroupMode {
    EnabledOnly,
    DisabledOnly,
    BothComparison,
}

const STRESSTEST_MODE: bool = false;
const ENOT_EQUAL: u64 = 17;
const EINVALID_ARG: u64 = 18;
const ERESOURCE_DOESNT_EXIST: u64 = 19;

// TODO[agg_v2](cleanup): This interface looks similar to aggregator v2 test harness.
// Could cleanup later on.
fn setup(
    executor_mode: ExecutorMode,
    // This mode describes whether to enable or disable RESOURCE_GROUPS_SPLIT_IN_VM_CHANGE_SET flag
    resource_group_mode: ResourceGroupMode,
    txns: usize,
) -> ResourceGroupsTestHarness {
    let path = common::test_dir_path("resource_groups.data/pack");
    match resource_group_mode {
        ResourceGroupMode::EnabledOnly => initialize(path, executor_mode, true, txns),
        ResourceGroupMode::DisabledOnly => initialize(path, executor_mode, false, txns),
        ResourceGroupMode::BothComparison => {
            initialize_enabled_disabled_comparison(path, executor_mode, txns)
        },
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct TestEnvConfig {
    pub executor_mode: ExecutorMode,
    pub resource_group_mode: ResourceGroupMode,
    pub block_split: BlockSplit,
}

#[allow(clippy::arc_with_non_send_sync)] // I think this is noise, don't see an issue, and tests run fine
fn arb_test_env(num_txns: usize) -> BoxedStrategy<TestEnvConfig> {
    prop_oneof![
        arb_block_split(num_txns).prop_map(|block_split| TestEnvConfig {
            executor_mode: ExecutorMode::BothComparison,
            resource_group_mode: ResourceGroupMode::BothComparison,
            block_split
        }),
    ]
    .boxed()
}

#[allow(clippy::arc_with_non_send_sync)] // I think this is noise, don't see an issue, and tests run fine
fn arb_test_env_non_equivalent(num_txns: usize) -> BoxedStrategy<TestEnvConfig> {
    prop_oneof![
        arb_block_split(num_txns).prop_map(|block_split| TestEnvConfig {
            executor_mode: ExecutorMode::BothComparison,
            resource_group_mode: ResourceGroupMode::DisabledOnly,
            block_split
        }),
        arb_block_split(num_txns).prop_map(|block_split| TestEnvConfig {
            executor_mode: ExecutorMode::BothComparison,
            resource_group_mode: ResourceGroupMode::EnabledOnly,
            block_split
        }),
    ]
    .boxed()
}

proptest! {
    #![proptest_config(ProptestConfig {
        // Cases are expensive, few cases is enough.
        // We will test a few more comprehensive tests more times, and the rest even fewer.
        cases: if STRESSTEST_MODE { 1000 } else { 20 },
        result_cache: if STRESSTEST_MODE { prop::test_runner::noop_result_cache } else {prop::test_runner::basic_result_cache },
        .. ProptestConfig::default()
    })]

    #[test]
    fn proptest_resource_groups_1(test_env in arb_test_env(17)) {
        println!("Testing test_aggregator_lifetime {:?}", test_env);
        let mut h = setup(test_env.executor_mode, test_env.resource_group_mode, 17);

        let txns = vec![
            (SUCCESS, h.init_signer(vec![5,2,3])),
            (SUCCESS, h.set_resource(4, "ABC".to_string(), 10)),
            (SUCCESS, h.set_resource(2, "DEFG".to_string(), 20)),
            (SUCCESS, h.unset_resource(3)),
            (SUCCESS, h.set_resource(3, "GH".to_string(), 30)),
            (SUCCESS, h.set_resource(4, "JKLMNO".to_string(), 40)),
            (SUCCESS, h.set_and_check(2,  4, "MNOP".to_string(), 50, "JKLMNO".to_string(), 40)),
            (SUCCESS, h.read_or_init(1)),
            (SUCCESS, h.check(2, "MNOP".to_string(), 50)),
            (SUCCESS, h.check(1, "init_name".to_string(), 5)),
            (SUCCESS, h.unset_resource(1)),
            (SUCCESS, h.set_3_group_members(1, 2, 3, "L".to_string(), 25)),
            (SUCCESS, h.check(1, "L".to_string(), 25)),
            (SUCCESS, h.unset_resource(3)),
            (ENOT_EQUAL, h.check(2, "MNOP".to_string(), 50)),
            (EINVALID_ARG, h.set_resource(5, "JKLI".to_string(), 40)),
            (ERESOURCE_DOESNT_EXIST, h.check(3, "L".to_string(), 25)),
        ];
        h.run_block_in_parts_and_check(
            test_env.block_split,
            txns,
        );
    }

    #[test]
    fn proptest_resource_groups_2(test_env in arb_test_env_non_equivalent(12)) {
        println!("Testing test_aggregator_lifetime {:?}", test_env);
        let mut h = setup(test_env.executor_mode, test_env.resource_group_mode, 12);

        let txns = vec![
            (SUCCESS, h.init_signer(vec![5,2,3])),
            (SUCCESS, h.set_resource(4, "ABCDEF".to_string(), 10)),
            (SUCCESS, h.set_resource(2, "DEF".to_string(), 20)),
            (SUCCESS, h.read_or_init(4)),
            (SUCCESS, h.set_resource(2, "XYZK".to_string(), 25)),
            (ENOT_EQUAL, h.check(2, "DEF".to_string(), 20)),
            (SUCCESS, h.check(2, "XYZK".to_string(), 25)),
            (SUCCESS, h.set_resource(3, "GH".to_string(), 30)),
            (SUCCESS, h.unset_resource(3)),
            (ERESOURCE_DOESNT_EXIST, h.check(3, "LJH".to_string(), 25)),
            (ERESOURCE_DOESNT_EXIST, h.set_and_check(2,  1, "MNO".to_string(), 50, "GH".to_string(), 30)),
            (SUCCESS, h.check(2, "XYZK".to_string(), 25)),
        ];
        h.run_block_in_parts_and_check(
            test_env.block_split,
            txns,
        );
    }
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
struct Primary {
    value: u64,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
struct Secondary {
    value: u32,
}

#[test_case(true)]
#[test_case(false)]
fn test_resource_groups(resource_group_charge_as_sum_enabled: bool) {
    let mut h = MoveHarness::new();
    if resource_group_charge_as_sum_enabled {
        h.enable_features(
            vec![FeatureFlag::RESOURCE_GROUPS_SPLIT_IN_VM_CHANGE_SET],
            vec![],
        );
    } else {
        h.enable_features(vec![], vec![
            FeatureFlag::RESOURCE_GROUPS_SPLIT_IN_VM_CHANGE_SET,
        ]);
    }

    let primary_addr = AccountAddress::from_hex_literal("0xcafe").unwrap();
    let primary_account = h.new_account_at(primary_addr);
    let secondary_addr = AccountAddress::from_hex_literal("0xf00d").unwrap();
    let secondary_account = h.new_account_at(secondary_addr);
    let user_addr = AccountAddress::from_hex_literal("0x0123").unwrap();
    let user_account = h.new_account_at(user_addr);

    let mut build_options = velor_framework::BuildOptions::default();
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
        type_args: vec![],
    };
    let primary_tag = StructTag {
        address: primary_addr,
        module: Identifier::new("primary").unwrap(),
        name: Identifier::new("Primary").unwrap(),
        type_args: vec![],
    };
    let secondary_tag = StructTag {
        address: secondary_addr,
        module: Identifier::new("secondary").unwrap(),
        name: Identifier::new("Secondary").unwrap(),
        type_args: vec![],
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
        str::parse(&format!("0x{}::secondary::init", secondary_addr.to_hex())).unwrap(),
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
        str::parse(&format!("0x{}::primary::init", primary_addr.to_hex())).unwrap(),
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
        str::parse(&format!(
            "0x{}::secondary::set_value",
            secondary_addr.to_hex()
        ))
        .unwrap(),
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
        str::parse(&format!("0x{}::primary::remove", primary_addr.to_hex())).unwrap(),
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
        str::parse(&format!("0x{}::secondary::remove", secondary_addr.to_hex())).unwrap(),
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
    let mut h = MoveHarness::new_with_features(vec![], vec![FeatureFlag::RESOURCE_GROUPS]);

    let primary_addr = AccountAddress::from_hex_literal("0xcafe").unwrap();
    let primary_account = h.new_account_at(primary_addr);

    let mut build_options = velor_framework::BuildOptions::default();
    build_options
        .named_addresses
        .insert("resource_groups_primary".to_string(), primary_addr);

    let result = h.publish_package_with_options(
        &primary_account,
        &common::test_dir_path("../../../move-examples/resource_groups/primary"),
        build_options.clone(),
    );
    assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED);
}

#[test]
fn verify_resource_group_member_upgrades() {
    let mut h = MoveHarness::new();
    let account = h.new_account_at(AccountAddress::from_hex_literal("0xf00d").unwrap());

    // Initial code
    let source = r#"
        module 0xf00d::M {
            #[resource_group_member(group = 0xf00d::M::ResourceGroup)]
            struct ResourceGroupMember has key { }

            struct NotResourceGroupMember has key { }

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

    // Incompatible change of ResourceGroupMember::group
    let source = r#"
        module 0xf00d::M {
            #[resource_group_member(group = 0xf00d::M::ResourceGroupExtra)]
            struct ResourceGroupMember has key { }

            struct NotResourceGroupMember has key { }

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
    assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED);

    // Incompatible addition of ResourceGroupMember
    let source = r#"
        module 0xf00d::M {
            #[resource_group_member(group = 0xf00d::M::ResourceGroup)]
            struct ResourceGroupMember has key { }

            #[resource_group_member(group = 0xf00d::M::ResourceGroup)]
            struct NotResourceGroupMember has key { }

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
    assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED);
}

#[test]
fn verify_unsafe_resource_group_member_upgrades() {
    let mut h = MoveHarness::new_with_features(vec![], vec![FeatureFlag::SAFER_RESOURCE_GROUPS]);
    let account = h.new_account_at(AccountAddress::from_hex_literal("0xf00d").unwrap());

    // Initial code
    let source = r#"
        module 0xf00d::M {
            struct NotResourceGroupMember has key { }

            #[resource_group(scope = address)]
            struct ResourceGroup { }
        }
        "#;
    let mut builder = PackageBuilder::new("Package");
    builder.add_source("m.move", source);
    let path = builder.write_to_temp().unwrap();
    let result = h.publish_package(&account, path.path());
    assert_success!(result);

    // Incompatible addition of ResourceGroupMember
    let source = r#"
        module 0xf00d::M {
            #[resource_group_member(group = 0xf00d::M::ResourceGroup)]
            struct NotResourceGroupMember has key { }

            #[resource_group(scope = address)]
            struct ResourceGroup { }
        }
        "#;
    let mut builder = PackageBuilder::new("Package");
    builder.add_source("m.move", source);
    let path = builder.write_to_temp().unwrap();
    let result = h.publish_package(&account, path.path());
    assert_success!(result);
}

#[test]
fn verify_resource_group_upgrades() {
    let mut h = MoveHarness::new();
    let account = h.new_account_at(AccountAddress::from_hex_literal("0xf00d").unwrap());

    // Initial code
    let source = r#"
        module 0xf00d::M {
            #[resource_group(scope = address)]
            struct ResourceGroup { }

            struct NotResourceGroup { }
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
            #[resource_group(scope = global)]
            struct ResourceGroup { }

            struct NotResourceGroup { }
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
            #[resource_group(scope = module_)]
            struct ResourceGroup { }

            struct NotResourceGroup { }
        }
        "#;
    let mut builder = PackageBuilder::new("Package");
    builder.add_source("m.move", source);
    let path = builder.write_to_temp().unwrap();
    let result = h.publish_package(&account, path.path());
    assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED);

    // Incompatible removal of ResourceGroupContainer
    let source = r#"
        module 0xf00d::M {
            struct ResourceGroup { }

            struct NotResourceGroup { }
        }
        "#;
    let mut builder = PackageBuilder::new("Package");
    builder.add_source("m.move", source);
    let path = builder.write_to_temp().unwrap();
    let result = h.publish_package(&account, path.path());
    assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED);

    // Incompatible promotion of ResourceGroup
    let source = r#"
        module 0xf00d::M {
            #[resource_group(scope = global)]
            struct ResourceGroup { }

            #[resource_group(scope = address)]
            struct NotResourceGroup { }
        }
        "#;
    let mut builder = PackageBuilder::new("Package");
    builder.add_source("m.move", source);
    let path = builder.write_to_temp().unwrap();
    let result = h.publish_package(&account, path.path());
    assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED);
}

#[test]
fn verify_unsafe_resource_group_upgrades() {
    let mut h = MoveHarness::new_with_features(vec![], vec![FeatureFlag::SAFER_RESOURCE_GROUPS]);
    let account = h.new_account_at(AccountAddress::from_hex_literal("0xf00d").unwrap());

    // Initial code
    let source = r#"
        module 0xf00d::M {
            #[resource_group(scope = address)]
            struct ResourceGroup { }

            struct NotResourceGroup { }
        }
        "#;
    let mut builder = PackageBuilder::new("Package");
    builder.add_source("m.move", source);
    let path = builder.write_to_temp().unwrap();
    let result = h.publish_package(&account, path.path());
    assert_success!(result);

    // Incompatible promotion of ResourceGroup
    let source = r#"
        module 0xf00d::M {
            #[resource_group(scope = address)]
            struct ResourceGroup { }

            #[resource_group(scope = address)]
            struct NotResourceGroup { }
        }
        "#;
    let mut builder = PackageBuilder::new("Package");
    builder.add_source("m.move", source);
    let path = builder.write_to_temp().unwrap();
    let result = h.publish_package(&account, path.path());
    assert_success!(result);
}
