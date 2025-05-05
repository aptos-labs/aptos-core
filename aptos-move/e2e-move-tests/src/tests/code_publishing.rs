// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    assert_abort, assert_success, assert_vm_status, build_package, tests::common, MoveHarness,
};
use aptos_framework::natives::code::{PackageRegistry, UpgradePolicy};
use aptos_language_e2e_tests::executor::FakeExecutor;
use aptos_package_builder::PackageBuilder;
use aptos_types::{
    account_address::{create_resource_address, AccountAddress},
    move_utils::MemberId,
    on_chain_config::FeatureFlag,
    transaction::{ExecutionStatus, TransactionPayload, TransactionStatus},
};
use claims::assert_ok;
use move_core_types::{
    identifier::Identifier,
    language_storage::ModuleId,
    parser::parse_struct_tag,
    vm_status::{AbortLocation, StatusCode},
};
use rstest::rstest;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

// Note: this module uses parameterized tests via the
// [`rstest` crate](https://crates.io/crates/rstest)
// to test for multiple feature combinations.

/// Mimics `0xcafe::test::State`
#[derive(Serialize, Deserialize)]
struct State {
    value: u64,
}

/// Mimics `0xcafe::test::State`
#[derive(Serialize, Deserialize)]
struct StateWithCoins {
    important_value: u64,
    value: u64,
}

/// Runs the basic publishing test for all legacy flag combinations. Otherwise we will only
/// run tests which are expected to make a difference for legacy flag combinations.
#[rstest(enabled, disabled,
    case(vec![], vec![FeatureFlag::CODE_DEPENDENCY_CHECK]),
    case(vec![FeatureFlag::CODE_DEPENDENCY_CHECK], vec![]),
)]
fn code_publishing_basic(enabled: Vec<FeatureFlag>, disabled: Vec<FeatureFlag>) {
    let mut h = MoveHarness::new_with_features(enabled, disabled);
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
    assert_success!(h.publish_package_cache_building(
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

#[test]
fn code_publishing_upgrade_success_compat() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // Install the initial version with compat requirements
    assert_success!(h.publish_package_cache_building(
        &acc,
        &common::test_dir_path("code_publishing.data/pack_initial"),
    ));

    // We should be able to upgrade it with the compatible version
    assert_success!(h.publish_package_cache_building(
        &acc,
        &common::test_dir_path("code_publishing.data/pack_upgrade_compat"),
    ));
}

#[test]
fn code_publishing_disallow_user_native() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    assert_vm_status!(
        h.publish_package_cache_building(
            &acc,
            &common::test_dir_path("code_publishing.data/pack_native"),
        ),
        StatusCode::USER_DEFINED_NATIVE_NOT_ALLOWED
    );
}

#[test]
fn code_publishing_disallow_user_native_entry() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    assert_vm_status!(
        h.publish_package_cache_building(
            &acc,
            &common::test_dir_path("code_publishing.data/pack_native_entry"),
        ),
        StatusCode::USER_DEFINED_NATIVE_NOT_ALLOWED
    );
}

#[test]
fn code_publishing_allow_system_native() {
    let mut h = MoveHarness::new();
    let acc = h.aptos_framework_account();

    assert_success!(h.publish_package_cache_building(
        &acc,
        &common::test_dir_path("code_publishing.data/pack_native_system"),
    ));
}

#[test]
fn code_publishing_disallow_system_native_entry() {
    let mut h = MoveHarness::new();
    let acc = h.aptos_framework_account();

    assert_vm_status!(
        h.publish_package_cache_building(
            &acc,
            &common::test_dir_path("code_publishing.data/pack_native_system_entry"),
        ),
        StatusCode::USER_DEFINED_NATIVE_NOT_ALLOWED
    );
}

#[test]
fn code_publishing_upgrade_fail_compat() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // Install the initial version with compat requirements
    assert_success!(h.publish_package_cache_building(
        &acc,
        &common::test_dir_path("code_publishing.data/pack_initial"),
    ));

    // We should not be able to upgrade it with the incompatible version
    let status = h.publish_package_cache_building(
        &acc,
        &common::test_dir_path("code_publishing.data/pack_upgrade_incompat"),
    );
    assert_vm_status!(status, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE)
}

#[test]
fn code_publishing_upgrade_fail_immutable() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // Install the initial version with immutable requirements
    assert_success!(h.publish_package_cache_building(
        &acc,
        &common::test_dir_path("code_publishing.data/pack_initial_immutable"),
    ));

    // We should not be able to upgrade it with the compatible version
    let status = h.publish_package_cache_building(
        &acc,
        &common::test_dir_path("code_publishing.data/pack_upgrade_compat"),
    );
    assert_abort!(status, _);
}

#[test]
fn code_publishing_upgrade_fail_overlapping_module() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // Install the initial version
    assert_success!(h.publish_package_cache_building(
        &acc,
        &common::test_dir_path("code_publishing.data/pack_initial"),
    ));

    // Install a different package with the same module.
    let status = h.publish_package_cache_building(
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
        h.create_publish_package_cache_building(
            &acc,
            &common::test_dir_path("code_publishing.data/pack_initial"),
            |_| {},
        ),
        // Compatible with above package
        h.create_publish_package_cache_building(
            &acc,
            &common::test_dir_path("code_publishing.data/pack_upgrade_compat"),
            |_| {},
        ),
        // Not compatible with above package, but with first one.
        // Correct behavior: should create backward_incompatible error
        // Bug behavior: succeeds because is compared with the first module
        h.create_publish_package_cache_building(
            &acc,
            &common::test_dir_path("code_publishing.data/pack_compat_first_not_second"),
            |_| {},
        ),
    ];
    let result = h.run_block(txns);
    assert_success!(result[0]);
    assert_success!(result[1]);
    assert_vm_status!(result[2], StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE)
}

#[test]
fn code_publishing_framework_upgrade() {
    let mut h = MoveHarness::new();
    let acc = h.aptos_framework_account();

    // We should be able to upgrade move-stdlib, as our local package has only
    // compatible changes. (We added a new function to string.move.)
    assert_success!(h.publish_package_cache_building(
        &acc,
        &common::test_dir_path("code_publishing.data/pack_stdlib"),
    ));
}

#[test]
fn code_publishing_framework_upgrade_fail() {
    let mut h = MoveHarness::new();
    let acc = h.aptos_framework_account();

    // We should not be able to upgrade move-stdlib because we removed a function
    // from the string module.
    let result = h.publish_package_cache_building(
        &acc,
        &common::test_dir_path("code_publishing.data/pack_stdlib_incompat"),
    );
    assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE)
}

#[test]
fn code_publishing_using_resource_account() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    let mut pack = PackageBuilder::new("Package1").with_policy(UpgradePolicy::compat());
    let module_address = create_resource_address(*acc.address(), &[]);
    pack.add_source(
        "m",
        &format!(
            "module 0x{}::m {{ public fun f() {{}} }}",
            module_address.to_hex()
        ),
    );
    let pack_dir = pack.write_to_temp().unwrap();
    let package = build_package(
        pack_dir.path().to_owned(),
        aptos_framework::BuildOptions::default(),
    )
    .expect("building package must succeed");

    let code = package.extract_code();
    let metadata = package
        .extract_metadata()
        .expect("extracting package metadata must succeed");
    let bcs_metadata = bcs::to_bytes(&metadata).expect("PackageMetadata has BCS");

    let result = h.run_transaction_payload(
        &acc,
        aptos_cached_packages::aptos_stdlib::resource_account_create_resource_account_and_publish_package(
            vec![],
            bcs_metadata,
            code,
        ),
    );
    assert_success!(result);
}

#[test]
fn code_publishing_with_two_attempts_and_verify_loader_is_invalidated() {
    let mut h = MoveHarness::new();
    let acc = h.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

    // First module publish attempt failed when executing the init_module.
    // Second attempt should pass.
    // We expect the correct logic in init_module to be executed from the second attempt so the
    // value stored is from the second code, and not the first (which would be the case if the
    // VM's loader cache is not properly cleared after the first attempt).
    //
    // Depending on how the loader cache is flushed, the second attempt might even fail if the
    // entire init_module from the first attempt still lingers around and will fail if invoked.
    let failed_module_publish = h.create_publish_package_cache_building(
        &acc,
        &common::test_dir_path("code_publishing.data/pack_init_module_failed"),
        |_| {},
    );
    let module_publish_second_attempt = h.create_publish_package_cache_building(
        &acc,
        &common::test_dir_path("code_publishing.data/pack_init_module_second_attempt"),
        |_| {},
    );
    let results = h.run_block(vec![failed_module_publish, module_publish_second_attempt]);
    assert_abort!(results[0], _);
    assert_success!(results[1]);

    let value_resource = h
        .read_resource::<StateWithCoins>(
            acc.address(),
            parse_struct_tag("0xcafe::test::State").unwrap(),
        )
        .unwrap();
    assert_eq!(2, value_resource.important_value);
}

#[rstest(enabled, disabled,
         case(vec![], vec![FeatureFlag::CODE_DEPENDENCY_CHECK]),
         case(vec![FeatureFlag::CODE_DEPENDENCY_CHECK], vec![]),
)]
fn code_publishing_faked_dependency(enabled: Vec<FeatureFlag>, disabled: Vec<FeatureFlag>) {
    let mut h = MoveHarness::new_with_features(enabled.clone(), disabled);
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
    if !enabled.contains(&FeatureFlag::CODE_DEPENDENCY_CHECK) {
        // In the previous version we were not able to detect this problem
        assert_success!(result)
    } else {
        assert_vm_status!(result, StatusCode::CONSTRAINT_NOT_SATISFIED)
    }
}

#[rstest(enabled, disabled,
         case(vec![], vec![FeatureFlag::TREAT_FRIEND_AS_PRIVATE]),
         case(vec![FeatureFlag::TREAT_FRIEND_AS_PRIVATE], vec![]),
)]
fn code_publishing_friend_as_private(enabled: Vec<FeatureFlag>, disabled: Vec<FeatureFlag>) {
    let mut h = MoveHarness::new_with_features(enabled.clone(), disabled);
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
    if enabled.contains(&FeatureFlag::TREAT_FRIEND_AS_PRIVATE) {
        // With this feature we can remove friends
        assert_success!(result)
    } else {
        assert_vm_status!(result, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE)
    }
}

#[test]
fn test_module_publishing_does_not_fallback() {
    let mut executor = FakeExecutor::from_head_genesis().set_parallel();
    executor.disable_block_executor_fallback();

    let mut h = MoveHarness::new_with_executor(executor);
    let addr = AccountAddress::from_hex_literal("0x123").unwrap();
    let account = h.new_account_at(addr);

    let module_name = "foo";
    let function_name = "bar";
    let member_id =
        MemberId::from_str(&format!("{}::{}::{}", addr, module_name, function_name)).unwrap();

    let mut txns = vec![];
    let mut expected_abort_codes: Vec<Option<u64>> = vec![];

    // Generate a simple test workload.
    for abort_code in [1, 2, 3, 4, 5, 1] {
        // Transaction that publishes code, must succeed.
        let source = format!(
            "module {}::{} {{ public entry fun {}() {{ abort {} }} }}",
            addr, module_name, function_name, abort_code
        );
        let txn = h.create_transaction_payload(&account, publish_module_txn(source, module_name));
        txns.push(txn);
        expected_abort_codes.push(None);

        let mut i = 0;
        while i < abort_code {
            // Transaction that calls an entry that aborts.
            let caller = h.new_account_at(AccountAddress::random());
            let txn = h.create_entry_function(&caller, member_id.clone(), vec![], vec![]);
            txns.push(txn);
            expected_abort_codes.push(Some(abort_code));

            i += 1;
        }
    }

    for (output, maybe_abort_code) in h
        .run_block_get_output(txns)
        .into_iter()
        .zip(expected_abort_codes.into_iter())
    {
        let status = output.status().clone();
        match maybe_abort_code {
            Some(abort_code) => {
                // Transaction aborts with correct code set by the previous module publish.
                let status = assert_ok!(status.as_kept_status());
                if let ExecutionStatus::MoveAbort {
                    location,
                    code,
                    info: _,
                } = status
                {
                    assert_eq!(code, abort_code);
                    assert_eq!(
                        location,
                        AbortLocation::Module(ModuleId::new(
                            addr,
                            Identifier::new(module_name).unwrap()
                        ))
                    );
                } else {
                    panic!(
                        "Transaction is expected to fail with Move abort and code {}",
                        abort_code
                    )
                }
            },
            None => {
                // Module publishing succeeds.
                assert_success!(status);
            },
        }
    }
}

fn publish_module_txn(source: String, module_name: &str) -> TransactionPayload {
    let mut builder = PackageBuilder::new(module_name).with_policy(UpgradePolicy::compat());
    builder.add_source(module_name, &source);

    let pack_dir = assert_ok!(builder.write_to_temp());
    let package = assert_ok!(build_package(
        pack_dir.path().to_owned(),
        aptos_framework::BuildOptions::default(),
    ));

    let code = package.extract_code();
    let metadata = package.extract_metadata().unwrap();
    aptos_cached_packages::aptos_stdlib::code_publish_package_txn(
        bcs::to_bytes(&metadata).unwrap(),
        code,
    )
}

#[derive(Serialize, Deserialize)]
struct Foo {
    data: u64,
}

#[test]
fn test_module_publishing_does_not_leak_speculative_information() {
    let mut executor = FakeExecutor::from_head_genesis().set_parallel();
    executor.disable_block_executor_fallback();

    let mut h = MoveHarness::new_with_executor(executor);
    let addr = AccountAddress::random();
    let account = h.new_account_at(addr);

    let tys_with_abort_codes = [
        ("u8", Some(1)),
        ("u16", Some(2)),
        ("u32", Some(3)),
        ("u128", Some(4)),
        ("u256", Some(5)),
        ("u64", None),
    ];

    let mut txns = vec![];
    let mut expected_abort_codes: Vec<Option<u64>> = vec![];

    for (ty, maybe_abort_code) in tys_with_abort_codes {
        expected_abort_codes.push(maybe_abort_code);

        // Create module to be published that uses the same type name but different layout. For
        // the first few modules, run 'init_module' to ensure the type is used and then abort the
        // transaction. This ensures that the struct name re-indexing cache and publishing works as
        // expected - not caching type layouts for the struct until it is actually committed.
        let struct_def = format!("struct Foo has key, store {{ data: {}, }}", ty);
        let init_module = if let Some(abort_code) = maybe_abort_code {
            format!("fun init_module(sender: &signer) {{ move_to(sender, Foo {{ data: 0 }}); abort {} }}", abort_code)
        } else {
            "fun init_module(sender: &signer) { move_to(sender, Foo { data: 77 }) }".to_string()
        };
        let source = format!("module {}::foo {{ {} {} }}", addr, struct_def, init_module);

        let txn = h.create_transaction_payload(&account, publish_module_txn(source, "foo"));
        txns.push(txn);
    }

    for (output, maybe_abort_code) in h
        .run_block_get_output(txns)
        .into_iter()
        .zip(expected_abort_codes.into_iter())
    {
        let status = output.status().clone();
        match maybe_abort_code {
            Some(abort_code) => {
                assert_move_abort(status, abort_code);
            },
            None => {
                assert_success!(status);
                let struct_tag = parse_struct_tag(&format!("{}::foo::Foo", addr)).unwrap();
                let data = h.read_resource::<Foo>(&addr, struct_tag).unwrap().data;
                assert_eq!(data, 77);
            },
        }
    }
}

fn assert_move_abort(status: TransactionStatus, expected_abort_code: u64) {
    let status = assert_ok!(status.as_kept_status());
    if let ExecutionStatus::MoveAbort {
        location: _,
        code,
        info: _,
    } = status
    {
        assert_eq!(code, expected_abort_code);
    } else {
        panic!(
            "Transaction should succeed with abort code {}, got {:?}",
            expected_abort_code, status
        );
    }
}

#[test]
fn test_init_module_should_not_publish_modules() {
    let mut h = MoveHarness::new();
    let addr = AccountAddress::from_hex_literal("0xcafe").unwrap();
    let account = h.new_account_at(addr);

    let txn = h.create_publish_package_cache_building(
        &account,
        &common::test_dir_path("code_publishing.data/pack_init_module_code_publish"),
        |_| {},
    );
    let output = h.run_block_get_output(vec![txn]).pop().unwrap();
    // The abort code corresponds to EALREADY_REQUESTED.
    assert_move_abort(output.status().clone(), 0x03_0000);
}
