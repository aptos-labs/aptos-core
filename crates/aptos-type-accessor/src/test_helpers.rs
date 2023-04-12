// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use aptos_api_types::MoveModule;
use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_types::account_address::AccountAddress;
use std::{collections::BTreeMap, path::PathBuf};

// Given the name of a Move package directory, compile it and return the compiled modules.
// TODO: Consider making something that memoizes this so we don't have to recompile the
// same stuff for each test.
pub(crate) fn compile_package(path: PathBuf) -> Result<Vec<MoveModule>> {
    let mut named_addresses = BTreeMap::new();
    named_addresses.insert("addr".to_string(), AccountAddress::TWO);
    let build_options = BuildOptions {
        with_abis: true,
        named_addresses,
        ..Default::default()
    };
    let pack = BuiltPackage::build(path.clone(), build_options)
        .with_context(|| format!("Failed to build package at {}", path.to_string_lossy()))?;
    pack.extract_metadata_and_save()
        .context("Failed to extract metadata and save")?;
    let modules: Vec<MoveModule> = pack
        .modules()
        .cloned()
        .into_iter()
        .map(|m| m.into())
        .collect();
    Ok(modules)
}
