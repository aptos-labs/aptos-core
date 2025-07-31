// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::raw_module_data::{PACKAGE_TO_METADATA, PACKAGE_TO_MODULES, PACKAGE_TO_SCRIPT};
use anyhow::anyhow;
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

/// Directory where all pre-builds are saved: package metadata, compiled modules and scripts.
const PREBUILT_PACKAGES_DIR: &str = "prebuilt-packages";
/// Name of the file where package metadata is saved.
pub const PACKAGE_METADATA_FILE: &str = "package_metadata";
/// Name of the directory where modules are saved.
pub const MODULES_DIR: &str = "modules";
/// Name of the directory where scripts are saved.
pub const SCRIPTS_DIR: &str = "scripts";
/// Name of the script file. There is only 1 script per package allowed currently.
pub const SCRIPT_FILE: &str = "script";
/// Extension for module and script binaries.
pub const FILE_EXTENSION: &str = "mv";

#[derive(Debug)]
pub struct PreBuiltPackagesImpl;

fn package_path(package_name: &str) -> PathBuf {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(PREBUILT_PACKAGES_DIR)
        .join(package_name);
    std::path::absolute(path).expect("Should always be able to get the absolute path")
}

impl PreBuiltPackages for PreBuiltPackagesImpl {
    fn package_metadata_path(&self, package_name: &str) -> PathBuf {
        package_path(package_name).join(PACKAGE_METADATA_FILE)
    }

    fn package_modules_paths(
        &self,
        package_name: &str,
    ) -> anyhow::Result<Box<dyn Iterator<Item = PathBuf>>> {
        let path = package_path(package_name).join(MODULES_DIR);
        let modules_dir = fs::read_dir(&path)
            .map_err(|err| anyhow!("Failed to read modules from {path:?}: {err}"))?;

        Ok(Box::new(modules_dir.map(|result| {
            let entry = result.expect("Should be able to traverse modules");
            let module_path = entry.path();

            let is_valid = entry.file_type().map(|t| t.is_file()).unwrap_or(false)
                && module_path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case(FILE_EXTENSION))
                    .unwrap_or(false);
            if !is_valid {
                panic!("Invalid module file: {module_path:?}");
            }

            module_path
        })))
    }

    fn package_script_path(&self, package_name: &str) -> PathBuf {
        package_path(package_name)
            .join(SCRIPTS_DIR)
            .join(format!("{SCRIPT_FILE}.{FILE_EXTENSION}"))
    }

    fn package_metadata(&self, package_name: &str) -> PackageMetadata {
        let build_metadata = |bytes: &[u8]| {
            bcs::from_bytes::<PackageMetadata>(bytes).expect("Package metadata must deserialize")
        };

        let path = self.package_metadata_path(package_name);
        match fs::read(&path) {
            Ok(bytes) => build_metadata(&bytes),
            Err(_) => {
                let bytes = PACKAGE_TO_METADATA.get(package_name).expect(package_name);
                build_metadata(bytes)
            },
        }
    }

    fn package_modules(&self, package_name: &str) -> Vec<(String, CompiledModule, u32)> {
        let mut results = vec![];
        let default_config = DeserializerConfig::new(VERSION_DEFAULT, IDENTIFIER_SIZE_MAX);

        let mut build_module = |bytes: &[u8]| {
            let (module, binary_format_version) = if let Ok(module) =
                CompiledModule::deserialize_with_config(bytes, &default_config)
            {
                (module, VERSION_DEFAULT)
            } else {
                let module =
                    CompiledModule::deserialize(bytes).expect("Module must always deserialize");
                (module, VERSION_MAX)
            };
            results.push((
                module.self_id().name().to_string(),
                module,
                binary_format_version,
            ));
        };

        match self.package_modules_paths(package_name) {
            Ok(paths) => {
                for module_path in paths {
                    let bytes = fs::read(&module_path).unwrap_or_else(|err| {
                        panic!("Cannot read module file {module_path:?}: {err}")
                    });
                    build_module(&bytes);
                }
            },
            Err(_) => {
                let modules = PACKAGE_TO_MODULES.get(package_name).expect(package_name);
                for bytes in modules {
                    build_module(bytes);
                }
            },
        }

        results
    }

    fn package_script(&self, package_name: &str) -> Option<CompiledScript> {
        let build_script =
            |bytes: &[u8]| CompiledScript::deserialize(bytes).expect("Script must deserialize");

        let path = self.package_script_path(package_name);
        match fs::read(&path) {
            Ok(bytes) => Some(build_script(&bytes)),
            Err(_) => PACKAGE_TO_SCRIPT
                .get(package_name)
                .map(|code| build_script(code)),
        }
    }
}
