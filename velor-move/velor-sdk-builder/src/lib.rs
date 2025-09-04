// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use velor_types::transaction::EntryABI;
use std::{ffi::OsStr, fs, io::Read, path::Path};

pub mod golang;
pub mod rust;

/// Internals shared between languages.
mod common;

fn get_abi_paths(dir: &Path) -> std::io::Result<Vec<String>> {
    let mut abi_paths = Vec::new();
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                abi_paths.append(&mut get_abi_paths(&path)?);
            } else if let Some("abi") = path.extension().and_then(OsStr::to_str) {
                abi_paths.push(path.to_str().unwrap().to_string());
            }
        }
    }
    Ok(abi_paths)
}

/// Read all ABI files the specified directories. This supports both new and old `EntryABI`s.
pub fn read_abis(dir_paths: &[impl AsRef<Path>]) -> anyhow::Result<Vec<EntryABI>> {
    let mut abis = Vec::<EntryABI>::new();
    for dir in dir_paths.iter() {
        for path in get_abi_paths(dir.as_ref())? {
            let mut buffer = Vec::new();
            let mut f = std::fs::File::open(path)?;
            f.read_to_end(&mut buffer)?;
            abis.push(bcs::from_bytes(&buffer)?);
        }
    }

    // Sort functions by (module, function) lexicographical order
    #[allow(clippy::unnecessary_sort_by)]
    abis.sort_by(|a, b| {
        let a0 = match a {
            EntryABI::EntryFunction(sf) => sf.module_name().to_string(),
            _ => "".to_owned(),
        };
        let b0 = match b {
            EntryABI::EntryFunction(sf) => sf.module_name().to_string(),
            _ => "".to_owned(),
        };

        (a0, a.name()).cmp(&(b0, b.name()))
    });
    Ok(abis)
}

/// How to copy ABI-generated source code for a given language.
pub trait SourceInstaller {
    type Error;

    /// Create a module exposing the transaction builders for the given ABIs.
    fn install_transaction_builders(
        &self,
        name: &str,
        abis: &[EntryABI],
    ) -> std::result::Result<(), Self::Error>;
}
