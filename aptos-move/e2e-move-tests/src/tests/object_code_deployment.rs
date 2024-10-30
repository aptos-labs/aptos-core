// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_abort, assert_success, assert_vm_status, tests::common, MoveHarness};
use aptos_framework::{
    natives::{
        code::{PackageRegistry, UpgradePolicy},
        object_code_deployment::ManagingRefs,
    },
    BuildOptions,
};
use aptos_language_e2e_tests::account::Account;
use aptos_types::{
    account_address::AccountAddress,
    object_address::create_object_code_deployment_address,
    on_chain_config::FeatureFlag,
    transaction::{ExecutionStatus, TransactionStatus},
};
use move_core_types::{parser::parse_struct_tag, vm_status::StatusCode};
use rstest::rstest;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

// TODO[Orderless]: Once Aaron updates `create_object_code_deployment_address` function to not take sequence number as input,
// revisit these tests and adapt them to orderless accounts

/// This tests the `object_code_deployment.move` module under the `aptos-framework` package.
/// The feature `OBJECT_CODE_DEPLOYMENT` is on by default for tests.

/// Mimics `object::test::State`
#[derive(Serialize, Deserialize)]
struct State {
    value: u64,
}

struct TestContext {
    harness: MoveHarness,
    account: Account,
    object_address: AccountAddress,
}

enum ObjectCodeAction {
    Deploy,
    Upgrade,
    Freeze,
}

impl TestContext {
    fn new(enabled: Option<Vec<FeatureFlag>>, disabled: Option<Vec<FeatureFlag>>) -> Self {
        let mut harness = if enabled.is_some() || disabled.is_some() {
            MoveHarness::new_with_features(
                enabled.unwrap_or_default(),
                disabled.unwrap_or_default(),
            )
        } else {
            MoveHarness::new()
        };

        let account =
            harness.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap(), Some(0));
        let sequence_number = harness.sequence_number_opt(account.address()).unwrap();
        let object_address =
            create_object_code_deployment_address(*account.address(), sequence_number + 1);
        TestContext {
            harness,
            account,
            object_address,
        }
    }

    fn execute_object_code_action(
        &mut self,
        account: &Account,
        path: &str,
        action: ObjectCodeAction,
    ) -> TransactionStatus {
        // Replace the module's address with the object address, this is needed to prevent module address mismatch errors.
        let mut options = BuildOptions::default();
        options
            .named_addresses
            .insert(MODULE_ADDRESS_NAME.to_string(), self.object_address);
        self.execute_object_code_action_with_options(account, path, action, options)
    }

    fn execute_object_code_action_with_options(
        &mut self,
        account: &Account,
        path: &str,
        action: ObjectCodeAction,
        options: BuildOptions,
    ) -> TransactionStatus {
        match action {
            ObjectCodeAction::Deploy => self.harness.object_code_deployment_package(
                account,
                &common::test_dir_path(path),
                options,
            ),
            ObjectCodeAction::Upgrade => self.harness.object_code_upgrade_package(
                account,
                &common::test_dir_path(path),
                options,
                self.object_address,
            ),
            ObjectCodeAction::Freeze => self
                .harness
                .object_code_freeze_code_object(account, self.object_address),
        }
    }

    fn assert_feature_flag_error(&self, status: TransactionStatus, err: &str) {
        if let TransactionStatus::Keep(ExecutionStatus::MoveAbort { info, .. }) = status {
            if let Some(abort_info) = info {
                assert_eq!(abort_info.reason_name, err);
            } else {
                panic!("Expected AbortInfo, but got None");
            }
        } else {
            panic!(
                "Expected TransactionStatus::Keep with ExecutionStatus::MoveAbort, but got {:?}",
                status
            );
        }
    }

    fn read_resource<T>(&self, address: &AccountAddress, struct_tag: &str) -> Option<T>
    where
        T: DeserializeOwned,
    {
        self.harness
            .read_resource::<T>(address, parse_struct_tag(struct_tag).unwrap())
    }
}

const MODULE_ADDRESS_NAME: &str = "object";
const MUT_DEPS_MODULE_ADDRESS_NAME: &str = "object_mutable_deps";
const IMMUT_DEPS_MODULE_ADDRESS_NAME: &str = "object_immutable_deps";
const PACKAGE_REGISTRY_ACCESS_PATH: &str = "0x1::code::PackageRegistry";
const EOBJECT_CODE_DEPLOYMENT_NOT_SUPPORTED: &str = "EOBJECT_CODE_DEPLOYMENT_NOT_SUPPORTED";
const ENOT_CODE_OBJECT_OWNER: &str = "ENOT_CODE_OBJECT_OWNER";
const ENOT_PACKAGE_OWNER: &str = "ENOT_PACKAGE_OWNER";
const EDEP_WEAKER_POLICY: &str = "EDEP_WEAKER_POLICY";

/// Tests the `publish` object code deployment function with feature flags enabled/disabled.
/// Deployment should only happen when feature is enabled.
#[rstest(enabled, disabled,
    case(vec![], vec![FeatureFlag::OBJECT_CODE_DEPLOYMENT]),
    case(vec![FeatureFlag::OBJECT_CODE_DEPLOYMENT], vec![]),
)]
fn object_code_deployment_publish_package(enabled: Vec<FeatureFlag>, disabled: Vec<FeatureFlag>) {
    let mut context = TestContext::new(Some(enabled.clone()), Some(disabled));
    let acc = context.account.clone();

    let status = context.execute_object_code_action(
        &acc,
        "object_code_deployment.data/pack_initial",
        ObjectCodeAction::Deploy,
    );

    if enabled.contains(&FeatureFlag::OBJECT_CODE_DEPLOYMENT) {
        assert_success!(status);

        let registry = context
            .read_resource::<PackageRegistry>(&context.object_address, PACKAGE_REGISTRY_ACCESS_PATH)
            .unwrap();
        assert_eq!(registry.packages.len(), 1);
        assert_eq!(registry.packages[0].name, "test_package");
        assert_eq!(registry.packages[0].modules.len(), 1);
        assert_eq!(registry.packages[0].modules[0].name, "test");

        let code_object: ManagingRefs = context
            .harness
            .read_resource_from_resource_group(
                &context.object_address,
                parse_struct_tag("0x1::object::ObjectGroup").unwrap(),
                parse_struct_tag("0x1::object_code_deployment::ManagingRefs").unwrap(),
            )
            .unwrap();
        // Verify the object created owns the `ManagingRefs`
        assert_eq!(code_object, ManagingRefs::new(context.object_address));

        let module_address = context.object_address.to_string();
        assert_success!(context.harness.run_entry_function(
            &context.account,
            str::parse(&format!("{}::test::hello", module_address)).unwrap(),
            vec![],
            vec![bcs::to_bytes::<u64>(&42).unwrap()]
        ));

        let state = context
            .read_resource::<State>(
                context.account.address(),
                &format!("{}::test::State", module_address),
            )
            .unwrap();
        assert_eq!(state.value, 42);
    } else {
        context.assert_feature_flag_error(status, EOBJECT_CODE_DEPLOYMENT_NOT_SUPPORTED);
    }
}

/// Tests the `upgrade` object code deployment function after `publish`ing a package prior calling.
#[test]
fn object_code_deployment_upgrade_success_compat() {
    let mut context = TestContext::new(None, None);
    let acc = context.account.clone();

    // Install the initial version with compat requirements
    assert_success!(context.execute_object_code_action(
        &acc,
        "object_code_deployment.data/pack_initial",
        ObjectCodeAction::Deploy,
    ));

    // We should be able to upgrade it with the compatible version
    assert_success!(context.execute_object_code_action(
        &acc,
        "object_code_deployment.data/pack_upgrade_compat",
        ObjectCodeAction::Upgrade,
    ));

    let module_address = context.object_address.to_string();
    // Call the new function added to the module
    assert_success!(context.harness.run_entry_function(
        &acc,
        str::parse(&format!("{}::test::hello2", module_address)).unwrap(),
        vec![],
        vec![bcs::to_bytes::<u64>(&42).unwrap()]
    ));
    let state = context
        .read_resource::<State>(acc.address(), &format!("{}::test::State", module_address))
        .unwrap();
    assert_eq!(state.value, 42);
}

#[test]
fn object_code_deployment_upgrade_fail_when_not_owner() {
    let mut context = TestContext::new(None, None);
    let acc = context.account.clone();

    // Install the initial version with compat requirements
    assert_success!(context.execute_object_code_action(
        &acc,
        "object_code_deployment.data/pack_initial",
        ObjectCodeAction::Deploy,
    ));

    // We should not be able to upgrade with a different account.
    let different_account = context
        .harness
        .new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap(), Some(0));
    let status = context.execute_object_code_action(
        &different_account,
        "object_code_deployment.data/pack_upgrade_compat",
        ObjectCodeAction::Upgrade,
    );
    context.assert_feature_flag_error(status, ENOT_CODE_OBJECT_OWNER);
}

#[test]
fn object_code_deployment_upgrade_fail_when_publisher_ref_does_not_exist() {
    let mut context = TestContext::new(None, None);
    let acc = context.account.clone();

    // We should not be able to `upgrade` as `ManagingRefs` does not exist.
    // `ManagingRefs` is only created when calling `publish` first, i.e. deploying a package.
    let status = context.execute_object_code_action(
        &acc,
        "object_code_deployment.data/pack_initial",
        ObjectCodeAction::Upgrade,
    );
    assert_abort!(status, _);
}

#[test]
fn object_code_deployment_upgrade_fail_compat() {
    let mut context = TestContext::new(None, None);
    let acc = context.account.clone();

    // Install the initial version with compat requirements
    assert_success!(context.execute_object_code_action(
        &acc,
        "object_code_deployment.data/pack_initial",
        ObjectCodeAction::Deploy,
    ));

    // We should not be able to upgrade it with the incompatible version
    let status = context.execute_object_code_action(
        &acc,
        "object_code_deployment.data/pack_upgrade_incompat",
        ObjectCodeAction::Upgrade,
    );
    assert_vm_status!(status, StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE);
}

#[test]
fn object_code_deployment_upgrade_fail_immutable() {
    let mut context = TestContext::new(None, None);
    let acc = context.account.clone();

    // Install the initial version with immutable requirements
    assert_success!(context.execute_object_code_action(
        &acc,
        "object_code_deployment.data/pack_initial_immutable",
        ObjectCodeAction::Deploy,
    ));

    // We should not be able to upgrade it with the incompatible version
    let status = context.execute_object_code_action(
        &acc,
        "object_code_deployment.data/pack_upgrade_compat",
        ObjectCodeAction::Upgrade,
    );
    assert_abort!(status, _);
}

#[test]
fn object_code_deployment_upgrade_fail_overlapping_module() {
    let mut context = TestContext::new(None, None);
    let acc = context.account.clone();

    // Install the initial version
    assert_success!(context.execute_object_code_action(
        &acc,
        "object_code_deployment.data/pack_initial",
        ObjectCodeAction::Deploy,
    ));

    // Install a different package with the same module.
    let status = context.execute_object_code_action(
        &acc,
        "object_code_deployment.data/pack_other_name",
        ObjectCodeAction::Upgrade,
    );
    assert_abort!(status, _);
}

/// Tests the `freeze_code_object` object code deployment function.
#[test]
fn object_code_deployment_freeze_code_object() {
    let mut context = TestContext::new(None, None);
    let acc = context.account.clone();

    // First deploy the package to an object.
    assert_success!(context.execute_object_code_action(
        &acc,
        "object_code_deployment.data/pack_initial",
        ObjectCodeAction::Deploy,
    ));

    // Mark packages immutable.
    assert_success!(context.execute_object_code_action(&acc, "", ObjectCodeAction::Freeze));

    let registry = context
        .read_resource::<PackageRegistry>(&context.object_address, PACKAGE_REGISTRY_ACCESS_PATH)
        .unwrap();
    // Verify packages are immutable.
    for package in &registry.packages {
        assert_eq!(package.upgrade_policy, UpgradePolicy::immutable());
    }
}

#[test]
fn freeze_code_object_fail_when_not_owner() {
    let mut context = TestContext::new(None, None);
    let acc = context.account.clone();

    assert_success!(context.execute_object_code_action(
        &acc,
        "object_code_deployment.data/pack_initial",
        ObjectCodeAction::Deploy,
    ));

    let different_account = context
        .harness
        .new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap(), Some(0));
    let status =
        context.execute_object_code_action(&different_account, "", ObjectCodeAction::Freeze);

    context.assert_feature_flag_error(status, ENOT_PACKAGE_OWNER);
}

#[test]
fn freeze_code_object_fail_when_having_mutable_dependency() {
    let mut context = TestContext::new(None, None);
    let acc = context.account.clone();

    assert_success!(context.execute_object_code_action(
        &acc,
        "object_code_deployment.data/pack_upgrade_compat",
        ObjectCodeAction::Deploy,
    ));
    let mut options = BuildOptions::default();
    options
        .named_addresses
        .insert(MODULE_ADDRESS_NAME.to_string(), context.object_address);
    let sequence_number = context.harness.sequence_number_opt(acc.address()).unwrap();
    context.object_address =
        create_object_code_deployment_address(*acc.address(), sequence_number + 1);
    options.named_addresses.insert(
        MUT_DEPS_MODULE_ADDRESS_NAME.to_string(),
        context.object_address,
    );
    assert_success!(context.execute_object_code_action_with_options(
        &acc,
        "object_code_deployment.data/pack_mutable_deps",
        ObjectCodeAction::Deploy,
        options
    ));

    // Freezing a package with upgradeable dependencies is not allowed.
    let status = context.execute_object_code_action(&acc, "", ObjectCodeAction::Freeze);
    context.assert_feature_flag_error(status, EDEP_WEAKER_POLICY);
}

#[test]
fn freeze_code_object_succeeds_when_all_dependencies_immutable() {
    let mut context = TestContext::new(None, None);
    let acc = context.account.clone();

    // Deploy immutable dependency package
    assert_success!(context.execute_object_code_action(
        &acc,
        "object_code_deployment.data/pack_initial_immutable",
        ObjectCodeAction::Deploy,
    ));
    let mut options = BuildOptions::default();
    options
        .named_addresses
        .insert(MODULE_ADDRESS_NAME.to_string(), context.object_address);
    let sequence_number = context.harness.sequence_number_opt(acc.address()).unwrap();
    context.object_address =
        create_object_code_deployment_address(*acc.address(), sequence_number + 1);
    options.named_addresses.insert(
        IMMUT_DEPS_MODULE_ADDRESS_NAME.to_string(),
        context.object_address,
    );

    // Deploy mutable package with immutable dependency
    assert_success!(context.execute_object_code_action_with_options(
        &acc,
        "object_code_deployment.data/pack_immutable_deps",
        ObjectCodeAction::Deploy,
        options
    ));

    // Attempt to freeze the initial package
    let status = context.execute_object_code_action(&acc, "", ObjectCodeAction::Freeze);
    assert_success!(status);
}

#[test]
fn freeze_code_object_fail_when_package_registry_does_not_exist() {
    let mut context = TestContext::new(None, None);
    let acc = context.account.clone();

    // We should not be able to `freeze_code_object` as `PackageRegistry` does not exist.
    // `PackageRegistry` is only created when calling `publish` first, i.e. deploying a package.
    let status = context.execute_object_code_action(&acc, "", ObjectCodeAction::Freeze);
    assert_abort!(status, _);
}
