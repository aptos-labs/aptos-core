// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Helpers for writing Move tests

use anyhow::Result;
use aptos::common::types::MovePackageDir;
use aptos::move_tool::BuiltPackage;
use aptos_sdk::transaction_builder::TransactionFactory;
use forge::AptosContext;
use framework::natives::code::UpgradePolicy;
use std::path::PathBuf;

/// New style publishing via `code::publish_package`
pub async fn publish_package(
    ctx: &mut AptosContext<'_>,
    move_dir: PathBuf,
    upgrade_policy: UpgradePolicy,
) -> Result<TransactionFactory> {
    let package = BuiltPackage::build(MovePackageDir::new(move_dir), true, true)?;
    let blobs = package.extract_code();
    let metadata = package.extract_metadata(upgrade_policy)?;
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
