// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::{anyhow, bail};
use aptos_framework::{natives::code::PackageMetadata, BuildOptions, BuiltPackage};
use aptos_sdk::bcs;
use move_package::source_package::std_lib::StdVersion;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    env, fs,
    io::{self, Write},
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
    /// Experiments for compiler optimization.
    pub experiments: Vec<String>,
}

impl PrebuiltPackageConfig {
    pub fn new(latest_language: bool, use_local_std: bool, experiments: Vec<String>) -> Self {
        Self {
            latest_language,
            use_local_std,
            experiments,
        }
    }

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
        for exp in &self.experiments {
            build_options = build_options.with_experiment(exp);
        }
        build_options
    }
}

/// Recursively traverses a directory to extract paths of all Move packages inside it.
/// A Move package is identified by the presence of a `Move.toml` file.
pub fn visit_packages(
    dir: &Path,
    config: &PrebuiltPackageConfig,
    package_paths: &mut Vec<(PathBuf, PrebuiltPackageConfig)>,
) -> io::Result<()> {
    // Package found, do not recurse further.
    if dir.join("Move.toml").is_file() {
        package_paths.push((dir.to_path_buf(), config.clone()));
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            visit_packages(&entry.path(), config, package_paths)?;
        }
    }
    Ok(())
}

/// Creates a [PrebuiltPackagesBundle] from the provided list of packages, serializes it and saves
/// as a file in `out_dir/{bundle_filename}`. Also generates a Rust file that allows to access
/// prebuilt information via `include!` macro.
///
/// # Arguments
/// * `out_dir` - Directory where the `.mpb` file and Rust file will be saved (typically `OUT_DIR`)
/// * `bundle_filename` - Name of the bundle file (e.g., "head_transaction_generator.mpb")
/// * `packages_to_build` - List of (package_path, config) tuples to build
/// * `output_rust_file` - Path where the generated Rust file will be saved
pub fn create_prebuilt_packages_bundle(
    out_dir: impl AsRef<Path>,
    bundle_filename: &str,
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
    fs::write(out_dir.as_ref().join(bundle_filename), bundle_bytes)
        .map_err(|err| anyhow!("Failed to save serialized packages: {err:?}"))?;

    // Step 2: generate implementation to access prebuilt packages.
    // Uses OUT_DIR for the include path, making it suitable for build.rs usage.
    let code = format!(
        r#"// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// This file was generated by build.rs. Do not modify!

use aptos_sdk::bcs;
use aptos_transaction_generator_lib::{{
    entry_point_trait::PreBuiltPackages, publishing::prebuild_packages::PrebuiltPackagesBundle,
}};
use once_cell::sync::Lazy;

/// Bytes of all pre-built packages (compiled by build.rs).
#[cfg(unix)]
const PREBUILT_BUNDLE_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/{bundle_filename}"));
#[cfg(windows)]
const PREBUILT_BUNDLE_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "\\{bundle_filename}"));

/// Pre-built deserialized data: for each package, stores package metadata, compiled modules and
/// scripts.
static PREBUILT_BUNDLE: Lazy<PrebuiltPackagesBundle> = Lazy::new(|| {{
    bcs::from_bytes::<PrebuiltPackagesBundle>(PREBUILT_BUNDLE_BYTES)
        .expect("{bundle_filename} can be deserialized")
}});

#[derive(Debug)]
pub struct PreBuiltPackagesImpl;

impl PreBuiltPackages for PreBuiltPackagesImpl {{
    fn package_bundle(&self) -> &PrebuiltPackagesBundle {{
        &PREBUILT_BUNDLE
    }}
}}
"#
    );

    let mut file = fs::File::create(output_rust_file)
        .map_err(|err| anyhow!("Failed to create output file: {err:?}"))?;
    write!(file, "{}", code)?;

    Ok(())
}

/// Arguments for building prebuilt packages in a cargo build.rs context.
pub struct PrebuiltPackagesArgs {
    /// Directories where regular packages are stored (relative to manifest_dir - folder in which Cargo.toml is located).
    pub packages: Vec<String>,
    /// Directories where experimental packages are stored (relative to manifest_dir - folder in which Cargo.toml is located).
    /// These will be compiled with latest (possibly unstable) language version.
    pub experimental_packages: Vec<String>,
    /// Name of the bundle file (e.g., "head_transaction_generator.mpb").
    pub bundle_filename: String,
    /// Name of the generated Rust file (e.g., "prebuilt_packages.rs").
    pub rust_filename: String,
    /// If true, uses local aptos-framework from aptos-core.
    pub use_local_std: bool,
    /// Experiments for compiler optimization.
    pub experiments: Vec<String>,
}

/// Build prebuilt packages in a cargo build.rs context.
///
/// This function:
/// - Reads `CARGO_MANIFEST_DIR` and `OUT_DIR` environment variables
/// - Prints `cargo:rerun-if-changed` directives for all package directories
/// - Builds all packages and creates the bundle in `OUT_DIR`
/// - Can be skipped by setting `SKIP_PREBUILT_PACKAGES_BUILD=1`
pub fn cargo_build_prebuilt_packages(args: PrebuiltPackagesArgs) -> anyhow::Result<()> {
    // Set the below variable to skip the building step. This might be useful if the build
    // is broken so it can be debugged with the old outdated artifacts.
    if env::var("SKIP_PREBUILT_PACKAGES_BUILD").is_ok() {
        return Ok(());
    }

    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR defined"));

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR defined"));

    // Print rerun-if-changed for all package directories
    for package_dir in args.packages.iter().chain(args.experimental_packages.iter()) {
        println!("cargo:rerun-if-changed={}", manifest_dir.join(package_dir).display());
    }

    if args.packages.is_empty() && args.experimental_packages.is_empty() {
        bail!("At least one path pointing to packages directory should be provided");
    }

    let mut all_package_paths = vec![];
    for (package_dir, latest_language) in args.packages.into_iter().map(|p| (p, false)).chain(
        args.experimental_packages
            .into_iter()
            .map(|p| (p, true)),
    ) {
        let config =
            PrebuiltPackageConfig::new(latest_language, args.use_local_std, args.experiments.clone());
        visit_packages(&manifest_dir.join(package_dir), &config, &mut all_package_paths)?;
    }

    // Create the prebuilt packages bundle.
    let output_rust_file = out_dir.join(&args.rust_filename);
    create_prebuilt_packages_bundle(&out_dir, &args.bundle_filename, all_package_paths, output_rust_file)?;

    Ok(())
}
