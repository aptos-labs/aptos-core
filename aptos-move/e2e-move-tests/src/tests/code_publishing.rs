// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_abort, assert_success, assert_vm_status, tests::common, MoveHarness};
use aptos_types::account_address::AccountAddress;
use aptos_types::on_chain_config::FeatureFlag;
use framework::natives::code::{PackageRegistry, UpgradePolicy};
use move_deps::move_core_types::parser::parse_struct_tag;
use move_deps::move_core_types::vm_status::StatusCode;
use package_builder::PackageBuilder;
use rstest::rstest;
use serde::{Deserialize, Serialize};

// Note: this module uses parameterized tests via the
// [`rstest` crate](https://crates.io/crates/rstest)
// to test for multiple feature combinations.

/// Mimics `0xcafe::test::State`
#[derive(Serialize, Deserialize)]
struct State {
    value: u64,
}

/// Runs the basic publishing test for all legacy flag combinations. Otherwise we will only
/// run tests which are expected to make a difference for legacy flag combinations.
#[rstest]
#[case(vec![])]
#[case(vec![FeatureFlag::CODE_DEPENDENCY_CHECK])]
fn code_publishing_basic(#[case] features: Vec<FeatureFlag>) {
    let mut h = MoveHarness::new_with_features(features);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("code_publishing.data/pack_initial"),
    ));

    // Validate metadata as expected.
    let registry = h
        .read_resource::<PackageRegistry>(
            acc.address(),
            parse_struct_tag("0x1::code::PackageRegistry").unwrap(),
        )
        .unwrap();
    assert_eq!(registry.packages.len(), 1);
    assert_eq!(registry.packages[0].name, "test_package");
    assert_eq!(registry.packages[0].modules.len(), 1);
    assert_eq!(registry.packages[0].modules[0].name, "test");

    // Validate code loaded as expected.
    assert_success!(h.run_entry_function(
        &acc,
        str::parse("0xcafe::test::hello").unwrap(),
        vec![],
        vec![bcs::to_bytes::<u64>(&42).unwrap()]
    ));
    let state = h
        .read_resource::<State>(
            acc.address(),
            parse_struct_tag("0xcafe::test::State").unwrap(),
        )
        .unwrap();
    assert_eq!(state.value, 42)
}

// Ignored because we've disabled incompatible upgrade policy.
#[ignore]
#[rstest]
#[case(vec![FeatureFlag::CODE_DEPENDENCY_CHECK])]
fn code_publishing_upgrade_success_no_compat(#[case] features: Vec<FeatureFlag>) {
    let mut h = MoveHarness::new_with_features(features);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // Install the initial version with no compat requirements
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("code_publishing.data/pack_initial_arbitrary"),
    ));

    // We should be able to upgrade it with the incompatible version
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("code_publishing.data/pack_upgrade_incompat_arbitrary"),
    ));
}

#[rstest]
#[case(vec![FeatureFlag::CODE_DEPENDENCY_CHECK])]
fn code_publishing_upgrade_success_compat(#[case] features: Vec<FeatureFlag>) {
    let mut h = MoveHarness::new_with_features(features);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // Install the initial version with compat requirements
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("code_publishing.data/pack_initial"),
    ));

    // We should be able to upgrade it with the compatible version
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("code_publishing.data/pack_upgrade_compat"),
    ));
}

#[rstest]
#[case(vec![FeatureFlag::CODE_DEPENDENCY_CHECK])]
fn code_publishing_upgrade_fail_compat(#[case] features: Vec<FeatureFlag>) {
    let mut h = MoveHarness::new_with_features(features);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // Install the initial version with compat requirements
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("code_publishing.data/pack_initial"),
    ));

    // We should not be able to upgrade it with the incompatible version
    let status = h.publish_package(
        &acc,
        &common::test_dir_path("code_publishing.data/pack_upgrade_incompat"),
    );
    assert_vm_status!(status, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE)
}

#[rstest]
#[case(vec![FeatureFlag::CODE_DEPENDENCY_CHECK])]
fn code_publishing_upgrade_fail_immutable(#[case] features: Vec<FeatureFlag>) {
    let mut h = MoveHarness::new_with_features(features);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // Install the initial version with immutable requirements
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("code_publishing.data/pack_initial_immutable"),
    ));

    // We should not be able to upgrade it with the compatible version
    let status = h.publish_package(
        &acc,
        &common::test_dir_path("code_publishing.data/pack_upgrade_compat"),
    );
    assert_abort!(status, _);
}

#[rstest]
#[case(vec![FeatureFlag::CODE_DEPENDENCY_CHECK])]
fn code_publishing_upgrade_fail_overlapping_module(#[case] features: Vec<FeatureFlag>) {
    let mut h = MoveHarness::new_with_features(features);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // Install the initial version
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("code_publishing.data/pack_initial"),
    ));

    // Install a different package with the same module.
    let status = h.publish_package(
        &acc,
        &common::test_dir_path("code_publishing.data/pack_other_name"),
    );
    assert_abort!(status, _);
}

/// This test verifies that the cache incoherence bug on module upgrade is fixed. This bug
/// exposes itself by that after module upgrade the old version of the module stays
/// active until the MoveVM terminates. In order to workaround this until there is a better
/// fix, we flush the cache in `MoveVmExt::new_session`. One can verify the fix by commenting
/// the flush operation out, then this test fails.
///
/// TODO: for some reason this test did not capture a serious bug in `code::check_coexistence`.
#[test]
fn code_publishing_upgrade_loader_cache_consistency() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // Create a sequence of package upgrades
    let txns = vec![
        h.create_publish_package(
            &acc,
            &common::test_dir_path("code_publishing.data/pack_initial"),
            None,
            |_| {},
        ),
        // Compatible with above package
        h.create_publish_package(
            &acc,
            &common::test_dir_path("code_publishing.data/pack_upgrade_compat"),
            None,
            |_| {},
        ),
        // Not compatible with above package, but with first one.
        // Correct behavior: should create backward_incompatible error
        // Bug behavior: succeeds because is compared with the first module
        h.create_publish_package(
            &acc,
            &common::test_dir_path("code_publishing.data/pack_compat_first_not_second"),
            None,
            |_| {},
        ),
    ];
    let result = h.run_block(txns);
    assert_success!(result[0]);
    assert_success!(result[1]);
    assert_vm_status!(result[2], StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE)
}

#[rstest]
#[case(vec![FeatureFlag::CODE_DEPENDENCY_CHECK])]
fn code_publishing_framework_upgrade(#[case] features: Vec<FeatureFlag>) {
    let mut h = MoveHarness::new_with_features(features);
    let acc = h.aptos_framework_account();

    // We should be able to upgrade move-stdlib, as our local package has only
    // compatible changes. (We added a new function to string.move.)
    assert_success!(h.publish_package(
        &acc,
        &common::test_dir_path("code_publishing.data/pack_stdlib"),
    ));
}

#[rstest]
#[case(vec![FeatureFlag::CODE_DEPENDENCY_CHECK])]
fn code_publishing_framework_upgrade_fail(#[case] features: Vec<FeatureFlag>) {
    let mut h = MoveHarness::new_with_features(features);
    let acc = h.aptos_framework_account();

    // We should not be able to upgrade move-stdlib because we removed a function
    // from the string module.
    let result = h.publish_package(
        &acc,
        &common::test_dir_path("code_publishing.data/pack_stdlib_incompat"),
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE)
}

// Ignored because we've disabled incompatible upgrade policy.
#[ignore]
#[rstest]
#[case(vec![FeatureFlag::CODE_DEPENDENCY_CHECK])]
fn code_publishing_weak_dep_fail(#[case] features: Vec<FeatureFlag>) {
    let mut h = MoveHarness::new_with_features(features);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    let mut weak = PackageBuilder::new("WeakPackage").with_policy(UpgradePolicy::arbitrary());
    weak.add_source("weak", "module 0xcafe::weak { public fun f() {} }");

    let weak_dir = weak.write_to_temp().unwrap();
    assert_success!(h.publish_package(&acc, weak_dir.path()));

    let mut normal = PackageBuilder::new("Package").with_policy(UpgradePolicy::compat());
    normal.add_local_dep("WeakPackage", &weak_dir.path().to_string_lossy());
    normal.add_source(
        "normal",
        "module 0xcafe::normal { use 0xcafe::weak; public fun f() { weak::f() } }",
    );
    let normal_dir = normal.write_to_temp().unwrap();
    let status = h.publish_package(&acc, normal_dir.path());
    assert_abort!(status, 0x10006 /*invalid_arhument(EDEP_WEAKER_POLICY)*/);
}

// Ignored because we've disabled incompatible upgrade policy.
#[ignore]
#[rstest]
#[case(vec![FeatureFlag::CODE_DEPENDENCY_CHECK])]
fn code_publishing_arbitray_dep_different_address(#[case] features: Vec<FeatureFlag>) {
    let mut h = MoveHarness::new_with_features(features);
    let acc1 = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    let acc2 = h.new_account_at(AccountAddress::from_hex_literal("0xdeaf").unwrap());

    let mut pack1 = PackageBuilder::new("Package1").with_policy(UpgradePolicy::arbitrary());
    pack1.add_source("m", "module 0xcafe::m { public fun f() {} }");
    let pack1_dir = pack1.write_to_temp().unwrap();

    let mut pack2 = PackageBuilder::new("Package2").with_policy(UpgradePolicy::arbitrary());
    pack2.add_local_dep("Package1", &pack1_dir.path().to_string_lossy());
    pack2.add_source(
        "m",
        "module 0xdeaf::m { use 0xcafe::m; public fun f() { m::f() } }",
    );
    let pack2_dir = pack2.write_to_temp().unwrap();

    assert_success!(h.publish_package(&acc1, pack1_dir.path()));
    assert_abort!(
        h.publish_package(&acc2, pack2_dir.path()),
        0x10007 /*EDEP_ARBITRARY_NOT_SAME_ADDRESS*/
    );
}

#[rstest]
#[case(vec![FeatureFlag::CODE_DEPENDENCY_CHECK])]
fn code_publishing_using_resource_account(#[case] features: Vec<FeatureFlag>) {
    let mut h = MoveHarness::new_with_features(features);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    let mut pack = PackageBuilder::new("Package1").with_policy(UpgradePolicy::compat());
    pack.add_source("m", "module 0x0b6beee9bc1ad3177403a04efeefb1901c12b7b575ac5124c0205efc0dd2e32a::m { public fun f() {} }");
    let pack_dir = pack.write_to_temp().unwrap();
    let package = framework::BuiltPackage::build(
        pack_dir.path().to_owned(),
        framework::BuildOptions::default(),
    )
    .expect("building package must succeed");

    let code = package.extract_code();
    let metadata = package
        .extract_metadata()
        .expect("extracting package metadata must succeed");
    let bcs_metadata = bcs::to_bytes(&metadata).expect("PackageMetadata has BCS");

    let result = h.run_transaction_payload(
        &acc,
        cached_packages::aptos_stdlib::resource_account_create_resource_account_and_publish_package(
            vec![],
            bcs_metadata,
            code,
        ),
    );
    assert_success!(result);
}

#[rstest]
#[case(vec![])]
#[case(vec![FeatureFlag::CODE_DEPENDENCY_CHECK])]
fn code_publishing_faked_dependency(#[case] features: Vec<FeatureFlag>) {
    let mut h = MoveHarness::new_with_features(features.clone());
    let acc1 = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    let acc2 = h.new_account_at(AccountAddress::from_hex_literal("0xdeaf").unwrap());

    let mut pack1 = PackageBuilder::new("Package1").with_policy(UpgradePolicy::compat());
    pack1.add_source("m", "module 0xcafe::m { public fun f() {} }");
    let pack1_dir = pack1.write_to_temp().unwrap();
    assert_success!(h.publish_package(&acc1, pack1_dir.path()));

    // pack2 has a higher policy and should not be able to depend on pack1
    let mut pack2 = PackageBuilder::new("Package2").with_policy(UpgradePolicy::immutable());
    pack2.add_local_dep("Package1", &pack1_dir.path().to_string_lossy());
    pack2.add_source(
        "m",
        "module 0xdeaf::m { use 0xcafe::m; public fun f() { m::f() } }",
    );
    let pack2_dir = pack2.write_to_temp().unwrap();
    let result = h.publish_package_with_patcher(&acc2, pack2_dir.path(), |metadata| {
        // Hide the dependency from the lower policy package from the metadata. We detect this
        // this via checking the actual bytecode module dependencies.
        metadata.deps.clear()
    });
    if !features.contains(&FeatureFlag::CODE_DEPENDENCY_CHECK) {
        // In the previous version we were not able to detect this problem
        assert_success!(result)
    } else {
        assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED)
    }
}

#[rstest]
#[case(vec![])]
#[case(vec![FeatureFlag::TREAT_FRIEND_AS_PRIVATE])]
fn code_publishing_friend_as_private(#[case] features: Vec<FeatureFlag>) {
    let mut h = MoveHarness::new_with_features(features.clone());
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    let mut pack1 = PackageBuilder::new("Package").with_policy(UpgradePolicy::compat());
    pack1.add_source(
        "m",
        "module 0xcafe::m { public fun f() {}  public(friend) fun g() {} }",
    );
    let pack1_dir = pack1.write_to_temp().unwrap();
    assert_success!(h.publish_package(&acc, pack1_dir.path()));

    let mut pack2 = PackageBuilder::new("Package").with_policy(UpgradePolicy::compat());
    // Removes friend
    pack2.add_source("m", "module 0xcafe::m { public fun f() {} }");
    let pack2_dir = pack2.write_to_temp().unwrap();

    let result = h.publish_package(&acc, pack2_dir.path());
    if features.contains(&FeatureFlag::TREAT_FRIEND_AS_PRIVATE) {
        // With this feature we can remove friends
        assert_success!(result)
    } else {
        assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE)
    }
}
