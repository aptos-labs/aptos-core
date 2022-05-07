// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use move_deps::{
    move_binary_format::file_format::CompiledModule,
    move_compiler::{
        compiled_unit::{CompiledUnit, NamedCompiledModule},
        shared::NumericalAddress,
    },
    move_package::compilation::compiled_package::CompiledPackage,
};
use once_cell::sync::Lazy;
use std::collections::BTreeMap;

const APTOS_MODULES_DIR: &str = "aptos-framework/sources";
static APTOS_PKG: Lazy<CompiledPackage> = Lazy::new(|| super::package("aptos-framework"));

pub fn files() -> Vec<String> {
    let mut files = move_deps::move_stdlib::move_stdlib_files();
    files.extend(super::move_files_in_path(APTOS_MODULES_DIR));
    files
}

pub fn module_blobs() -> Vec<Vec<u8>> {
    super::module_blobs(&*APTOS_PKG)
}

pub fn named_addresses() -> BTreeMap<String, NumericalAddress> {
    super::named_addresses(&*APTOS_PKG)
}

pub fn modules() -> Vec<CompiledModule> {
    APTOS_PKG
        .transitive_compiled_units()
        .iter()
        .filter_map(|unit| match unit {
            CompiledUnit::Module(NamedCompiledModule { module, .. }) => Some(module.clone()),
            CompiledUnit::Script(_) => None,
        })
        .collect()
}
