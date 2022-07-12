// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Helpers for writing Move tests

use anyhow::Result;
use aptos_sdk::transaction_builder::TransactionFactory;
use aptos_types::transaction::{ModuleBundle, TransactionPayload};
use forge::AptosContext;
use move_deps::move_package;
use std::path::PathBuf;

pub async fn publish_code(
    ctx: &mut AptosContext<'_>,
    move_dir: PathBuf,
) -> Result<TransactionFactory> {
    let build_config = move_package::BuildConfig {
        generate_docs: true,
        generate_abis: true,
        install_dir: Some(move_dir.clone()),
        ..Default::default()
    };

    let compiled_package = build_config
        .clone()
        .compile_package(&move_dir, &mut std::io::stderr())
        .unwrap();

    let mut blobs = vec![];
    compiled_package
        .root_modules_map()
        .iter_modules()
        .first()
        .unwrap()
        .serialize(&mut blobs)
        .unwrap();

    let txn_factory = ctx.aptos_transaction_factory();
    let publish_txn = ctx
        .root_account()
        .sign_with_transaction_builder(txn_factory.payload(TransactionPayload::ModuleBundle(
            ModuleBundle::singleton(blobs.clone()),
        )));
    ctx.client().submit_and_wait(&publish_txn).await?;
    Ok(txn_factory)
}
