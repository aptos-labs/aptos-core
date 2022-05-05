// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use include_dir::{include_dir, Dir, DirEntry};
use move_deps::{
    move_binary_format::file_format::CompiledModule, move_bytecode_utils::Modules,
    move_core_types::abi::ScriptABI,
};
use once_cell::sync::Lazy;

pub mod aptos_stdlib;

static PACKAGE: Dir<'static> = include_dir!("$OUT_DIR");

static MODULE_BLOBS: Lazy<Vec<Vec<u8>>> = Lazy::new(|| load_modules("build"));

static MODULES: Lazy<Vec<CompiledModule>> = Lazy::new(|| {
    MODULE_BLOBS
        .iter()
        .map(|blob| CompiledModule::deserialize(blob).unwrap())
        .collect()
});

static ABIS: Lazy<Vec<ScriptABI>> = Lazy::new(|| load_abis("build"));

pub fn abis() -> Vec<ScriptABI> {
    ABIS.clone()
}

pub fn load_abis(path: &str) -> Vec<ScriptABI> {
    PACKAGE
        .get_dir(path)
        .unwrap()
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

fn load_modules(path: &str) -> Vec<Vec<u8>> {
    let modules = PACKAGE
        .get_dir(path)
        .unwrap()
        .find("**/*modules/*.mv")
        .unwrap()
        .filter_map(|file_module| match file_module {
            DirEntry::Dir(_) => None,
            DirEntry::File(file) => Some(CompiledModule::deserialize(file.contents()).unwrap()),
        })
        .collect::<Vec<_>>();

    Modules::new(modules.iter())
        .compute_dependency_graph()
        .compute_topological_order()
        .unwrap()
        .into_iter()
        .map(|module| {
            let mut bytes = vec![];
            module.serialize(&mut bytes).unwrap();
            bytes
        })
        .collect()
}

pub fn error_map() -> &'static [u8] {
    PACKAGE
        .get_file("error_description/error_description.errmap")
        .unwrap()
        .contents()
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
fn verify_load() {
    module_blobs();
    error_map();
    std::fs::read(concat!(env!("OUT_DIR"), "/transaction_script_builder.rs")).unwrap();
}
