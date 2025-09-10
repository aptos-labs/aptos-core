// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, bail};
use aptos_framework::{natives::code::PackageMetadata, BuildOptions, BuiltPackage};
use aptos_sdk::bcs;
use move_package::source_package::std_lib::StdVersion;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
};

/// Get the local framework path based on this source file's location.
/// Note: If this source file is moved to a different location, this function
/// may need to be updated.
fn get_local_framework_path() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("aptos-move").join("framework"))
        .expect("framework path")
        .to_string_lossy()
        .to_string()
}

/// Prebuilt package that stores metadata, modules, and scripts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrebuiltPackage {
    pub metadata: PackageMetadata,
    pub modules: BTreeMap<String, Vec<u8>>,
    pub scripts: Vec<Vec<u8>>,
}

/// Bundle of multiple prebuilt packages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrebuiltPackagesBundle {
    pub packages: BTreeMap<String, PrebuiltPackage>,
}

impl PrebuiltPackagesBundle {
    /// Returns the package corresponding to the given name. Panics if such a package does not
    /// exist.
    pub fn get_package(&self, package_name: &str) -> &PrebuiltPackage {
        self.packages
            .get(package_name)
            .unwrap_or_else(|| panic!("Package {package_name} does not exist"))
    }
}

/// Configuration for building a prebuilt package.
#[derive(Debug, Clone)]
pub struct PrebuiltPackageConfig {
    /// If true, packages are compiled with latest (possibly unstable) version.
    pub latest_language: bool,
    /// If true, will use the local Aptos framework.
    pub use_local_std: bool,
}

impl PrebuiltPackageConfig {
    /// Returns built options corresponding to the prebuilt config.
    pub fn build_options(&self) -> BuildOptions {
        let mut build_options = BuildOptions::move_2();
        build_options.dev = true;
        if self.latest_language {
            build_options = build_options.set_latest_language();
        }
        if self.use_local_std {
            build_options.override_std = Some(StdVersion::Local(get_local_framework_path()));
        }
        build_options
    }
}

/// Creates a [PrebuiltPackagesBundle] from the provided list of packages, serializes it and saves
/// as a file in `base_dir/prebuilt.mpb` (`base_dir` should be a Cargo crate). Also generates a
/// Rust file in that crate that allows to access prebuilt information. The output file must live
/// in the same crate.
pub fn create_prebuilt_packages_bundle(
    base_dir: impl AsRef<Path>,
    packages_to_build: Vec<(PathBuf, PrebuiltPackageConfig)>,
    output_rust_file: impl AsRef<Path>,
) -> anyhow::Result<()> {
    // Step 1: save serialized pre-built data.
    let mut packages = BTreeMap::new();

    for (package_path, config) in packages_to_build {
        let package = BuiltPackage::build(package_path, config.build_options())
            .map_err(|err| anyhow!("Failed to build a package: {err:?}"))?;

        let metadata = package.extract_metadata()?;
        let modules = package.module_code_iter().collect();
        let scripts = package.extract_script_code();
        if scripts.len() > 1 {
            bail!("For benchmarks, define 1 script per package to make name resolution easier")
        }

        packages.insert(package.name().to_owned(), PrebuiltPackage {
            metadata,
            modules,
            scripts,
        });
    }

    let bundle = PrebuiltPackagesBundle { packages };
    let bundle_bytes = bcs::to_bytes(&bundle)
        .map_err(|err| anyhow!("Failed to serialize prebuilt packages: {err:?}"))?;
    fs::write(base_dir.as_ref().join("prebuilt.mpb"), bundle_bytes)
        .map_err(|err| anyhow!("Failed to save serialized packages: {err:?}"))?;

    // Step 2: generate implementation to access prebuilt packages.
    let code = r#"
// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// This file was generated. Do not modify!
//
// To update this code, run `testsuite/benchmark-workloads/generate.py`.

use aptos_sdk::bcs;
use aptos_transaction_generator_lib::{
    entry_point_trait::PreBuiltPackages, publishing::prebuild_packages::PrebuiltPackagesBundle,
};
use once_cell::sync::Lazy;

/// Bytes of all pre-build packages.
#[rustfmt::skip]
const PREBUILT_BUNDLE_BYTES: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/prebuilt.mpb"));

/// Pre-built deserialized data: for each package, stores package metadata, compiled modules and
/// scripts.
#[rustfmt::skip]
static PREBUILT_BUNDLE: Lazy<PrebuiltPackagesBundle> = Lazy::new(|| {
    bcs::from_bytes::<PrebuiltPackagesBundle>(PREBUILT_BUNDLE_BYTES)
        .expect("prebuilt.mpb can be deserialized")
});

#[rustfmt::skip]
#[derive(Debug)]
pub struct PreBuiltPackagesImpl;

#[rustfmt::skip]
impl PreBuiltPackages for PreBuiltPackagesImpl {
    fn package_bundle(&self) -> &PrebuiltPackagesBundle {
        &PREBUILT_BUNDLE
    }
}
"#;

    let mut file = fs::File::create(output_rust_file)
        .map_err(|err| anyhow!("Failed to create output file: {err:?}"))?;
    write!(file, "{}", code.trim_start())?;

    Ok(())
}
