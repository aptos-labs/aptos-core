// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_transaction_generator_lib::{create_prebuilt_packages_bundle, PrebuiltPackageConfig};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

/// Recursively traverses a directory to extract paths of all Move packages inside it.
fn visit(
    dir: &Path,
    config: &PrebuiltPackageConfig,
    package_paths: &mut Vec<(PathBuf, PrebuiltPackageConfig)>,
) -> std::io::Result<()> {
    // Package found, do not recurse further.
    if dir.join("Move.toml").is_file() {
        package_paths.push((dir.to_path_buf(), config.clone()));
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            visit(&entry.path(), config, package_paths)?;
        }
    }
    Ok(())
}

/// Print rerun-if-changed directives for a directory (recursively).
fn print_rerun_if_changed(dir: &Path) {
    if !dir.exists() {
        return;
    }

    // Track the directory itself
    println!("cargo:rerun-if-changed={}", dir.display());

    // Track all files and subdirectories
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                print_rerun_if_changed(&path);
            } else {
                println!("cargo:rerun-if-changed={}", path.display());
            }
        }
    }
}

fn main() -> anyhow::Result<()> {
    // Set the below variable to skip the building step. This might be useful if the build
    // is broken so it can be debugged with the old outdated artifacts.
    if env::var("SKIP_PREBUILT_PACKAGES_BUILD").is_ok() {
        return Ok(());
    }

    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR defined"));
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR defined"));

    // Get the root of the aptos-core repository
    let root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("Could not find aptos-core root");

    let framework_path = root.join("aptos-move").join("framework");

    // Specifies directories for regular packages.
    // Paths are relative to the root of the aptos-core repository.
    let packages = [
        "testsuite/benchmark-workloads/packages",
        "aptos-move/move-examples/token_objects/ambassador",
        "aptos-move/move-examples/aggregator_examples",
        "aptos-move/move-examples/bcs-stream",
    ];

    // Specifies directories for experimental packages (will be compiled with latest,
    // possibly unstable language version).
    // Paths are relative to the root of the aptos-core repository.
    let experimental_packages =
        ["testsuite/benchmark-workloads/packages-experimental/experimental_usecases"];

    // Print rerun-if-changed for all package directories
    for package_dir in packages.iter().chain(experimental_packages.iter()) {
        print_rerun_if_changed(&root.join(package_dir));
    }

    // Also track the framework directories since packages depend on them
    for framework_subdir in [
        "move-stdlib",
        "aptos-stdlib",
        "aptos-framework",
        "aptos-token",
        "aptos-token-objects",
    ] {
        let sources_dir = framework_path.join(framework_subdir).join("sources");
        let move_toml = framework_path.join(framework_subdir).join("Move.toml");
        println!("cargo:rerun-if-changed={}", sources_dir.display());
        println!("cargo:rerun-if-changed={}", move_toml.display());
    }

    // Collect all package paths
    let mut all_package_paths = vec![];

    // Regular packages
    for package_dir in &packages {
        let config = PrebuiltPackageConfig::new(false, true, vec![]);
        visit(&root.join(package_dir), &config, &mut all_package_paths)?;
    }

    // Experimental packages (use latest language version)
    for package_dir in &experimental_packages {
        let config = PrebuiltPackageConfig::new(true, true, vec![]);
        visit(&root.join(package_dir), &config, &mut all_package_paths)?;
    }

    // Create the prebuilt packages bundle.
    // The rust file is written to OUT_DIR and ignored - we use a manually maintained
    // prebuilt_packages.rs that includes from OUT_DIR instead.
    let dummy_rust_file = out_dir.join("prebuilt_packages_generated.rs");
    create_prebuilt_packages_bundle(&out_dir, all_package_paths, dummy_rust_file)?;

    Ok(())
}
