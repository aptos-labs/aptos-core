// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    assert_move_abort, assert_success, assert_vm_status, build_package, tests::common, MoveHarness,
};
use aptos_framework::{
    natives::code::{PackageMetadata, PackageRegistry, UpgradePolicy},
    BuildOptions, BuiltPackage,
};
use aptos_language_e2e_tests::account::Account;
use aptos_types::transaction::{
    AbortInfo, EntryFunction, SignedTransaction, TransactionPayload, TransactionStatus,
};
use move_core_types::{
    account_address::AccountAddress, ident_str, language_storage::ModuleId,
    parser::parse_struct_tag, vm_status::StatusCode,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

/// Mimics `0xcafe::eight::State`
#[derive(Serialize, Deserialize)]
struct State {
    value: u64,
}

struct LargePackageTestContext {
    harness: MoveHarness,
    admin_account: Account, // publish `large_packages.move` under this account
    account: Account,       // publish the large package under this account
}

static CACHED_BUILT_PACKAGES: Lazy<Mutex<HashMap<PathBuf, Arc<anyhow::Result<BuiltPackage>>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

const MAX_CHUNK_SIZE: usize = 30_000;

impl LargePackageTestContext {
    /// Create a new test context with initialized accounts and published `large_packages.move` module.
    fn new() -> Self {
        let mut harness = MoveHarness::new();
        let admin_account =
            harness.new_account_at(AccountAddress::from_hex_literal("0xbeef").unwrap());
        let account = harness.new_account_at(AccountAddress::from_hex_literal("0xcafe").unwrap());

        let build_option = Self::get_build_options_with_named_addresses(vec![(
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
        }
    }

    fn get_build_options_with_named_addresses(
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
    ) -> Vec<TransactionStatus> {
        let build_options = Self::get_build_options_with_named_addresses(vec![(
            String::from("large_package_example"),
            AccountAddress::from_hex_literal("0xcafe").unwrap(),
        )]);
        let transactions = self.create_publish_large_package_from_path(
            account,
            path,
            Some(build_options),
            patch_metadata,
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
    ) -> Vec<SignedTransaction> {
        let package_arc = {
            let mut cache = CACHED_BUILT_PACKAGES.lock().unwrap();
            Arc::clone(cache.entry(path.to_owned()).or_insert_with(|| {
                Arc::new(build_package(path.to_owned(), options.unwrap_or_default()))
            }))
        };
        let package_ref = package_arc
            .as_ref()
            .as_ref()
            .expect("building package must succeed");
        let package_code = package_ref.extract_code();
        let metadata = package_ref
            .extract_metadata()
            .expect("extracting package metadata must succeed");
        self.create_payloads_from_metadata_and_code(account, package_code, metadata, patch_metadata)
    }

    /// Create payloads from metadata and code chunks for a large package.
    fn create_payloads_from_metadata_and_code(
        &mut self,
        account: &Account,
        package_code: Vec<Vec<u8>>,
        mut metadata: PackageMetadata,
        mut patch_metadata: impl FnMut(&mut PackageMetadata),
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
                    self.large_packages_stage_code(chunk, vec![], vec![], false),
                )
            })
            .collect::<Vec<_>>();

        let mut code_indices: Vec<u16> = vec![];
        let mut code_chunks: Vec<Vec<u8>> = vec![];

        for (idx, module_code) in package_code.into_iter().enumerate() {
            let chunked_module = Self::create_chunks(module_code);
            for chunk in chunked_module {
                if taken_size + chunk.len() > MAX_CHUNK_SIZE {
                    // Create a payload and reset accumulators
                    let transaction = self.harness.create_transaction_payload(
                        account,
                        self.large_packages_stage_code(
                            metadata_chunk,
                            code_indices.clone(),
                            code_chunks.clone(),
                            false,
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

        // Add the last payload (publishing transaction)
        let transaction = self.harness.create_transaction_payload(
            account,
            self.large_packages_stage_code(metadata_chunk, code_indices, code_chunks, true),
        );
        transactions.push(transaction);

        transactions
    }

    /// Create chunks of data based on the defined maximum chunk size.
    fn create_chunks(data: Vec<u8>) -> Vec<Vec<u8>> {
        data.chunks(MAX_CHUNK_SIZE)
            .map(|chunk| chunk.to_vec())
            .collect()
    }

    /// Create a transaction payload for staging or publishing the large package.
    fn large_packages_stage_code(
        &self,
        metadata_chunk: Vec<u8>,
        code_indices: Vec<u16>,
        code_chunks: Vec<Vec<u8>>,
        publish: bool,
    ) -> TransactionPayload {
        TransactionPayload::EntryFunction(EntryFunction::new(
            ModuleId::new(
                self.admin_account.address().to_owned(),
                ident_str!("large_packages").to_owned(),
            ),
            ident_str!("stage_code").to_owned(),
            vec![],
            vec![
                bcs::to_bytes(&metadata_chunk).unwrap(),
                bcs::to_bytes(&code_indices).unwrap(),
                bcs::to_bytes(&code_chunks).unwrap(),
                bcs::to_bytes(&publish).unwrap(),
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
    assert_eq!(registry.packages[0].modules.len(), 9);

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
    );
    for tx_status in tx_statuses.into_iter() {
        assert_success!(tx_status);
    }

    // Upgrade to compatible version
    let tx_statuses = context.publish_large_package(
        &acc,
        &common::test_dir_path("../../../move-examples/large_packages/large_package_example"), // upgrade with the same package
        |_| {},
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
