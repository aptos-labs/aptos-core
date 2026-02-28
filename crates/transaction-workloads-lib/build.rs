// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use std::{env, path::PathBuf};

use aptos_transaction_generator_lib::{cargo_build_prebuilt_packages, PrebuiltPackagesArgs};

/// Bundle filename for the prebuilt packages.
const BUNDLE_FILENAME: &str = "head_transaction_generator.mpb";
const PREBUILT_RUST_FILE: &str = "prebuilt_transaction_generator_packages.rs";


/// Print `cargo:rerun-if-changed` directives for framework directories.
/// This tracks changes in the Aptos framework that packages depend on.
pub fn print_framework_rerun_if_changed() {
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR defined"));

    // Get the root of the aptos-core repository
    let root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("Could not find aptos-core root");

    let framework_path = root.join("aptos-move").join("framework");

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
}

fn main() -> anyhow::Result<()> {
    print_framework_rerun_if_changed();

    // Specifies directories for regular packages.
    // Paths are relative to the root of the aptos-core repository.
    let packages = vec![
        "../../testsuite/benchmark-workloads/packages".to_string(),
        "../../aptos-move/move-examples/token_objects/ambassador".to_string(),
        "../../aptos-move/move-examples/aggregator_examples".to_string(),
        "../../aptos-move/move-examples/bcs-stream".to_string(),
    ];

    // Specifies directories for experimental packages (will be compiled with latest,
    // possibly unstable language version).
    // Paths are relative to the root of the aptos-core repository.
    let experimental_packages =
        vec!["../../testsuite/benchmark-workloads/packages-experimental/experimental_usecases".to_string()];

    let args = PrebuiltPackagesArgs {
        packages,
        experimental_packages,
        bundle_filename: BUNDLE_FILENAME.to_string(),
        rust_filename: PREBUILT_RUST_FILE.to_string(),
        use_local_std: true,
        experiments: vec![],
    };
    cargo_build_prebuilt_packages(args)?;

    Ok(())
}
