// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_transaction_generator_lib::entry_point_trait::PreBuiltPackages;
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

    fn package_modules_paths(&self, package_name: &str) -> Box<dyn Iterator<Item = PathBuf>> {
        let path = package_path(package_name).join(MODULES_DIR);
        let modules_dir = fs::read_dir(&path)
            .unwrap_or_else(|err| panic!("Failed to read modules from {path:?}: {err}"));

        Box::new(modules_dir.map(|result| {
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
        }))
    }

    fn package_script_path(&self, package_name: &str) -> PathBuf {
        package_path(package_name)
            .join(SCRIPTS_DIR)
            .join(format!("{SCRIPT_FILE}.{FILE_EXTENSION}"))
    }
}
