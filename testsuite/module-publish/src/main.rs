// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_transaction_generator_lib::publishing::prebuild_packages::create_prebuilt_packages_rs_file;
use clap::Parser;

#[derive(Parser, Debug)]
#[clap(
    name = "module-publish",
    about = "Write Move packages binaries in a Rust file (raw_module_data.rs). Defaults to \n\
         aptos-core/crates/transaction-workloads-lib/src/raw_module_data.rs"
)]
struct Args {
    #[clap(long, help = "Optional output directory for raw_module_data.rs")]
    out_dir: Option<String>,
}

// List of additional packages (beyond those in testsuite/module-publish/src/packages) to include
fn additional_packages() -> Vec<(&'static str, &'static str, bool)> {
    // Pairs of (package_name, package_path)
    vec![
        ("simple", "src/packages/simple", true),
        (
            "framework_usecases",
            "src/packages/framework_usecases",
            true,
        ),
        (
            "experimental_usecases",
            "src/packages/experimental_usecases",
            true,
        ),
        ("complex", "src/packages/complex", true),
        (
            "ambassador_token",
            "../../aptos-move/move-examples/token_objects/ambassador",
            true,
        ),
        (
            "aggregator_examples",
            "../../aptos-move/move-examples/aggregator_examples",
            true,
        ),
        (
            "bcs_stream",
            "../../aptos-move/move-examples/bcs-stream",
            true,
        ),
    ]
}

// Run "cargo run -p module-publish" to generate the file `raw_module_data.rs`.

// This file updates `raw_module_data.rs` in
// `crates/transaction-emitter-lib/src/transaction_generator/publishing/` by default,
// or in a provided directory.
// That file contains `Lazy` static variables for the binary of all the packages in
// `testsuite/simple/src/packages` as `Lazy`.
// In `crates/transaction-emitter-lib/src/transaction_generator/publishing` you should
// also find the files that can load, manipulate and use the modules.
// Typically those modules will be altered (publishing at different addresses requires a module
// address rewriting, versioning may benefit from real changes), published and used in transaction.
// Code to conveniently do that should be in that crate.
//
// All of that considered, please be careful when changing this file or the modules in
// `testsuite/simple/src/packages` given that it will likely require
// changes in `crates/transaction-emitter-lib/src/transaction_generator/publishing`.
fn main() -> Result<()> {
    let args = Args::parse();

    let packages_to_build = additional_packages();

    // build GenericModule
    let provided_dir = match &args.out_dir {
        None => env!("CARGO_MANIFEST_DIR"),
        Some(str) => str,
    };
    println!("Building GenericModule in {}", provided_dir);
    let base_dir = std::path::Path::new(provided_dir);
    // this is gotta be the most brittle solution ever!
    // If directory structure changes this breaks.
    // However it is a test that is ignored and runs only with the intent of creating files
    // for the modules compiled, so people can change it as they wish and need to.
    let base_path = base_dir.join("../../crates/transaction-workloads-lib/src/");
    let output_file = base_path.join("raw_module_data.rs");

    create_prebuilt_packages_rs_file(base_dir, packages_to_build, output_file, true)
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Args::command().debug_assert()
}
