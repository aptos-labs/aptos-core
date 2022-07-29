// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use include_dir::{include_dir, Dir, DirEntry};
use move_deps::{
    move_binary_format::file_format::CompiledModule, move_bytecode_utils::Modules,
    move_core_types::abi::ScriptABI,
};
use once_cell::sync::Lazy;

pub mod aptos_framework_sdk_builder;
pub mod aptos_stdlib;
pub mod aptos_token_sdk_builder;

// ================================================================================
// Artifacts

static PACKAGE: Dir<'static> = include_dir!("$OUT_DIR/framework");

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

    #[test]
    fn token_sdk_up_to_date() {
        sdk_is_up_to_date(
            "aptos-token",
            "aptos_sdk_builder.rs",
            "aptos_token_sdk_builder.rs",
        )
    }
}
