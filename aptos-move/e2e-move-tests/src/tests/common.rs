// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_framework::BuiltPackage;
use std::{collections::BTreeMap, path::PathBuf};

pub fn test_dir_path(s: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("tests")
        .join(s)
}

pub fn framework_dir_path(s: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("framework")
        .join(s)
}

pub fn build_scripts(package_folder: &str, package_names: Vec<&str>) -> BTreeMap<String, Vec<u8>> {
    let mut scripts = BTreeMap::new();
    for package_name in package_names {
        let script = BuiltPackage::build(
            test_dir_path(format!("{}/{}", package_folder, package_name).as_str()),
            aptos_framework::BuildOptions::default(),
        )
        .expect("building packages with scripts must succeed")
        .extract_script_code()[0]
            .clone();
        scripts.insert(package_name.to_string(), script);
    }
    scripts
}
