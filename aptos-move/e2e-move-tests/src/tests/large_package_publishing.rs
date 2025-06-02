// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_move_abort, assert_success, assert_vm_status, tests::common, MoveHarness};
use aptos_framework::{
    chunked_publish::{
        chunk_package_and_create_payloads, PublishType, CHUNK_SIZE_IN_BYTES,
        LARGE_PACKAGES_DEV_MODULE_ADDRESS,
    },
    natives::{
        code::{PackageMetadata, PackageRegistry, UpgradePolicy},
        object_code_deployment::ManagingRefs,
    },
    BuildOptions, BuiltPackage,
};
use aptos_language_e2e_tests::account::Account;
use aptos_types::{
    object_address::create_object_code_deployment_address,
    transaction::{AbortInfo, TransactionPayload, TransactionStatus},
};
use move_core_types::{
    account_address::AccountAddress, parser::parse_struct_tag, vm_status::StatusCode,
};
use move_package::source_package::std_lib::StdVersion;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    option::Option,
    path::{Path, PathBuf},
    str::FromStr,
};

/// Number of transactions needed for staging code chunks before publishing to accounts or objects
/// This is used to derive object address for testing object code deployment feature
const NUMBER_OF_TRANSACTIONS_FOR_STAGING: u64 = 2;

/// Mimics `0xcafe::eight::State`
#[derive(Serialize, Deserialize)]
struct State {
    value: u64,
}

struct LargePackageTestContext {
    harness: MoveHarness,
    account: Account, // used for testing account code deployment for large packages
    object_address: AccountAddress, // used for testing object code deployment for large packages
}

impl LargePackageTestContext {
    /// Create a new test context with initialized accounts and published `large_packages.move` module.
    fn new() -> Self {
        let mut harness = MoveHarness::new();
        let account = harness.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
        let sequence_number = harness.sequence_number(account.address());
        let object_address = create_object_code_deployment_address(
            *account.address(),
            sequence_number + NUMBER_OF_TRANSACTIONS_FOR_STAGING + 1,
        );

        LargePackageTestContext {
            harness,
            account,
            object_address,
        }
    }

    /// Get the local framework path based on this source file's location.
    /// Note: If this source file is moved to a different location, this function
    /// may need to be updated.
    fn get_local_framework_path() -> String {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .map(|p| p.join("framework"))
            .expect("framework path")
            .to_string_lossy()
            .to_string()
    }

    fn get_named_addresses_build_options(
        named_addresses: Vec<(String, AccountAddress)>,
    ) -> BuildOptions {
        let mut build_options = BuildOptions::default();
        let mut map = BTreeMap::new();
        for (k, v) in named_addresses {
            map.insert(k, v);
        }
        build_options.named_addresses = map;
        build_options.override_std = Some(StdVersion::Local(Self::get_local_framework_path()));

        build_options
    }

    /// Publish a large package by creating and running the necessary transactions.
    fn publish_large_package(
        &mut self,
        account: &Account,
        path: &Path,
        patch_metadata: impl FnMut(&mut PackageMetadata),
        publish_type: PublishType,
    ) -> Vec<TransactionStatus> {
        let deploy_address = match publish_type {
            PublishType::AccountDeploy => AccountAddress::from_hex_literal("0xcafe").unwrap(),
            PublishType::ObjectDeploy | PublishType::ObjectUpgrade => self.object_address,
        };

        let build_options = Self::get_named_addresses_build_options(vec![(
            String::from("large_package_example"),
            deploy_address,
        )]);
        let payloads = self.create_publish_large_package_from_path(
            path,
            Some(build_options),
            patch_metadata,
            publish_type,
        );
        payloads
            .into_iter()
            .map(|payload| {
                let signed_tx = self
                    .harness
                    .create_transaction_without_sign(account, payload)
                    .sign();
                self.harness.run(signed_tx)
            })
            .collect()
    }

    /// Create transactions for publishing a large package.
    fn create_publish_large_package_from_path(
        &mut self,
        path: &Path,
        options: Option<BuildOptions>,
        mut patch_metadata: impl FnMut(&mut PackageMetadata),
        publish_type: PublishType,
    ) -> Vec<TransactionPayload> {
        let package = BuiltPackage::build(path.to_owned(), options.unwrap())
            .expect("package build must succeed");
        let package_code = package.extract_code();
        let mut metadata = package
            .extract_metadata()
            .expect("extracting package metadata must succeed");
        patch_metadata(&mut metadata);
        let metadata_serialized = bcs::to_bytes(&metadata).expect("Failed deserializing metadata");
        chunk_package_and_create_payloads(
            metadata_serialized,
            package_code,
            publish_type,
            Some(self.object_address),
            AccountAddress::from_str(LARGE_PACKAGES_DEV_MODULE_ADDRESS).unwrap(),
            CHUNK_SIZE_IN_BYTES,
        )
    }
}

#[test]
fn large_package_publishing_basic() {
    let mut context = LargePackageTestContext::new();
    let acc = context.account.clone();

    // Test transactions for publishing the large package are successful
    let tx_statuses = context.publish_large_package(
        &acc,
        &common::test_dir_path("../../../move-examples/large_packages/large_package_example"),
        |_| {},
        PublishType::AccountDeploy,
    );
    for tx_status in tx_statuses.into_iter() {
        assert_success!(tx_status);
    }

    // Validate metadata
    let registry = context
        .harness
        .read_resource::<PackageRegistry>(
            acc.address(),
            parse_struct_tag("0x1::code::PackageRegistry").unwrap(),
        )
        .unwrap();
    assert_eq!(registry.packages.len(), 1);
    assert_eq!(registry.packages[0].name, "LargePackageExample");
    assert_eq!(registry.packages[0].modules.len(), 9); // `LargePackageExample` package includes 9 modules

    // Validate code loaded as expected.
    assert_success!(context.harness.run_entry_function(
        &acc,
        str::parse("0xcafe::eight::hello").unwrap(),
        vec![],
        vec![bcs::to_bytes::<u64>(&42).unwrap()]
    ));
    let state = context
        .harness
        .read_resource::<State>(
            acc.address(),
            parse_struct_tag("0xcafe::eight::State").unwrap(),
        )
        .unwrap();
    assert_eq!(state.value, 42);
}

#[test]
fn large_package_upgrade_success_compat() {
    let mut context = LargePackageTestContext::new();
    let acc = context.account.clone();

    // Initial version
    let tx_statuses = context.publish_large_package(
        &acc,
        &common::test_dir_path("../../../move-examples/large_packages/large_package_example"),
        |_| {},
        PublishType::AccountDeploy,
    );
    for tx_status in tx_statuses.into_iter() {
        assert_success!(tx_status);
    }

    // Upgrade to compatible version
    let tx_statuses = context.publish_large_package(
        &acc,
        &common::test_dir_path("../../../move-examples/large_packages/large_package_example"), // upgrade with the same package
        |_| {},
        PublishType::AccountDeploy,
    );
    for tx_status in tx_statuses.into_iter() {
        assert_success!(tx_status);
    }
}

#[test]
fn large_package_upgrade_fail_compat() {
    let mut context = LargePackageTestContext::new();
    let acc = context.account.clone();

    // Initial version
    let tx_statuses = context.publish_large_package(
        &acc,
        &common::test_dir_path("../../../move-examples/large_packages/large_package_example"),
        |_| {},
        PublishType::AccountDeploy,
    );
    for tx_status in tx_statuses.into_iter() {
        assert_success!(tx_status);
    }

    // Upgrade to incompatible version should fail
    // Staging metadata and code should pass, and the final publishing transaction should fail
    let mut tx_statuses = context.publish_large_package(
        &acc,
        &common::test_dir_path("large_package_publishing.data/large_pack_upgrade_incompat"),
        |_| {},
        PublishType::AccountDeploy,
    );

    let last_tx_status = tx_statuses.pop().unwrap(); // transaction for publishing

    for tx_status in tx_statuses.into_iter() {
        assert_success!(tx_status);
    }
    assert_vm_status!(
        last_tx_status,
        StatusCode::BACKWARD_INCOMPATIBLE_MODULE_UPDATE
    );
}

#[test]
fn large_package_upgrade_fail_immutable() {
    let mut context = LargePackageTestContext::new();
    let acc = context.account.clone();

    // Initial version (immutable package)
    let tx_statuses = context.publish_large_package(
        &acc,
        &common::test_dir_path("../../../move-examples/large_packages/large_package_example"),
        |metadata| metadata.upgrade_policy = UpgradePolicy::immutable(),
        PublishType::AccountDeploy,
    );

    for tx_status in tx_statuses.into_iter() {
        assert_success!(tx_status);
    }

    // Upgrading immutable package should fail
    // Staging metadata and code should pass, and the final publishing transaction should fail
    let mut tx_statuses = context.publish_large_package(
        &acc,
        &common::test_dir_path("../../../move-examples/large_packages/large_package_example"),
        |_| {},
        PublishType::AccountDeploy,
    );
    let last_tx_status = tx_statuses.pop().unwrap(); // transaction for publishing
    for tx_status in tx_statuses.into_iter() {
        assert_success!(tx_status);
    }
    let abort_info = Some(AbortInfo {
        reason_name: "EUPGRADE_IMMUTABLE".to_string(),
        description: "Cannot upgrade an immutable package".to_string(),
    });
    assert_move_abort!(last_tx_status, abort_info);
}

#[test]
fn large_package_upgrade_fail_overlapping_module() {
    let mut context = LargePackageTestContext::new();
    let acc = context.account.clone();

    // Initial version
    let tx_statuses = context.publish_large_package(
        &acc,
        &common::test_dir_path("../../../move-examples/large_packages/large_package_example"),
        |_| {},
        PublishType::AccountDeploy,
    );
    for tx_status in tx_statuses.into_iter() {
        assert_success!(tx_status);
    }

    // Publishing the same package with different name should fail
    // Staging metadata and code should pass, and the final publishing transaction should fail
    let mut tx_statuses = context.publish_large_package(
        &acc,
        &common::test_dir_path("../../../move-examples/large_packages/large_package_example"),
        |metadata| metadata.name = "other_large_pack".to_string(),
        PublishType::AccountDeploy,
    );

    let last_tx_status = tx_statuses.pop().unwrap(); // transaction for publishing

    for tx_status in tx_statuses.into_iter() {
        assert_success!(tx_status);
    }
    let abort_info = Some(AbortInfo {
        reason_name: "EMODULE_NAME_CLASH".to_string(),
        description: "Package contains duplicate module names with existing modules publised in other packages on this address".to_string(),
    });
    assert_move_abort!(last_tx_status, abort_info);
}

#[test]
fn large_package_object_code_deployment_basic() {
    let mut context = LargePackageTestContext::new();
    let acc = context.account.clone();

    // Test transactions for publishing the large package are successful
    let tx_statuses = context.publish_large_package(
        &acc,
        &common::test_dir_path("../../../move-examples/large_packages/large_package_example"),
        |_| {},
        PublishType::ObjectDeploy,
    );
    for tx_status in tx_statuses.into_iter() {
        assert_success!(tx_status);
    }

    // Validate metadata
    let registry = context
        .harness
        .read_resource::<PackageRegistry>(
            &context.object_address,
            parse_struct_tag("0x1::code::PackageRegistry").unwrap(),
        )
        .unwrap();
    assert_eq!(registry.packages.len(), 1);
    assert_eq!(registry.packages[0].name, "LargePackageExample");
    assert_eq!(registry.packages[0].modules.len(), 9);

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

    // Validate code loaded as expected.
    assert_success!(context.harness.run_entry_function(
        &acc,
        str::parse(&format!("{}::eight::hello", module_address)).unwrap(),
        vec![],
        vec![bcs::to_bytes::<u64>(&42).unwrap()]
    ));

    let state = context
        .harness
        .read_resource::<State>(
            acc.address(),
            parse_struct_tag(&format!("{}::eight::State", module_address)).unwrap(),
        )
        .unwrap();

    assert_eq!(state.value, 42);
}

#[test]
fn large_package_object_code_deployment_upgrade_success_compat() {
    let mut context = LargePackageTestContext::new();
    let acc = context.account.clone();

    // Initial version
    let tx_statuses = context.publish_large_package(
        &acc,
        &common::test_dir_path("../../../move-examples/large_packages/large_package_example"),
        |_| {},
        PublishType::ObjectDeploy,
    );
    for tx_status in tx_statuses.into_iter() {
        assert_success!(tx_status);
    }

    // Upgrade to compatible version
    let tx_statuses = context.publish_large_package(
        &acc,
        &common::test_dir_path("../../../move-examples/large_packages/large_package_example"), // upgrade with the same package
        |_| {},
        PublishType::ObjectUpgrade,
    );
    for tx_status in tx_statuses.into_iter() {
        assert_success!(tx_status);
    }
}
