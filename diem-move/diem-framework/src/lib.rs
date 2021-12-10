// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use move_binary_format::file_format::CompiledModule;
use move_command_line_common::files::{extension_equals, find_filenames, MOVE_EXTENSION};
use move_compiler::{
    compiled_unit::{CompiledUnit, NamedCompiledModule},
    shared::{NumberFormat, NumericalAddress},
};
use move_package::compilation::compiled_package::CompiledPackage;
use once_cell::sync::Lazy;
use std::{collections::BTreeMap, path::PathBuf};
use tempfile::tempdir;

pub mod natives;
pub mod release;

const CORE_MODULES_DIR: &str = "core/sources";
const DPN_MODULES_DIR: &str = "DPN/sources";

pub fn path_in_crate<S>(relative: S) -> PathBuf
where
    S: Into<String>,
{
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push(relative.into());
    path
}

pub fn diem_core_modules_full_path() -> String {
    format!("{}/{}", env!("CARGO_MANIFEST_DIR"), CORE_MODULES_DIR)
}

pub fn diem_payment_modules_full_path() -> String {
    format!("{}/{}", env!("CARGO_MANIFEST_DIR"), DPN_MODULES_DIR)
}

pub fn diem_stdlib_files_no_dependencies() -> Vec<String> {
    let diem_core_modules_path = path_in_crate(CORE_MODULES_DIR);
    let diem_payment_modules_path = path_in_crate(DPN_MODULES_DIR);
    find_filenames(&[diem_core_modules_path, diem_payment_modules_path], |p| {
        extension_equals(p, MOVE_EXTENSION)
    })
    .unwrap()
}

pub fn diem_stdlib_files() -> Vec<String> {
    let mut files = move_stdlib::move_stdlib_files();
    files.extend(diem_stdlib_files_no_dependencies());
    files
}

pub fn diem_framework_named_addresses() -> BTreeMap<String, NumericalAddress> {
    DPN_FRAMEWORK_PKG
        .compiled_package_info
        .address_alias_instantiation
        .iter()
        .map(|(name, addr)| {
            (
                name.to_string(),
                NumericalAddress::new(addr.into_bytes(), NumberFormat::Hex),
            )
        })
        .collect()
}

static DPN_FRAMEWORK_PKG: Lazy<CompiledPackage> = Lazy::new(|| {
    let build_config = move_package::BuildConfig {
        install_dir: Some(tempdir().unwrap().path().to_path_buf()),
        ..Default::default()
    };
    build_config
        .compile_package(&path_in_crate("DPN"), &mut Vec::new())
        .unwrap()
});

pub fn modules() -> Vec<CompiledModule> {
    DPN_FRAMEWORK_PKG
        .transitive_compiled_units()
        .iter()
        .filter_map(|unit| match unit {
            CompiledUnit::Module(NamedCompiledModule { module, .. }) => Some(module.clone()),
            CompiledUnit::Script(_) => None,
        })
        .collect()
}

pub fn module_blobs() -> Vec<Vec<u8>> {
    DPN_FRAMEWORK_PKG
        .transitive_compiled_units()
        .iter()
        .filter_map(|unit| match unit {
            CompiledUnit::Module(NamedCompiledModule { module, .. }) => {
                let mut bytes = vec![];
                module.serialize(&mut bytes).unwrap();
                Some(bytes)
            }
            CompiledUnit::Script(_) => None,
        })
        .collect()
}
