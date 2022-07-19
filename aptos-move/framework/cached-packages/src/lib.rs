// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use include_dir::{include_dir, Dir, DirEntry};
use move_deps::{move_binary_format::file_format::CompiledModule, move_core_types::abi::ScriptABI};
use once_cell::sync::Lazy;
use std::collections::HashSet;

pub mod aptos_stdlib;
pub mod aptos_token_stdlib;
mod generated_aptos_txn_builder;
mod generated_token_txn_builder;

static PACKAGE: Dir<'static> = include_dir!("$OUT_DIR");

static MODULE_BLOBS: Lazy<Vec<Vec<u8>>> = Lazy::new(load_modules);

static MODULES: Lazy<Vec<CompiledModule>> = Lazy::new(|| {
    MODULE_BLOBS
        .iter()
        .map(|blob| CompiledModule::deserialize(blob).unwrap())
        .collect()
});

static ABIS: Lazy<Vec<ScriptABI>> = Lazy::new(load_abis);

pub fn abis() -> Vec<ScriptABI> {
    ABIS.clone()
}

pub fn load_abis() -> Vec<ScriptABI> {
    PACKAGE
        .find("**/*abis/*.abi")
        .unwrap()
        .filter_map(|file_module| match file_module {
            DirEntry::File(file) if !file.path().to_str().unwrap().contains("Genesis") => {
                Some(bcs::from_bytes::<ScriptABI>(file.contents()).unwrap())
            }
            _ => None,
        })
        .collect::<Vec<_>>()
}

fn load_modules() -> Vec<Vec<u8>> {
    let modules: Vec<CompiledModule> = PACKAGE
        .find("**/*.mv")
        .unwrap()
        .filter_map(|file_module| match file_module {
            DirEntry::Dir(_) => None,
            DirEntry::File(file) => Some(CompiledModule::deserialize(file.contents()).unwrap()),
        })
        .collect();
    let mut unique_modules = Vec::new();
    let mut module_keys = HashSet::new();
    for m in modules {
        let key = m.self_id();
        if module_keys.insert(key) {
            unique_modules.push(m);
        }
    }

    unique_modules
        .into_iter()
        .map(|module| {
            let mut bytes = vec![];
            module.serialize(&mut bytes).unwrap();
            bytes
        })
        .collect()
}

pub fn error_map() -> Vec<u8> {
    let error_vec = PACKAGE
        .find("**/error_description.errmap")
        .unwrap()
        .filter_map(|e| match e {
            DirEntry::Dir(_) => None,
            DirEntry::File(file) => Some(file.contents().to_vec()),
        })
        .flatten()
        .collect::<Vec<u8>>();
    error_vec
}

pub fn modules() -> &'static [CompiledModule] {
    &MODULES
}

pub fn modules_with_blobs() -> impl Iterator<Item = (&'static Vec<u8>, &'static CompiledModule)> {
    MODULE_BLOBS.iter().zip(MODULES.iter())
}

pub fn module_blobs() -> &'static [Vec<u8>] {
    &MODULE_BLOBS
}

#[test]
fn verify_load_framework() {
    module_blobs();
    error_map();
    /*
    std::fs::read(concat!(
        env!("OUT_DIR"),
        "/framework/transaction_script_builder.rs"
    ))
    .unwrap();
     */
}

#[test]
fn verify_load_token() {
    module_blobs();
    error_map();
    /*
    std::fs::read(concat!(
        env!("OUT_DIR"),
        "/token/transaction_script_builder.rs"
    ))
    .unwrap();
     */
}
