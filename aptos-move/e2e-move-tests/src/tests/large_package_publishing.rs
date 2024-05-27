// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    assert_move_abort, assert_success, assert_vm_status, build_package, tests::common, MoveHarness,
};
use aptos_framework::{
    natives::{
        code::{PackageMetadata, PackageRegistry, UpgradePolicy},
        object_code_deployment::ManagingRefs,
    },
    BuildOptions, BuiltPackage,
};
use aptos_language_e2e_tests::account::Account;
use aptos_types::{
    object_address::create_object_code_deployment_address,
    transaction::{
        AbortInfo, EntryFunction, SignedTransaction, TransactionPayload, TransactionStatus,
    },
};
use move_core_types::{
    account_address::AccountAddress, ident_str, language_storage::ModuleId,
    parser::parse_struct_tag, vm_status::StatusCode,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    option::Option,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

/// Maximum code & metadata chunk size to be included in a transaction
const MAX_CHUNK_SIZE_IN_BYTES: usize = 60_000;

/// Number of transactions needed for staging code chunks before publishing to accounts or objects
/// This is used to derive object address for testing object code deployment feature
const NUMBER_OF_TRANSACTIONS_FOR_STAGING: u64 = 2;

static CACHED_BUILT_PACKAGES: Lazy<Mutex<HashMap<PathBuf, Arc<anyhow::Result<BuiltPackage>>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Mimics `0xcafe::eight::State`
#[derive(Serialize, Deserialize)]
struct State {
    value: u64,
}

struct LargePackageTestContext {
    harness: MoveHarness,
    admin_account: Account, // publish `large_packages.move` under this account
    account: Account,       // publish the large package under this account
    object_address: AccountAddress, // used for testing object code deployment for large packages
}

enum PackagePublishMode {
    AccountDeploy,
    ObjectDeploy,
    ObjectUpgrade,
}

impl LargePackageTestContext {
    /// Create a new test context with initialized accounts and published `large_packages.move` module.
    fn new() -> Self {
        let mut harness = MoveHarness::new();
        let admin_account =
            harness.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());
        let account = harness.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());
        let sequence_number = harness.sequence_number(account.address());
        let object_address = create_object_code_deployment_address(
            *account.address(),
            sequence_number + NUMBER_OF_TRANSACTIONS_FOR_STAGING + 1,
        );

        // publish `large_packages.move` module
        let build_option = Self::get_named_addresses_build_options(vec![(
            String::from("large_packages"),
            AccountAddress::from_hex_literal("0xbeef").unwrap(),
        )]);

        let txn = harness.create_publish_package(
            &admin_account,
            &common::test_dir_path("../../../move-examples/large_packages"),
            Some(build_option),
            |_| {},
        );
        assert_success!(harness.run(txn));

        LargePackageTestContext {
            harness,
            admin_account,
            account,
            object_address,
        }
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

        build_options
    }

    /// Publish a large package by creating and running the necessary transactions.
    fn publish_large_package(
        &mut self,
        account: &Account,
        path: &Path,
        patch_metadata: impl FnMut(&mut PackageMetadata),
        package_publish_mode: PackagePublishMode,
    ) -> Vec<TransactionStatus> {
        let deploy_address = match package_publish_mode {
            PackagePublishMode::AccountDeploy => {
                AccountAddress::from_hex_literal("0xcafe").unwrap()
            },
            PackagePublishMode::ObjectDeploy | PackagePublishMode::ObjectUpgrade => {
                self.object_address
            },
        };

        let build_options = Self::get_named_addresses_build_options(vec![(
            String::from("large_package_example"),
            deploy_address,
        )]);
        let transactions = self.create_publish_large_package_from_path(
            account,
            path,
            Some(build_options),
            patch_metadata,
            package_publish_mode,
        );
        transactions
            .into_iter()
            .map(|txn| self.harness.run(txn))
            .collect()
    }

    /// Create transactions for publishing a large package.
    fn create_publish_large_package_from_path(
        &mut self,
        account: &Account,
        path: &Path,
        options: Option<BuildOptions>,
        patch_metadata: impl FnMut(&mut PackageMetadata),
        package_publish_mode: PackagePublishMode,
    ) -> Vec<SignedTransaction> {
        let package_arc = {
            let mut cache = CACHED_BUILT_PACKAGES.lock().unwrap();
            Arc::clone(
                cache
                    .entry(path.to_owned())
                    .or_insert_with(|| Arc::new(build_package(path.to_owned(), options.unwrap()))),
            )
        };
        let package_ref = package_arc
            .as_ref()
            .as_ref()
            .expect("building package must succeed");
        let package_code = package_ref.extract_code();
        let metadata = package_ref
            .extract_metadata()
            .expect("extracting package metadata must succeed");
        self.create_payloads_from_metadata_and_code(
            account,
            package_code,
            metadata,
            patch_metadata,
            package_publish_mode,
        )
    }

    /// Create payloads from metadata and code chunks for a large package.
    fn create_payloads_from_metadata_and_code(
        &mut self,
        account: &Account,
        package_code: Vec<Vec<u8>>,
        mut metadata: PackageMetadata,
        mut patch_metadata: impl FnMut(&mut PackageMetadata),
        package_publish_mode: PackagePublishMode,
    ) -> Vec<SignedTransaction> {
        patch_metadata(&mut metadata);

        // Chunk the metadata
        let mut metadata_chunks =
            Self::create_chunks(bcs::to_bytes(&metadata).expect("Failed deserializing metadata"));

        // Separate last chunk for special handling
        let mut metadata_chunk = metadata_chunks.pop().unwrap_or_default();
        let mut taken_size = metadata_chunk.len();

        let mut transactions = metadata_chunks
            .into_iter()
            .map(|chunk| {
                self.harness.create_transaction_payload(
                    account,
                    self.large_packages_stage_code_chunk(
                        chunk,
                        vec![],
                        vec![],
                        false,
                        false,
                        false,
                        None,
                    ),
                )
            })
            .collect::<Vec<_>>();

        let mut code_indices: Vec<u16> = vec![];
        let mut code_chunks: Vec<Vec<u8>> = vec![];

        for (idx, module_code) in package_code.into_iter().enumerate() {
            let chunked_module = Self::create_chunks(module_code);
            for chunk in chunked_module {
                if taken_size + chunk.len() > MAX_CHUNK_SIZE_IN_BYTES {
                    // Create a payload and reset accumulators
                    let transaction = self.harness.create_transaction_payload(
                        account,
                        self.large_packages_stage_code_chunk(
                            metadata_chunk,
                            code_indices.clone(),
                            code_chunks.clone(),
                            false,
                            false,
                            false,
                            None,
                        ),
                    );
                    transactions.push(transaction);

                    metadata_chunk = vec![];
                    code_indices.clear();
                    code_chunks.clear();
                    taken_size = 0;
                }

                if !code_indices.contains(&(idx as u16)) {
                    code_indices.push(idx as u16);
                }
                taken_size += chunk.len();
                code_chunks.push(chunk);
            }
        }

        let (is_account_deploy, is_object_deploy, is_object_upgrade, object_address) =
            match package_publish_mode {
                PackagePublishMode::AccountDeploy => (true, false, false, None),
                PackagePublishMode::ObjectDeploy => (false, true, false, None),
                PackagePublishMode::ObjectUpgrade => {
                    (false, false, true, Some(self.object_address))
                },
            };

        // Add the last payload (publishing transaction)
        let transaction = self.harness.create_transaction_payload(
            account,
            self.large_packages_stage_code_chunk(
                metadata_chunk,
                code_indices,
                code_chunks,
                is_account_deploy,
                is_object_deploy,
                is_object_upgrade,
                object_address,
            ),
        );
        transactions.push(transaction);

        transactions
    }

    /// Create chunks of data based on the defined maximum chunk size.
    fn create_chunks(data: Vec<u8>) -> Vec<Vec<u8>> {
        data.chunks(MAX_CHUNK_SIZE_IN_BYTES)
            .map(|chunk| chunk.to_vec())
            .collect()
    }

    /// Create a transaction payload for staging or publishing the large package.
    fn large_packages_stage_code_chunk(
        &self,
        metadata_chunk: Vec<u8>,
        code_indices: Vec<u16>,
        code_chunks: Vec<Vec<u8>>,
        is_account_deploy: bool,
        is_object_deploy: bool,
        is_object_upgrade: bool,
        code_object: Option<AccountAddress>,
    ) -> TransactionPayload {
        TransactionPayload::EntryFunction(EntryFunction::new(
            ModuleId::new(
                self.admin_account.address().to_owned(),
                ident_str!("large_packages").to_owned(),
            ),
            ident_str!("stage_code_chunk").to_owned(),
            vec![],
            vec![
                bcs::to_bytes(&metadata_chunk).unwrap(),
                bcs::to_bytes(&code_indices).unwrap(),
                bcs::to_bytes(&code_chunks).unwrap(),
                bcs::to_bytes(&is_account_deploy).unwrap(),
                bcs::to_bytes(&is_object_deploy).unwrap(),
                bcs::to_bytes(&is_object_upgrade).unwrap(),
                bcs::to_bytes(&code_object).unwrap(),
            ],
        ))
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
        PackagePublishMode::AccountDeploy,
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
        PackagePublishMode::AccountDeploy,
    );
    for tx_status in tx_statuses.into_iter() {
        assert_success!(tx_status);
    }

    // Upgrade to compatible version
    let tx_statuses = context.publish_large_package(
        &acc,
        &common::test_dir_path("../../../move-examples/large_packages/large_package_example"), // upgrade with the same package
        |_| {},
        PackagePublishMode::AccountDeploy,
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
        PackagePublishMode::AccountDeploy,
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
        PackagePublishMode::AccountDeploy,
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
        PackagePublishMode::AccountDeploy,
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
        PackagePublishMode::AccountDeploy,
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
        PackagePublishMode::AccountDeploy,
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
        PackagePublishMode::AccountDeploy,
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
        PackagePublishMode::ObjectDeploy,
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
        PackagePublishMode::ObjectDeploy,
    );
    for tx_status in tx_statuses.into_iter() {
        assert_success!(tx_status);
    }

    // Upgrade to compatible version
    let tx_statuses = context.publish_large_package(
        &acc,
        &common::test_dir_path("../../../move-examples/large_packages/large_package_example"), // upgrade with the same package
        |_| {},
        PackagePublishMode::ObjectUpgrade,
    );
    for tx_status in tx_statuses.into_iter() {
        assert_success!(tx_status);
    }
}
