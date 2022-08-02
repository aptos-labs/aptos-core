// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use include_dir::{include_dir, Dir, DirEntry};
use move_deps::{move_binary_format::file_format::CompiledModule, move_core_types::abi::ScriptABI};
use once_cell::sync::Lazy;
use std::collections::HashSet;

pub mod aptos_framework_sdk_builder;
pub mod aptos_stdlib;
pub mod aptos_token_sdk_builder;

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

pub fn error_map() -> Vec<Vec<u8>> {
    PACKAGE
        .find("**/error_description.errmap")
        .unwrap()
        .filter_map(|e| match e {
            DirEntry::Dir(_) => None,
            DirEntry::File(file) => Some(file.contents().to_vec()),
        })
        .collect()
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
}

#[test]
fn verify_load_token() {
    module_blobs();
    error_map();
}

// ================================================================================
// SDK builders

#[cfg(test)]
mod tests {
    use move_deps::move_prover_test_utils::baseline_test::verify_or_update_baseline;
    use std::path::PathBuf;

    fn sdk_is_up_to_date(package_name: &str, built_file: &str, target_file: &str) {
        let tempdir = tempfile::tempdir().unwrap();
        let release = framework::release::ReleaseOptions {
            no_check_layout_compatibility: false,
            no_build_docs: true,
            with_diagram: false,
            no_script_builder: false,
            no_script_abis: false,
            no_errmap: true,
            package: PathBuf::from(package_name),
            output: tempdir.path().to_path_buf(),
        };
        release.create_release();
        let current_content = std::fs::read_to_string(tempdir.path().join(built_file)).unwrap();
        let target_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join(target_file);
        if verify_or_update_baseline(&target_path, &current_content).is_err() {
            // Don't use original output of baseline test but specific one for this scenario
            panic!(
                "`{}` out of date. Please run `UPBL=1 cargo test -p cached-framework-packages`",
                target_file
            )
        }
    }

    #[test]
    fn aptos_sdk_up_to_date() {
        sdk_is_up_to_date(
            "aptos-framework",
            "aptos_sdk_builder.rs",
            "aptos_framework_sdk_builder.rs",
        )
    }

    #[ignore] // re-enable after string is supported in abigen
    #[test]
    fn token_sdk_up_to_date() {
        sdk_is_up_to_date(
            "aptos-token",
            "aptos_sdk_builder.rs",
            "aptos_token_sdk_builder.rs",
        )
    }
}
