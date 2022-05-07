// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use move_deps::{
    move_command_line_common::files::{
        extension_equals, find_filenames, MOVE_COMPILED_EXTENSION, MOVE_EXTENSION,
    },
    move_compiler::{
        compiled_unit::{CompiledUnit, NamedCompiledModule},
        shared::{NumberFormat, NumericalAddress},
    },
    move_package::compilation::compiled_package::CompiledPackage,
};
use std::{collections::BTreeMap, path::PathBuf};
use tempfile::tempdir;

pub mod aptos;
pub mod natives;
pub mod release;

const CORE_MODULES_DIR: &str = "core/sources";

pub fn path_in_crate<S>(relative: S) -> PathBuf
where
    S: Into<String>,
{
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push(relative.into());
    path
}

pub fn core_modules_full_path() -> String {
    format!("{}/{}", env!("CARGO_MANIFEST_DIR"), CORE_MODULES_DIR)
}

/// Load the serialized modules from the specified paths.
pub fn load_modules_from_paths(paths: &[PathBuf]) -> Vec<Vec<u8>> {
    find_filenames(paths, |path| {
        extension_equals(path, MOVE_COMPILED_EXTENSION)
    })
    .expect("module loading failed")
    .iter()
    .map(|file_name| std::fs::read(file_name).unwrap())
    .collect::<Vec<_>>()
}

pub(crate) fn module_blobs(pkg: &CompiledPackage) -> Vec<Vec<u8>> {
    pkg.transitive_compiled_units()
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

pub(crate) fn move_files_in_path(path: &str) -> Vec<String> {
    let modules_path = path_in_crate(path);
    find_filenames(&[modules_path], |p| extension_equals(p, MOVE_EXTENSION)).unwrap()
}

pub(crate) fn named_addresses(pkg: &CompiledPackage) -> BTreeMap<String, NumericalAddress> {
    pkg.compiled_package_info
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

pub(crate) fn package(name: &str) -> CompiledPackage {
    let build_config = move_deps::move_package::BuildConfig {
        install_dir: Some(tempdir().unwrap().path().to_path_buf()),
        ..Default::default()
    };
    build_config
        .compile_package(&path_in_crate(name), &mut Vec::new())
        .unwrap()
}
