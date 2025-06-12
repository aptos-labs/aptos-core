// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_framework::natives::code::PackageMetadata;
use aptos_sdk::bcs;
use aptos_transaction_generator_lib::entry_point_trait::PreBuiltPackages;
use move_binary_format::{
    deserializer::DeserializerConfig,
    file_format::CompiledScript,
    file_format_common::{IDENTIFIER_SIZE_MAX, VERSION_DEFAULT, VERSION_MAX},
    CompiledModule,
};
use std::{fs, path::PathBuf};

#[derive(Debug)]
pub struct PreBuiltPackagesImpl;

impl PreBuiltPackages for PreBuiltPackagesImpl {
    fn package_metadata(&self, package_name: &str) -> PackageMetadata {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("prebuilt-packages")
            .join(package_name)
            .join("package_metadata");

        let bytes = fs::read(&path).unwrap_or_else(|err| panic!("Failed to read {path:?}: {err}"));

        bcs::from_bytes::<PackageMetadata>(&bytes).expect("Package metadata must deserialize")
    }

    fn package_modules(&self, package_name: &str) -> Vec<(String, CompiledModule, u32)> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("prebuilt-packages")
            .join(package_name)
            .join("modules");

        let default_config = DeserializerConfig::new(VERSION_DEFAULT, IDENTIFIER_SIZE_MAX);
        let mut results = vec![];

        let modules_dir = fs::read_dir(&path)
            .unwrap_or_else(|err| panic!("Failed to read modules from {path:?}: {err}"));
        for result in modules_dir {
            let entry = result.expect("Should be able to traverse modules");
            let module_path = entry.path();

            let is_valid = entry.file_type().map(|t| t.is_file()).unwrap_or(false)
                && module_path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case("mv"))
                    .unwrap_or(false);
            if !is_valid {
                panic!("Invalid module file: {module_path:?}");
            }

            let bytes = fs::read(&module_path)
                .unwrap_or_else(|err| panic!("Cannot read module file {module_path:?}: {err}"));
            let (module, binary_format_version) = if let Ok(module) =
                CompiledModule::deserialize_with_config(&bytes, &default_config)
            {
                (module, VERSION_DEFAULT)
            } else {
                let module =
                    CompiledModule::deserialize(&bytes).expect("Module must always deserialize");
                (module, VERSION_MAX)
            };

            results.push((
                module.self_id().name().to_string(),
                module,
                binary_format_version,
            ));
        }

        results
    }

    fn package_script(&self, package_name: &str) -> Option<CompiledScript> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("prebuilt-packages")
            .join(package_name)
            .join("scripts/script.mv");

        fs::read(&path)
            .ok()
            .map(|bytes| CompiledScript::deserialize(&bytes).expect("Script must deserialize"))
    }
}
