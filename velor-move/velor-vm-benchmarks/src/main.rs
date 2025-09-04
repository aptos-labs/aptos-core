// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

mod helper;
use velor_framework::{BuildOptions, BuiltPackage};
use velor_language_e2e_tests::executor::FakeExecutor;
use clap::Parser;
use move_binary_format::CompiledModule;
use std::{fs::read_dir, path::PathBuf};

// CLI options
#[derive(Parser, Debug)]
struct Cli {
    #[clap(default_value = "")]
    pattern: String,
}

fn main() {
    //// Implement CLI
    let args = Cli::parse();
    let pattern = &args.pattern;

    //// Discover all top-level packages in samples directory
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("samples");
    let dirs = read_dir(path.as_path()).unwrap();

    //// Setting up local execution environment once
    // disable parallel execution
    let executor = FakeExecutor::from_head_genesis();
    let mut executor = executor.set_not_parallel();

    //// get all paths for Move projects
    let dir_paths = helper::get_dir_paths(dirs);

    //// Go over all Move projects
    for dir_path in dir_paths {
        // configure and build Move package
        let build_options = BuildOptions {
            with_srcs: true,
            with_abis: true,
            with_source_maps: true,
            with_error_map: true,
            ..BuildOptions::default()
        };
        let package =
            BuiltPackage::build(dir_path, build_options).expect("build package must succeed");

        // iterate over all Move package code
        let codes = package.extract_code();
        for code in codes {
            let compiled_module = CompiledModule::deserialize(&code).unwrap();
            let module_id = compiled_module.self_id();
            let identifier = &module_id.name().to_string();

            //// get module address
            let address = module_id.address();

            //// get all benchmark tagged functions
            let func_identifiers = helper::get_functional_identifiers(
                compiled_module,
                identifier,
                *address,
                pattern.clone(),
            );

            //// publish module and sign user transaction
            //// the benchmark happens when in signing user txn
            helper::publish(
                &package,
                &mut executor,
                func_identifiers,
                *address,
                identifier,
            )
        }
    }
}
