// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Helpers for writing Move tests

use anyhow::Result;
use aptos::common::types::MovePackageDir;
use aptos::move_tool::BuiltPackage;
use aptos_sdk::transaction_builder::TransactionFactory;
use aptos_types::transaction::{ModuleBundle, TransactionPayload};
use aptos_vm::move_vm_ext::UpgradePolicy;
use forge::AptosContext;
use std::path::PathBuf;

/// Old style publishing via module bundle.
pub async fn publish_code(
    ctx: &mut AptosContext<'_>,
    move_dir: PathBuf,
) -> Result<TransactionFactory> {
    let package = BuiltPackage::build(MovePackageDir::new(move_dir), true, true)?;
    // This used to take only the first of the modules, so we are continuing to do this.
    let blobs = package.extract_code().into_iter().next().unwrap();
    let txn_factory = ctx.aptos_transaction_factory();
    let publish_txn = ctx
        .root_account()
        .sign_with_transaction_builder(txn_factory.payload(TransactionPayload::ModuleBundle(
            ModuleBundle::singleton(blobs),
        )));
    ctx.client().submit_and_wait(&publish_txn).await?;
    Ok(txn_factory)
}

/// New style publishing via `code::publish_package`
pub async fn publish_package(
    ctx: &mut AptosContext<'_>,
    move_dir: PathBuf,
    package_name: impl Into<String>,
    upgrade_policy: UpgradePolicy,
) -> Result<TransactionFactory> {
    let package = BuiltPackage::build(MovePackageDir::new(move_dir), true, true)?;
    let blobs = package.extract_code();
    let metadata = package.extract_metadata(package_name.into(), upgrade_policy)?;
    let payload = aptos_transaction_builder::aptos_stdlib::code_publish_package_txn(
        bcs::to_bytes(&metadata).expect("PackageMetadata has BCS"),
        blobs,
    );
    let txn_factory = ctx.aptos_transaction_factory();
    let publish_txn = ctx
        .root_account()
        .sign_with_transaction_builder(txn_factory.payload(payload));
    ctx.client().submit_and_wait(&publish_txn).await?;
    Ok(txn_factory)
}
